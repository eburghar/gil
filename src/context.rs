use crate::{
    api::{
        keys::GetKey,
        personal_access_tokens::{PersonalAccessTokenState, PersonalAccessTokens},
        users::keys::ListKeys,
    },
    args::{ColorChoice, IdOrName, KeyIdType, Opts, PipelineLog, SubCommand},
    color::{Style, StyledStr},
    config::{AuthType, Config, OAuth2Token},
    fmt::{Colorizer, Stream},
    git::GitProject,
    types::{PersonalAccessToken, SshKey},
    utils::{format_duration, take_from_vec},
};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{NaiveTime, Utc};
use gitlab::{
    api::{
        projects::{
            self,
            jobs::JobScope,
            pipelines,
            repository::{branches, tags},
        },
        users::{CurrentUser, Users},
        Query,
    },
    types, Gitlab, StatusState,
};
use std::{fmt::Display, str::FromStr, sync::OnceLock};

fn status_style(status: StatusState) -> Option<Style> {
    Some(match status {
        StatusState::Success | StatusState::Running => Style::Good,
        StatusState::Canceled | StatusState::Failed => Style::Error,
        StatusState::WaitingForResource | StatusState::Skipped | StatusState::Pending => {
            Style::Warning
        }
        StatusState::Created
        | StatusState::Manual
        | StatusState::Preparing
        | StatusState::Scheduled => Style::Literal,
    })
}

/// Marker for section start and end
#[derive(Debug, PartialEq, Clone)]
enum SectionType {
    Start,
    End,
}

impl FromStr for SectionType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "section_start" {
            Ok(SectionType::Start)
        } else if s == "section_end" {
            Ok(SectionType::End)
        } else {
            Err(anyhow!("Section delimiter not found"))
        }
    }
}

/// Parsing result of a log section
#[derive(Debug, Clone)]
struct Section {
    type_: SectionType,
    timestamp: i64,
    name: String,
    collapsed: bool,
}

impl FromStr for Section {
    type Err = anyhow::Error;

    /// dumb parser for <https://docs.gitlab.com/ee/ci/jobs/#expand-and-collapse-job-log-sections>
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("section_start:") || s.starts_with("section_end:") {
            // section_type:id:name[flags]
            let info: [&str; 3] = s
                // remove leading \r
                .trim()
                .splitn(3, ':')
                .collect::<Vec<&str>>()
                .try_into()
                .unwrap();
            // string to enum
            let type_ = SectionType::from_str(info[0])?;
            let name = info[2];
            // try to separate name and flags
            let name_flags = name
                .find('[')
                .and_then(|i| name.find(']').map(|j| (&name[..i], &name[i + 1..j])));
            Ok(Self {
                type_,
                timestamp: info[1].parse()?,
                name: name_flags
                    .map(|(n, _)| n.to_owned())
                    .unwrap_or_else(|| name.to_owned()),
                collapsed: name_flags
                    .map(|(_, flags)| flags == "collapsed=true")
                    .unwrap_or(false),
            })
        } else {
            Err(anyhow!("Section delimiter not found"))
        }
    }
}
/// Type of current line in the log printer
enum LogState {
    Text,
    Section(Section),
}

/// Structure to drive the section headers parser
struct LogContext {
    pub state: LogState,
    pub sections: Vec<Section>,
}

impl Default for LogContext {
    fn default() -> Self {
        Self {
            state: LogState::Text,
            sections: Vec::default(),
        }
    }
}

impl LogContext {
    /// Decide if we show the current line int the log printer
    fn show_line(&self, args: &PipelineLog) -> bool {
        // show line if we have no filter
        args.all
			// if we are outside of any sections (the first log lines)
			|| ((args.only_headers || args.headers) && self.sections.is_empty())
			// if we are inside a non collapsed section or a collapsed one which id contains the given string
			|| (!args.only_headers
				&& self
				.sections
				.iter()
				.all(|section| !section.collapsed || section.name.contains(&args.section))
				&& self
				.sections
				.iter()
				.any(|section| section.name.contains(&args.section)))
    }
}

/// Static initializer for CliContext
pub static CONTEXT: OnceLock<CliContext> = OnceLock::new();

/// Structure to pass around functions containing informations
/// about execution context
pub struct CliContext {
    /// command
    pub cmd: SubCommand,
    /// verbose mode
    pub verbose: bool,
    /// don't save oidc login to cache
    pub no_cache: bool,
    /// open links automatically
    pub open: bool,
    /// show urls
    pub url: bool,
    /// color mode
    pub color: ColorChoice,
    /// the gitlab connexion
    pub gitlab: Gitlab,
    /// the configuration file
    pub config: Config,
    /// information about the current git repo
    pub repo: GitProject,
}

impl CliContext {
    pub fn global() -> &'static CliContext {
        CONTEXT.get().expect("Context not initialized")
    }

    /// Inializer from cli arguments
    pub fn from_args(opts: Opts) -> Result<Self> {
        // read yaml config
        let config = Config::from_file(opts.config.as_ref(), opts.verbose)?;

        // get information from git
        let repo = GitProject::from_currentdir()?;

        // get the auth configuration for the remote host
        let host_config = config.hosts.get(&repo.host).ok_or_else(|| {
            anyhow!(
                "Missing authentication configuration (hosts.\"{}\" key) in {:?}",
                &repo.host,
                &config.path
            )
        })?;

        let gitlab = match &host_config.auth {
            AuthType::OAuth2(oauth2) => {
                // try to get the token from cache
                if let Some(token) = OAuth2Token::from_cache(&repo.host) {
                    // check if we can login with that
                    if let Ok(gitlab) = Gitlab::with_oauth2(&repo.host, token) {
                        Ok(gitlab)
                    // otherwise try renew the token
                    } else {
                        println!("Trying to log in through https://{}", &repo.host);
                        let token =
                            OAuth2Token::from_login(&repo.host, &host_config.ca, oauth2, &opts)?;
                        Gitlab::with_oauth2(&repo.host, token)
                    }
                // otherwise try to login following the oauth2 flow
                } else {
                    println!("Trying to log in through https://{}", &repo.host);
                    let token = crate::oidc::login(&repo.host, &host_config.ca, oauth2, &opts)?;
                    Gitlab::with_oauth2(&repo.host, token)
                }
            }

            AuthType::Token(token) => Gitlab::new(&repo.host, token),
        }
        .with_context(|| format!("Can't connect to {}", &repo.host))?;

        #[cfg(feature = "color")]
        let color = opts.color;
        #[cfg(not(feature = "color"))]
        let color = ColorChoice::Never;

        Ok(Self {
            cmd: opts.cmd,
            verbose: opts.verbose,
            no_cache: opts.no_cache,
            open: opts.open,
            url: opts.url,
            color,
            gitlab,
            config,
            repo,
        })
    }

    /// Get a project (which can be the one provided or a default one)
    pub fn get_project<'a, T>(&'a self, default: Option<&'a T>) -> Result<types::Project>
    where
        T: AsRef<str> + Display,
    {
        let id = default
            .map(AsRef::as_ref)
            .or_else(|| self.repo.name.as_deref());
        if let Some(id) = id {
            projects::Project::builder()
                .project(id)
                .build()?
                .query(&self.gitlab)
                .with_context(|| format!("Can't find a project named {}", id))
        } else {
            Err(anyhow!(
                "Can't find a project name. Specify one manually on the command line"
            ))
        }
    }

    /// Get a tag (which can be the one provided or a default one) for the given project
    pub fn get_tag<T>(&self, default: Option<&T>, project: &types::Project) -> Result<types::Tag>
    where
        T: AsRef<str> + Display,
    {
        let tag = default
            .map(AsRef::as_ref)
            .or_else(|| self.repo.tag.as_deref());
        if let Some(tag) = tag {
            tags::Tag::builder()
                .project(project.path_with_namespace.as_str())
                .tag_name(tag)
                .build()?
                .query(&self.gitlab)
                .with_context(|| {
                    format!(
                        "Can't find a tag {} for project {}",
                        tag, &project.path_with_namespace
                    )
                })
        } else {
            Err(anyhow!(
                "Can't find a tag for project {}. Specify one manually on the command line",
                &project.path_with_namespace
            ))
        }
    }

    /// Get a branch (which can be the one provided or a default one) for the given project
    pub fn get_branch<T>(
        &self,
        default: Option<&T>,
        project: &types::Project,
    ) -> Result<types::RepoBranch>
    where
        T: AsRef<str> + Display,
    {
        let branch = default
            .map(AsRef::as_ref)
            .or_else(|| Some(&self.repo.branch));
        if let Some(branch) = branch {
            branches::Branch::builder()
                .project(project.path_with_namespace.as_str())
                .branch(branch)
                .build()?
                .query(&self.gitlab)
                .with_context(|| {
                    format!(
                        "Can't find a branch {} for project {}",
                        branch, &project.name_with_namespace
                    )
                })
        } else {
            Err(anyhow!(
                "Can't find a branch for project {}.",
                &project.path_with_namespace
            ))
        }
    }

    /// Returns the provided tag name (default) or the one extracted from the repo
    pub fn get_tagexp<'a>(&'a self, default: Option<&'a String>) -> Result<&'a String> {
        default.or_else(|| self.repo.tag.as_ref()).ok_or_else(|| {
            anyhow!("Can't find a project tag. Specify one manually on the command line")
        })
    }

    /// Returns the provided branch name (default) or the one extracted from the repo
    pub fn get_branchexp<'a>(&'a self, default: Option<&'a String>) -> Result<&'a String> {
        default.or_else(|| Some(&self.repo.branch)).ok_or_else(|| {
            anyhow!("Can't find a project branch. Specify one manually on the command line")
        })
    }

    /// Get a reference (which can be the one provided or a default one) for the given project
    /// checking that it is tag or a branch name
    pub fn get_ref<T>(&self, ref_: Option<&T>, project: &types::Project) -> Result<String>
    where
        T: AsRef<str> + Display,
    {
        // get a reference (a tag or a branch)
        self.get_tag(ref_, project)
			.map(|tag| tag.name)
				.or_else(|_| {
					self
						// get branch from the context
						.get_branch(None::<&String>, project)
						.map(|branch| branch.name)
				})
				.with_context(|| {
					anyhow!("Failed to find a suitable reference for project {} to build the pipeline upon.", &project.name_with_namespace)
				})
    }

    /// Get a reference but returns an Err if the given reference has diverged
    pub fn check_ref<T>(&self, ref_: Option<&T>, project: &types::Project) -> Result<String>
    where
        T: AsRef<str> + Display,
    {
        let ref2_ = self.get_ref(ref_, project)?;
        // check that ref didn't change
        if let Some(r) = ref_ {
            if r.as_ref() != ref2_.as_str() {
                bail!(
                    "Reference {} not found in Project {}",
                    r,
                    &project.path_with_namespace
                )
            }
        }
        Ok(ref2_)
    }

    /// Returns the provided pipeline id (default) or the last pipeline id for a given project and ref
    /// from gitlab or raises an error
    pub fn get_pipeline(
        &self,
        default: Option<u64>,
        project: &types::Project,
        ref_: &String,
    ) -> Result<types::PipelineBasic> {
        if let Some(id) = default {
            let endpoint = pipelines::Pipeline::builder()
                .project(project.path_with_namespace.as_str())
                .pipeline(id)
                .build()?;
            let pipeline = endpoint.query(&self.gitlab).with_context(|| {
                format!(
                    "Failed to get pipeline {} for project {}",
                    id, &project.path_with_namespace
                )
            })?;
            Ok(pipeline)
        } else {
            let endpoint = pipelines::Pipelines::builder()
                .project(project.path_with_namespace.as_str())
                .ref_(ref_)
                .build()?;
            let pipelines: Vec<_> = endpoint.query(&self.gitlab).with_context(|| {
                format!(
                    "Failed to list pipeline for {} @ {}",
                    &project.path_with_namespace, ref_
                )
            })?;

            pipelines.into_iter().next().ok_or_else(|| {
                anyhow!(
                    "Unable to determine the latest pipeline id for {} @ {}",
                    &project.path_with_namespace,
                    ref_
                )
            })
        }
    }

    /// Returns the job with the provived id (default) or the first job of the last pipeline for the a given
    /// project and tag or raises an error
    pub fn get_job<I>(
        &self,
        default: Option<u64>,
        pipeline_id: Option<u64>,
        project: &types::Project,
        ref_: &String,
        scopes: I,
    ) -> Result<types::Job>
    where
        I: Iterator<Item = JobScope>,
    {
        let pipeline = self.get_pipeline(pipeline_id, project, ref_)?;
        let endpoint = pipelines::PipelineJobs::builder()
            .project(project.path_with_namespace.as_str())
            .pipeline(pipeline.id.value())
            .include_retried(true)
            .scopes(scopes)
            .build()?;
        let jobs: Vec<types::Job> = endpoint.query(&self.gitlab).with_context(|| {
            format!(
                "Failed to list jobs for the pipeline {} {} @ {}",
                pipeline.id, &project.path_with_namespace, ref_
            )
        })?;

        // try to get the index of a suitable job
        let i = if let Some(job_id) = default {
            // from the given id
            jobs.iter()
                .enumerate()
                // if it belongs to the pipeline
                .find(|(_, job)| job.id.value() == job_id)
                .ok_or_else(|| {
                    anyhow!(
                        "The Job {} does not belong to Pipeline {} ({} @ {})",
                        job_id,
                        pipeline.id.value(),
                        &project.name_with_namespace,
                        ref_
                    )
                })
                // and if it is in suitable state
                .and_then(|(i, job)| {
                    has_log(job).then_some(i).ok_or_else(|| {
                        anyhow!(
                            "The Job {} from Pipeline {} ({} @ {}) has no log",
                            job_id,
                            pipeline.id.value(),
                            &project.name_with_namespace,
                            ref_
                        )
                    })
                })?
        } else {
            // or from the pipeline jobs list
            has_log(&pipeline)
                // if we find a job in the same state than the pipeline
                .then(|| {
                    jobs.iter()
                        .enumerate()
                        .find_map(|(i, job)| (job.status == pipeline.status).then_some(i))
                })
                .flatten()
                // or if we find a job in a suitable state
                .or_else(|| {
                    jobs.iter()
                        .rev()
                        .enumerate()
                        .find_map(|(i, job)| has_log(job).then_some(i))
                })
                // otherwise fails
                .ok_or_else(|| {
                    anyhow!(
                        "Unable to determine the latest Job id for {} @ {}",
                        &project.path_with_namespace,
                        ref_
                    )
                })?
        };

        self.print_pipeline(&pipeline, project)?;
        self.print_jobs(&jobs)?;
        Ok(take_from_vec(jobs, i).unwrap())
    }

    /// Get a list of Job(s) for a given project's pipeline id
    pub fn get_jobs(&self, project: &types::Project, pipeline: u64) -> Result<Vec<types::Job>> {
        let endpoint = pipelines::PipelineJobs::builder()
            .project(project.path_with_namespace.as_str())
            .pipeline(pipeline)
            .include_retried(true)
            .build()?;
        let jobs: Vec<_> = endpoint.query(&self.gitlab).with_context(|| {
            format!(
                "Failed to jobs list for the pipeline {} of the project {}",
                pipeline, &project.path_with_namespace
            )
        })?;
        Ok(jobs)
    }

    /// Get current user
    pub fn get_current_user(&self) -> Result<types::UserBasic> {
        let endpoint = CurrentUser::builder().build()?;
        let user = endpoint
            .query(&self.gitlab)
            .with_context(|| "Failed to get current user information")?;
        Ok(user)
    }

    /// Get user with name or current user
    pub fn get_user<T>(&self, username: Option<&T>) -> Result<types::UserBasic>
    where
        T: AsRef<str> + Display,
    {
        if let Some(username) = username {
            let endpoint = Users::builder().username(username.as_ref()).build()?;
            let users: Vec<types::UserBasic> = endpoint
                .query(&self.gitlab)
                .with_context(|| format!("Failed to get user {} information", username))?;
            if users.len() > 1 {
                bail!("More than one user matching {}", username);
            }
            users
                .into_iter()
                .nth(0)
                .ok_or_else(|| anyhow!("Fail to get a user"))
        } else {
            self.get_current_user()
        }
    }

    /// Get a token by its name or id
    pub fn get_token(&self, name: &IdOrName) -> Result<PersonalAccessToken> {
        let user = self.get_current_user()?;

        // search token by id
        let tokens: Vec<PersonalAccessToken> = match name {
            IdOrName::Id(id) => {
                let endpoint = PersonalAccessTokens::builder()
                    .user_id(user.id.value())
                    .state(Some(PersonalAccessTokenState::Active))
                    .build()?;
                let tokens: Vec<PersonalAccessToken> = endpoint.query(&self.gitlab)?;
                tokens.into_iter().filter(|e| e.id == *id).collect()
            }
            IdOrName::Name(name) => {
                // search token by name
                let endpoint = PersonalAccessTokens::builder()
                    .user_id(user.id.value())
                    .state(Some(PersonalAccessTokenState::Active))
                    .search(Some(name))
                    .build()?;
                endpoint.query(&self.gitlab)?
            }
        };

        if tokens.len() > 1 {
            bail!(
                "More than one token matching {}: revoke by id instead of name.",
                name
            );
        }
        tokens
            .into_iter()
            .nth(0)
            .ok_or_else(|| anyhow!("Token {} not found", name))
    }

    /// Get a key by its name of id
    pub fn get_key(&self, id: &KeyIdType) -> Result<SshKey> {
        let user = self.get_current_user()?;
        let endpoint = ListKeys::builder().user(&user.username).build()?;
        let keys: Vec<SshKey> = endpoint.query(&self.gitlab)?;

        let key = match id {
            KeyIdType::Id(id) => keys.into_iter().filter(|k| k.id.value() == *id).nth(0),
            KeyIdType::Name(name) => {
                let mkeys: Vec<SshKey> = keys.into_iter().filter(|k| k.title == *name).collect();
                if mkeys.len() > 1 {
                    bail!(
                        "More than one key matching {}: Delete by id instead of name.",
                        name
                    );
                }
                mkeys.into_iter().nth(0)
            }
            KeyIdType::FingerPrint(fingerprint) => {
                let f = &fingerprint.to_string();
                let endpoint = GetKey::builder().fingerprint(f).build()?;
                endpoint.query(&self.gitlab).ok()
            }
        };
        key.ok_or_else(|| anyhow!("Key {} not found", id))
    }

    /// Print a StyledStr with Colorize
    pub fn print_msg(&self, msg: StyledStr) -> Result<()> {
        Colorizer::new(Stream::Stdout, self.color)
            .with_content(msg)
            .print()
            .with_context(|| "Failed to print")
    }

    /// Print section headers
    fn print_section(&self, title: &str, section: &Section, show_line: bool) -> Result<()> {
        let mut msg = StyledStr::new();

        msg.warning(format!("\n> {} [", title));
        msg.literal(&section.name);
        msg.warning("]");
        msg.none(" ");
        if show_line {
            msg.none("\n");
        }

        self.print_msg(msg)
    }

    /// Print the log coming from Gitlab line by line filtering sections if necessary
    fn print_log_lines(&self, log: &[u8], args: &PipelineLog) -> Result<()> {
        use std::io::{BufRead, BufReader};

        let colored = self.color == ColorChoice::Always
            || self.color == ColorChoice::Auto && atty::is(atty::Stream::Stdout);

        let mut reader = BufReader::new(log).lines();
        let mut state = LogContext::default();
        while let Some(Ok(line)) = reader.next() {
            // evaluate show_line for each line
            let mut show_line = state.show_line(args);
            for (_effect, s) in yew_ansi::get_sgr_segments(&line) {
                match state.state {
                    LogState::Text => {
                        if let Ok(section) = Section::from_str(s) {
                            state.state = LogState::Section(section);
                        } else {
                            // when not in color mode we need to print the segment without style
                            if show_line && !colored {
                                let mut msg = StyledStr::new();
                                msg.none(s);
                                self.print_msg(msg)?;
                            }
                        }
                    }
                    LogState::Section(ref section) => {
                        match section.type_ {
                            // start of new section
                            SectionType::Start => {
                                state.sections.push(section.clone());
                                // reevaluate show_line when changing section
                                show_line = state.show_line(args);
                                if args.all || args.headers || args.only_headers {
                                    self.print_section(s, section, show_line)?;
                                }
                                state.state = LogState::Text;
                                // line has already been printed so force to skip in colored mode
                                if colored {
                                    if show_line {
                                        self.print_msg("\n".into())?;
                                    }
                                    show_line = false;
                                }
                            }
                            // end of a section
                            SectionType::End => {
                                let prev_section = state.sections.pop();
                                if args.all || args.headers || args.only_headers {
                                    if let Some(prev_section) = prev_section {
                                        let f = format_duration(
                                            section.timestamp - prev_section.timestamp,
                                        );
                                        let mut msg = StyledStr::new();
                                        msg.warning(format!("< [{}]\n", f));
                                        self.print_msg(msg)?;
                                    }
                                }
                                // reevaluate show_line when changing section
                                show_line = state.show_line(args);
                                // stay in section state if current line is a start or end
                                state.state = Section::from_str(s)
                                    .ok()
                                    .map(LogState::Section)
                                    .unwrap_or(LogState::Text);
                                // line has already been printed so force to skip in colored mode
                                if colored {
                                    show_line = false;
                                }
                            }
                        }
                    }
                }
            }
            if show_line {
                let mut msg = StyledStr::new();
                if colored {
                    msg.none(line);
                }
                msg.none("\n");
                self.print_msg(msg)?;
            }
        }

        Ok(())
    }

    /// Print job's log header
    pub fn print_log(&self, log: &[u8], job: &types::Job, args: &PipelineLog) -> Result<()> {
        let mut msg = StyledStr::new();
        msg.none("Log for job ");
        msg.literal(job.id.to_string());
        msg.none(" - ");
        msg.stylize(status_style(job.status), format!("{:?}", job.status));
        if self.url {
            msg.hint(format!(" ({})", job.web_url));
        }
        msg.none("\n\n");
        Colorizer::new(Stream::Stdout, self.color)
            .with_content(msg)
            .print()?;

        self.print_log_lines(log, args)
    }

    /// Print pipeline header
    pub fn msg_pipeline(
        &self,
        msg: &mut StyledStr,
        pipeline: &types::PipelineBasic,
        project: &types::Project,
    ) {
        msg.none("Pipeline ");
        msg.literal(pipeline.id.to_string());
        msg.none(format!(
            " ({} @ {} = {})",
            project.name_with_namespace.as_str(),
            &pipeline.ref_.as_ref().unwrap_or(&"??".to_owned()),
            &pipeline.sha.value()[..8]
        ));
        if let Some(created_at) = pipeline.created_at {
            msg.none(" [");
            msg.literal(timeago::Formatter::new().convert_chrono(created_at, Utc::now()));
            msg.none("]");
        }
        msg.none(" - ");
        msg.stylize(
            status_style(pipeline.status),
            format!("{:?}", pipeline.status),
        );
        if self.url {
            msg.hint(format!(" ({})", pipeline.web_url));
        }
        msg.none("\n");
    }

    pub fn print_pipeline(
        &self,
        pipeline: &types::PipelineBasic,
        project: &types::Project,
    ) -> Result<()> {
        let mut msg = StyledStr::new();
        self.msg_pipeline(&mut msg, pipeline, project);
        self.print_msg(msg)
    }

    /// Print pipelines list
    pub fn print_pipelines(
        &self,
        pipelines: &[types::PipelineBasic],
        project: &types::Project,
    ) -> Result<()> {
        let mut msg = StyledStr::new();
        if pipelines.is_empty() {
            msg.none("No pipelines found for ");
            msg.literal(project.name_with_namespace.as_str());
        } else {
            msg.none("Pipelines for ");
            msg.literal(project.name_with_namespace.as_str());
            msg.none("\n");
            for pipeline in pipelines.iter() {
                msg.none("- ");
                self.msg_pipeline(&mut msg, pipeline, project);
            }
        }
        // msg.none("\n");
        self.print_msg(msg)
    }

    /// Print the provided jobs list in reverse order (run order)
    pub fn print_jobs(&self, jobs: &[types::Job]) -> Result<()> {
        let mut msg = StyledStr::new();
        if !jobs.is_empty() {
            for job in jobs.iter().rev() {
                msg.none("- Job ");
                msg.literal(job.id.to_string());
                msg.none(format!(" {} ", job.name));
                msg.hint(format!("({})", job.stage));
                if let Some(duration) =
                    job.finished_at
                        .or_else(|| Some(Utc::now()))
                        .and_then(|end| {
                            job.started_at
                                .map(|start| format_duration((end - start).num_seconds()))
                        })
                {
                    msg.none(" [");
                    msg.literal(duration);
                    msg.none("]");
                }
                msg.none(" - ");
                msg.stylize(status_style(job.status), format!("{:?}", job.status));
                if self.url {
                    msg.hint(format!(" ({}))", job.web_url));
                }
                msg.none("\n");
            }
            msg.none("\n");
        }
        self.print_msg(msg)
    }

    // Print project header
    pub fn print_project(&self, project: &types::Project, ref_: &String) -> Result<()> {
        let mut msg = StyledStr::new();
        msg.none("Project ");
        msg.literal(&project.id.to_string());
        msg.none(" ( ");
        msg.literal(&project.name_with_namespace);
        msg.none(" @ ");
        msg.literal(ref_);
        msg.none(" ) ");
        if self.url {
            msg.hint(format!("({})", &project.web_url));
        }
        msg.none("\n");
        self.print_msg(msg)
    }

    pub fn print_tokens(
        &self,
        tokens: &[crate::types::PersonalAccessToken],
        user: &types::UserBasic,
    ) -> Result<()> {
        let mut msg = StyledStr::new();
        msg.none("Token(s) for user ");
        msg.literal(&user.username);
        msg.hint(format!("({}) :\n", user.id.value()));
        if !tokens.is_empty() {
            for token in tokens.iter().rev() {
                msg.literal(format!("- {}", token.name));
                let token_id = token.id.to_string();
                msg.hint("(");
                if token.active {
                    msg.good(token_id)
                } else {
                    msg.error(token_id)
                }
                msg.hint(")");
                msg.hint(" [");
                for (i, scope) in token.scopes.iter().enumerate() {
                    if i > 0 {
                        msg.hint(",")
                    }
                    msg.hint(scope.as_str());
                }
                msg.hint("] - ");
                if token.active {
                    msg.good("active");
                } else {
                    msg.error("inactive")
                }
                msg.none(" - ");
                if token.revoked {
                    msg.error("revoked");
                } else {
                    msg.good("issued");
                    msg.hint(format!(
                        " ({})",
                        timeago::Formatter::new().convert_chrono(token.created_at, Utc::now(),)
                    ));
                }
                msg.none(", ");
                if token.expired() {
                    msg.error("expired");
                    if let Some(expires_at) = token.expires_at.map(|d| {
                        d.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                            .and_utc()
                    }) {
                        msg.hint(format!(
                            " ({})",
                            timeago::Formatter::new().convert_chrono(expires_at, Utc::now(),)
                        ));
                    }
                } else {
                    msg.good("valid");
                    if let Some(expires_at) = token.expires_at {
                        msg.hint(format!(" (until {})", expires_at));
                    }
                };
                msg.none("\n");
            }
        }
        self.print_msg(msg)
    }

    /// Print ssh keys
    pub fn print_keys(&self, keys: &Vec<SshKey>, user: &types::UserBasic) -> Result<()> {
        let mut msg = StyledStr::new();
        msg.none("Key(s) for user ");
        msg.literal(&user.username);
        msg.hint(format!("({}) :\n", user.id.value()));
        if !keys.is_empty() {
            for key in keys {
                msg.none("- ");
                msg.literal(&key.title);
                msg.none(" (");
                msg.hint(key.id.value().to_string());
                msg.none(")");
                msg.none("\n");
            }
        }
        self.print_msg(msg)
    }

    /// print a username
    pub fn print_username(&self, user: &types::UserBasic) -> Result<()> {
        let mut msg = StyledStr::new();
        msg.none(&user.username);
        self.print_msg(msg)
    }
}

/// Trait for gitlab types having a statusstate field
trait HasStatusState {
    fn get_status(&self) -> StatusState;
}

impl HasStatusState for &types::PipelineBasic {
    fn get_status(&self) -> StatusState {
        self.status
    }
}

impl HasStatusState for &types::Job {
    fn get_status(&self) -> StatusState {
        self.status
    }
}

#[inline]
fn has_log<T>(job: T) -> bool
where
    T: HasStatusState,
{
    let status = job.get_status();
    status == StatusState::Canceled
        || status == StatusState::Failed
        || status == StatusState::Running
        || status == StatusState::Success
}
