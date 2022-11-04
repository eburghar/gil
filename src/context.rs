use crate::{
	args::{ColorChoice, Opts, PipelineLog},
	color::{Style, StyledStr},
	config::{AuthType, Config, OAuth2Token},
	fmt::{Colorizer, Stream},
	git::GitProject,
	utils::{format_duration, take_from_vec},
};

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use gitlab::{
	api::{
		projects::{
			self,
			jobs::JobScope,
			pipelines,
			repository::{branches, tags},
		},
		Query,
	},
	types, Gitlab, StatusState,
};
use std::str::FromStr;

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

#[derive(Debug, PartialEq, Clone)]
/// Marker for section start and end
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

#[derive(Debug, Clone)]
/// Parsing result of a log section
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
			bail!("Section delimiter not found")
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
			// if we are outside of any sections
			|| (args.headers && self.sections.is_empty())
			// if we are inside a non collapsed section which id contains the given string
			|| (self
				.sections
				.iter()
				.all(|section| !section.collapsed)
				&& self
				.sections
				.iter()
				.any(|section| section.name.contains(&args.section)))
	}
}

/// Structure to pass around functions containing informations
/// about execution context
pub struct CliContext {
	/// verbose mode
	pub verbose: bool,
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
	pub repo: Option<GitProject>,
}

impl CliContext {
	/// Inializer from cli arguments
	pub fn from_args(opts: &Opts) -> Result<Self> {
		// read yaml config
		let config = Config::from_file(opts.config.as_ref(), opts.verbose)?;

		// get information from git
		let repo = GitProject::from_currentdir();

		// connect to gitlab
		let gitlab = match &config.auth {
			AuthType::OAuth2(oauth2) => {
				// try to get the token from cache
				if let Some(token) = OAuth2Token::from_cache() {
					// check if we can login with that
					if let Ok(gitlab) = Gitlab::with_oauth2(&config.host, token) {
						Ok(gitlab)
					// otherwise try relogin
					} else {
						let token = OAuth2Token::from_login(&config.host, oauth2, opts)?;
						Gitlab::with_oauth2(&config.host, token)
					}
				// otherwise try to login
				} else {
					let token = crate::oidc::login(&config.host, oauth2, opts)?;
					Gitlab::with_oauth2(&config.host, token)
				}
			}

			AuthType::Token(token) => Gitlab::new(&config.host, token),
		}
		.with_context(|| format!("Can't connect to {}", &config.host))?;

		#[cfg(feature = "color")]
		let color = opts.color;
		#[cfg(not(feature = "color"))]
		let color = ColorChoice::Never;

		Ok(Self {
			verbose: opts.verbose,
			open: opts.open,
			url: opts.url,
			color,
			gitlab,
			config,
			repo,
		})
	}

	/// Get a project (which can be the one provided or a default one)
	pub fn get_project<'a>(&'a self, default: Option<&'a String>) -> Result<types::Project> {
		let id = default.or_else(|| self.repo.as_ref().and_then(|repo| repo.name.as_ref()));
		if let Some(id) = id {
			projects::Project::builder()
				.project(id.as_str())
				.build()?
				.query(&self.gitlab)
				.with_context(|| format!("Can't find a project named {}", id))
		} else {
			bail!("Can't find a project name. Specify one manually on the command line")
		}
	}

	/// Get a tag (which can be the one provided or a default one) for the given project
	pub fn get_tag(
		&self,
		default: Option<&String>,
		project: &types::Project,
	) -> Result<types::Tag> {
		let tag = default.or_else(|| self.repo.as_ref().and_then(|repo| repo.tag.as_ref()));
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
			bail!(
				"Can't find a tag for project {}. Specify one manually on the command line",
				&project.path_with_namespace
			)
		}
	}

	/// Get a branch (which can be the one provided or a default one) for the given project
	pub fn get_branch(
		&self,
		default: Option<&String>,
		project: &types::Project,
	) -> Result<types::RepoBranch> {
		let branch = default.or_else(|| self.repo.as_ref().and_then(|repo| repo.branch.as_ref()));
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
			bail!(
				"Can't find a branch for project {}.",
				&project.path_with_namespace
			)
		}
	}

	/// Returns the provided tag name (default) or the one extracted from the repo
	pub fn get_tagexp<'a>(&'a self, default: Option<&'a String>) -> Result<&'a String> {
		default
			.or_else(|| self.repo.as_ref().and_then(|repo| repo.tag.as_ref()))
			.ok_or_else(|| {
				anyhow!("Can't find a project tag. Specify one manually on the command line")
			})
	}

	/// Get a reference (which can be the one provided or a default one) for the given project
	/// checking that it is tag or a branch name
	pub fn get_ref(&self, ref_: Option<&String>, project: &types::Project) -> Result<String> {
		// get a reference (a tag or a branch)
		self.get_tag(ref_, project)
			.map(|tag| tag.name)
				.or_else(|_| {
					self
						// get branch from the context
						.get_branch(None, project)
						.map(|branch| branch.name)
				})
				.with_context(|| {
					anyhow!("Failed to find a suitable reference for project {} to build the pipeline upon.", &project.name_with_namespace)
				})
	}

	/// Get a reference but returns an Err if the given reference has diverged
	pub fn check_ref(&self, ref_: Option<&String>, project: &types::Project) -> Result<String> {
		let ref2_ = self.get_ref(ref_, project)?;
		// check that ref didn't change
		if let Some(r) = ref_ {
			if r != &ref2_ {
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
		project: &types::Project,
		ref_: &String,
		scopes: I,
	) -> Result<types::Job>
	where
		I: Iterator<Item = JobScope>,
	{
		let pipeline = self.get_pipeline(None, project, ref_)?;
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

		self.print_pipeline(&pipeline, project, ref_)?;
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
		// if !show_line {
		// 	msg.warning(" <");
		// 	if section.collapsed
		// 		&& (self.color == ColorChoice::Always
		// 			|| self.color == ColorChoice::Auto && atty::is(atty::Stream::Stdout))
		// 	{
		// 		msg.none("\n");
		// 	}
		// }
		// msg.none("\n");
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
								if args.all || args.headers {
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
								if args.all || args.headers {
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
	pub fn print_pipeline(
		&self,
		pipeline: &types::PipelineBasic,
		project: &types::Project,
		ref_: &String,
	) -> Result<()> {
		let mut msg = StyledStr::new();
		msg.none("Pipeline ");
		msg.literal(pipeline.id.value().to_string());
		msg.none(format!(
			" ({} @ {})",
			project.name_with_namespace.as_str(),
			&ref_
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
				job.finished_at
					.or_else(|| Some(Utc::now()))
					.and_then(|end| {
						job.started_at
							.map(|start| format_duration((end - start).num_seconds()))
					})
					.map(|duration| {
						msg.none(" [");
						msg.literal(duration);
						msg.none("]");
					});
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
