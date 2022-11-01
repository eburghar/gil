use crate::{
	args::{ColorChoice, Opts},
	config::{AuthType, Config, OAuth2Token},
	git::GitProject,
	utils::{print_jobs, print_pipeline},
};

use anyhow::{anyhow, bail, Context, Result};
use gitlab::{
	api::{
		projects::{
			self,
			jobs::{self, JobScope},
			pipelines,
			repository::{branches, commits, tags},
		},
		Query,
	},
	types, Gitlab, StatusState,
};

/// Structure to pass around functions containing informations
/// about execution context
pub struct CliContext {
	/// verbose mode
	pub verbose: bool,
	/// open links automatically
	pub open: bool,
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
			color,
			gitlab,
			config,
			repo,
		})
	}

	/// Returns the provided project name (default) or the one extracted from the repo url
	/// or raises an error
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

	#[allow(dead_code)]
	/// Returns the provided commit id (default) or the one extracted from the repo
	/// or raises an error
	pub fn get_commit(
		&self,
		default: Option<&String>,
		project: &types::Project,
	) -> Result<types::Commit> {
		let commit = default.or_else(|| self.repo.as_ref().and_then(|repo| repo.commit.as_ref()));
		if let Some(commit) = commit {
			commits::Commit::builder()
				.project(project.path_with_namespace.as_str())
				.commit(commit)
				.build()?
				.query(&self.gitlab)
				.with_context(|| {
					format!(
						"Can't find a commit {} for project {}",
						commit, &project.path_with_namespace
					)
				})
		} else {
			bail!(
				"Can't find a commit for project {}",
				&project.name_with_namespace
			)
		}
	}

	/// Returns the provided tag name (default) or the one extracted from the repo
	/// or raises an error
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

	/// Returns the provided branch name (default) or the one extracted from the repo
	/// or raises an error
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
	/// or raises an error
	pub fn get_tagexp<'a>(&'a self, default: Option<&'a String>) -> Result<&'a String> {
		default
			.or_else(|| self.repo.as_ref().and_then(|repo| repo.tag.as_ref()))
			.ok_or_else(|| {
				anyhow!("Can't find a project tag. Specify one manually on the command line")
			})
	}

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
		if let Some(id) = default {
			let endpoint = jobs::Job::builder()
				.project(project.path_with_namespace.as_str())
				.job(id)
				.build()?;
			let job: types::Job = endpoint
				.query(&self.gitlab)
				.with_context(|| format!("Unable to get the job {}", id))?;
			Ok(job)
		} else {
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
			print_pipeline(&pipeline, project, ref_, self.color)?;
			if jobs.len() > 1 {
				print_jobs(&jobs, self.color)?;

				let job = match pipeline.status {
					// return the first job in the same state than the pipeline
					StatusState::Failed | StatusState::Running | StatusState::Success => jobs
						.into_iter()
						.find(|job| job.status == pipeline.status)
						.unwrap(),
					// otherwise return first pipeline job
					_ => jobs.into_iter().last().unwrap(),
				};

				Ok(job)
			} else {
				jobs.into_iter().last().ok_or_else(|| {
					anyhow!(
						"Unable to determine the latest job id for {} @ {}",
						&project.path_with_namespace,
						ref_
					)
				})
			}
		}
	}

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

	pub fn get_tag_commit(&self, project: &String, tag: &str) -> Result<types::Tag> {
		// get commit sha associated with tag
		let endpoint = tags::Tag::builder()
			.project(project.as_str())
			.tag_name(tag)
			.build()?;
		let res: types::Tag = endpoint.query(&self.gitlab).with_context(|| {
			format!(
				"Failed to get commit info for tag {} on project {}",
				tag, project
			)
		})?;
		Ok(res)
	}
}
