use crate::{args, context::CliContext};

use anyhow::{anyhow, Context, Result};
use gitlab::api::{
	self,
	projects::{
		jobs::{Job, JobTrace},
		pipelines::{
			CancelPipeline, CreatePipeline, Pipeline, PipelineJobs, Pipelines, RetryPipeline,
		},
	},
	Query,
};
use serde::Deserialize;
use std::io::{stdout, Write};

#[derive(Debug, Deserialize)]
struct CreatePipelineRes {
	id: u64,
	web_url: String,
}

#[derive(Debug, Deserialize)]
struct PipelineRes {
	id: u64,
	status: String,
	web_url: String,
}

/// Returns the provided pipeline id (default) or the last pipeline id for a given project and tag
/// from gitlab or raises an error
fn get_pipeline(
	default: Option<u64>,
	context: &CliContext,
	project: &String,
	tag: &String,
) -> Result<u64> {
	if let Some(id) = default {
		Ok(id)
	} else {
		let endpoint = Pipelines::builder()
			.project(project.to_owned())
			.ref_(tag.to_owned())
			.build()?;
		let pipelines: Vec<PipelineRes> = endpoint
			.query(&context.gitlab)
			.with_context(|| format!("Failed to list pipeline for {} @ {}", &project, &tag))?;

		pipelines.get(0).map(|pipeline| pipeline.id).ok_or_else(|| {
			anyhow!(
				"Unable to determine the latest pipeline id for {} @ {}",
				project,
				tag
			)
		})
	}
}

#[derive(Debug, Deserialize)]
struct JobRes {
	id: u64,
	web_url: String,
}

/// Returns the provided job id (default) or the first job of the last pipeline for the a given
/// project and tag or raises an error
fn get_job(
	default: Option<u64>,
	context: &CliContext,
	project: &String,
	tag: &String,
) -> Result<(u64, String)> {
	if let Some(id) = default {
		let endpoint = Job::builder().project(project.to_owned()).job(id).build()?;
		let job: JobRes = endpoint
			.query(&context.gitlab)
			.with_context(|| anyhow!("Unable to get the job #{}", id))?;
		Ok((id, job.web_url))
	} else {
		let pipeline = get_pipeline(None, context, project, tag)?;
		let endpoint = PipelineJobs::builder()
			.project(project.to_owned())
			.pipeline(pipeline)
			.include_retried(true)
			.build()?;
		let jobs: Vec<JobRes> = endpoint.query(&context.gitlab).with_context(|| {
			format!(
				"Failed to list jobs for the pipeline {} of the project {} @ {}",
				pipeline, project, tag
			)
		})?;
		jobs.get(0)
			.map(|job| (job.id, job.web_url.to_string()))
			.ok_or_else(|| {
				anyhow!(
					"Unable to determine the latest job id for {} @ {}",
					project,
					tag
				)
			})
	}
}

/// Command implementation
pub fn cmd(context: &CliContext, args: &args::Pipeline) -> Result<()> {
	match &args.cmd {
		args::PipelineCmd::Create(cmd_args) => {
			// get project from command line or context
			let project = context.project(cmd_args.project.as_ref())?;
			// get tag from command line or context
			let tag = context.tag(cmd_args.tag.as_ref())?;

			let endpoint = CreatePipeline::builder()
				.project(project.to_owned())
				.ref_(tag.to_owned())
				.build()?;
			let pipeline: CreatePipelineRes =
				endpoint.query(&context.gitlab).with_context(|| {
					format!("Failed to create pipeline for {} @ {}", &project, &tag)
				})?;
			println!("#{} ({})", pipeline.id, pipeline.web_url);
			if context.open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(())
		}

		args::PipelineCmd::Get(cmd_args) => {
			// get project from command line or context
			let project = context.project(cmd_args.project.as_ref())?;
			let tag = context.tag(None)?;
			let id = get_pipeline(cmd_args.id, context, project, tag)?;
			// let id = get_pipeline(cmd_args.id)?;

			let endpoint = Pipeline::builder()
				.project(project.to_owned())
				.pipeline(id)
				.build()?;
			let pipeline: PipelineRes = endpoint
				.query(&context.gitlab)
				.with_context(|| format!("Failed get pipeline #{}", &id))?;
			println!("#{}: {} ({})", id, pipeline.status, pipeline.web_url);
			if context.open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(())
		}

		args::PipelineCmd::Cancel(cmd_args) => {
			// get project from command line or context
			let project = context.project(cmd_args.project.as_ref())?;
			let tag = context.tag(None)?;
			let id = get_pipeline(cmd_args.id, context, project, tag)?;
			// let id = context.pipeline(cmd_args.id, project)?;

			let endpoint = CancelPipeline::builder()
				.project(project.to_owned())
				.pipeline(id)
				.build()?;
			let pipeline: PipelineRes = endpoint
				.query(&context.gitlab)
				.with_context(|| format!("Failed to cancel pipeline #{}", &id))?;
			println!("#{}: {} ({})", id, pipeline.status, pipeline.web_url);
			if context.open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(())
		}

		args::PipelineCmd::Retry(cmd_args) => {
			// get project from command line or context
			let project = context.project(cmd_args.project.as_ref())?;
			let tag = context.tag(None)?;
			let id = get_pipeline(cmd_args.id, context, project, tag)?;
			// let id = get_pipeline(cmd_args.id)?;

			let endpoint = RetryPipeline::builder()
				.project(project.to_owned())
				.pipeline(id)
				.build()?;
			let pipeline: PipelineRes = endpoint
				.query(&context.gitlab)
				.with_context(|| format!("Failed to retry pipeline #{}", &id))?;
			println!("#{}: {} ({})", id, pipeline.status, pipeline.web_url);
			if context.open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(())
		}

		args::PipelineCmd::Log(cmd_args) => {
			// get project from command line or context
			let project = context.project(cmd_args.project.as_ref())?;
			let tag = context.tag(None)?;
			let (id, web_url) = get_job(cmd_args.id, context, project, tag)?;
			let endpoint = JobTrace::builder()
				.project(project.to_owned())
				.job(id)
				.build()?;
			let log = api::raw(endpoint).query(&context.gitlab)?;
			println!("Log for job #{} - {} @ {} ({})", id, project, tag, web_url);
			stdout().write_all(&log)?;
			if context.open {
				let _ = open::that(web_url);
			}
			Ok(())
		}
	}
}
