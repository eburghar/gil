use crate::{
	args::Opts,
	config::{AuthType, Config, OAuth2Token},
	git::GitProject,
	utils::print_jobs,
};

use anyhow::{anyhow, Context, Result};
use gitlab::{
	api::{
		projects::{
			jobs::{self, JobScope},
			pipelines,
			repository::tags,
		},
		Query,
	},
	Gitlab,
};
use serde::Deserialize;

/// Structure to pass around functions containing informations
/// about execution context
pub struct CliContext {
	/// verbose mode
	pub verbose: bool,
	/// open links automatically
	pub open: bool,
	/// the gitlab connexion
	pub gitlab: Gitlab,
	/// the configuration file
	pub config: Config,
	/// information about the current git repo
	pub repo: Option<GitProject>,
}

#[derive(Debug, Deserialize)]
pub struct Pipeline {
	pub id: u64,
	pub status: String,
	pub web_url: String,
}

#[derive(Debug, Deserialize)]
pub struct Job {
	pub id: u64,
	pub name: String,
	pub stage: String,
	pub status: String,
	pub web_url: String,
}

#[derive(Debug, Deserialize)]
pub struct TagCommit {
	pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct Tag {
	pub name: String,
	pub commit: TagCommit,
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
					if let Ok(gitlab) = Gitlab::with_oauth2(&config.host, &token.token) {
						Ok(gitlab)
					// otherwise try relogin
					} else {
						let cache = OAuth2Token::from_login(&config.host, oauth2, opts)?;
						Gitlab::with_oauth2(&config.host, &cache.token)
					}
				// otherwise try to login
				} else {
					let cache = crate::oidc::login(&config.host, oauth2, opts)?;
					Gitlab::with_oauth2(&config.host, &cache.token)
				}
			}

			AuthType::Token(token) => Gitlab::new(&config.host, token),
		}
		.with_context(|| format!("Can't connect to {}", &config.host))?;

		Ok(Self {
			verbose: opts.verbose,
			open: opts.open,
			gitlab,
			config,
			repo,
		})
	}

	/// Returns the provided project name (default) or the one extracted from the repo url
	/// or raises an error
	pub fn project<'a>(&'a self, default: Option<&'a String>) -> Result<&'a String> {
		default
			.or_else(|| self.repo.as_ref().and_then(|repo| repo.name.as_ref()))
			.ok_or_else(|| {
				anyhow!("Can't find a project name. Specify one manually on the command line")
			})
	}

	/// Returns the provided tag name (default) or the one extracted from the repo
	/// or raises an error
	pub fn tag<'a>(&'a self, default: Option<&'a String>) -> Result<&'a String> {
		default
			.or_else(|| self.repo.as_ref().and_then(|repo| repo.tag.as_ref()))
			.ok_or_else(|| {
				anyhow!("Can'f find a project tag. Specify one manually on the command line")
			})
	}

	/// Returns the provided pipeline id (default) or the last pipeline id for a given project and tag
	/// from gitlab or raises an error
	pub fn get_pipeline(
		&self,
		default: Option<u64>,
		project: &String,
		tag: &String,
	) -> Result<Pipeline> {
		if let Some(id) = default {
			let endpoint = pipelines::Pipeline::builder()
				.project(project.to_owned())
				.pipeline(id)
				.build()?;
			let pipeline = endpoint.query(&self.gitlab).with_context(|| {
				format!("Failed to get pipeline #{} for project {}", id, project)
			})?;
			Ok(pipeline)
		} else {
			let endpoint = pipelines::Pipelines::builder()
				.project(project.to_owned())
				.ref_(tag.to_owned())
				.build()?;
			let pipelines: Vec<_> = endpoint
				.query(&self.gitlab)
				.with_context(|| format!("Failed to list pipeline for {} @ {}", project, tag))?;

			pipelines.into_iter().next().ok_or_else(|| {
				anyhow!(
					"Unable to determine the latest pipeline id for {} @ {}",
					project,
					tag
				)
			})
		}
	}

	/// Returns the job with the provived id (default) or the first job of the last pipeline for the a given
	/// project and tag or raises an error
	pub fn get_job<I>(
		&self,
		default: Option<u64>,
		project: &String,
		tag: &String,
		scopes: I,
	) -> Result<Job>
	where
		I: Iterator<Item = JobScope>,
	{
		if let Some(id) = default {
			let endpoint = jobs::Job::builder()
				.project(project.to_owned())
				.job(id)
				.build()?;
			let job: Job = endpoint
				.query(&self.gitlab)
				.with_context(|| anyhow!("Unable to get the job #{}", id))?;
			Ok(job)
		} else {
			let pipeline = self.get_pipeline(None, project, tag)?;
			let endpoint = pipelines::PipelineJobs::builder()
				.project(project.to_owned())
				.pipeline(pipeline.id)
				.include_retried(true)
				.scopes(scopes)
				.build()?;
			let jobs: Vec<Job> = endpoint.query(&self.gitlab).with_context(|| {
				format!(
					"Failed to list jobs for the pipeline {} of the project {} @ {}",
					pipeline.id, project, tag
				)
			})?;
			if jobs.len() > 1 {
				let job = jobs.last().unwrap();
				print_jobs(
					format!(
						"Multiple jobs are available. The oldest one (#{}) has been picked. \
				 	Specify the id as argument to change :\n",
						job.id
					),
					&jobs,
				);
			}
			jobs.into_iter().last().ok_or_else(|| {
				anyhow!(
					"Unable to determine the latest job id for {} @ {}",
					project,
					tag
				)
			})
		}
	}

	pub fn get_jobs(&self, project: &String, pipeline: u64) -> Result<Vec<Job>> {
		let endpoint = pipelines::PipelineJobs::builder()
			.project(project.to_owned())
			.pipeline(pipeline)
			.include_retried(true)
			.build()?;
		let jobs: Vec<Job> = endpoint.query(&self.gitlab).with_context(|| {
			format!(
				"Failed to jobs list for the pipeline #{} of the project {}",
				pipeline, project
			)
		})?;
		Ok(jobs)
	}

	pub fn get_tag_commit(&self, project: &str, tag: &str) -> Result<Tag> {
		// get commit sha associated with tag
		let endpoint = tags::Tag::builder()
			.project(project.to_owned())
			.tag_name(tag.to_owned())
			.build()?;
		let res: Tag = endpoint.query(&self.gitlab).with_context(|| {
			format!(
				"Failed to get commit info for tag {} on project {}",
				&tag, &project
			)
		})?;
		Ok(res)
	}
}
