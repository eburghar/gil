use crate::{
	args::{self, PipelineCmd},
	context::CliContext,
	utils::{print_jobs, print_log, print_pipeline},
};

use anyhow::{Context, Result};
use gitlab::{
	api::{
		self,
		projects::{
			jobs::{self, JobScope},
			pipelines,
		},
		Query,
	},
	types,
};

/// Command implementation
pub fn cmd(context: &CliContext, args: &args::Pipeline) -> Result<()> {
	match &args.cmd {
		PipelineCmd::Create(cmd_args) => {
			// get project from command line or context
			let project = context.get_project(cmd_args.project.as_ref())?;
			// get a reference (a tag or a branch)
			let ref_ = context.get_ref(cmd_args.tag.as_ref(), &project)?;

			let endpoint = pipelines::CreatePipeline::builder()
				.project(project.path_with_namespace.to_string())
				.ref_(&ref_)
				.build()?;
			let pipeline: types::PipelineBasic =
				endpoint.query(&context.gitlab).with_context(|| {
					format!(
						"Failed to create pipeline for {} @ {}",
						&project.path_with_namespace, &ref_
					)
				})?;

			print_pipeline(&pipeline, &project, &ref_, context.color)?;
			let jobs = context.get_jobs(&project, pipeline.id.value())?;
			print_jobs(&jobs, context.color)?;

			if context.open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(())
		}

		PipelineCmd::Status(cmd_args) => {
			// get project from command line or context
			let project = context.get_project(cmd_args.project.as_ref())?;
			// get a reference (a tag or a branch)
			let ref_ = context.get_ref(None, &project)?;
			let pipeline = context.get_pipeline(cmd_args.id, &project, &ref_)?;
			let jobs = context.get_jobs(&project, pipeline.id.value())?;

			print_pipeline(&pipeline, &project, &ref_, context.color)?;
			print_jobs(&jobs, context.color)?;

			if context.open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(())
		}

		PipelineCmd::Cancel(cmd_args) => {
			// get project from command line or context
			let project = context.get_project(cmd_args.project.as_ref())?;
			// get a reference (a tag or a branch)
			let ref_ = context.get_ref(None, &project)?;
			let pipeline = context.get_pipeline(cmd_args.id, &project, &ref_)?;

			let endpoint = pipelines::CancelPipeline::builder()
				.project(project.path_with_namespace.to_string())
				.pipeline(pipeline.id.value())
				.build()?;
			let pipeline: types::PipelineBasic =
				endpoint.query(&context.gitlab).with_context(|| {
					format!("Failed to cancel pipeline {}", &pipeline.id.to_string())
				})?;

			print_pipeline(&pipeline, &project, &ref_, context.color)?;
			// list jobs after cancel
			let jobs = context.get_jobs(&project, pipeline.id.value())?;
			print_jobs(&jobs, context.color)?;

			if context.open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(())
		}

		PipelineCmd::Retry(cmd_args) => {
			// get project from command line or context
			let project = context.get_project(cmd_args.project.as_ref())?;
			// get a reference (a tag or a branch)
			let ref_ = context.get_ref(None, &project)?;
			let pipeline = context.get_pipeline(cmd_args.id, &project, &ref_)?;

			let endpoint = pipelines::RetryPipeline::builder()
				.project(project.path_with_namespace.to_string())
				.pipeline(pipeline.id.value())
				.build()?;
			let pipeline: types::PipelineBasic = endpoint
				.query(&context.gitlab)
				.with_context(|| format!("Failed to retry pipeline {}", pipeline.id))?;

			print_pipeline(&pipeline, &project, &ref_, context.color)?;
			// list jobs after retry
			let jobs = context.get_jobs(&project, pipeline.id.value())?;
			print_jobs(&jobs, context.color)?;

			if context.open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(())
		}

		PipelineCmd::Log(cmd_args) => {
			// get project from command line or context
			let project = context.get_project(cmd_args.project.as_ref())?;
			// get a reference (a tag or a branch)
			let ref_ = context.get_ref(None, &project)?;
			let scopes = [
				JobScope::Running,
				JobScope::Failed,
				JobScope::Success,
				JobScope::Canceled,
			];
			let job = context.get_job(cmd_args.id, &project, &ref_, scopes.into_iter())?;
			let endpoint = jobs::JobTrace::builder()
				.project(project.path_with_namespace.to_string())
				.job(job.id.value())
				.build()?;

			let log = api::raw(endpoint).query(&context.gitlab)?;
			print_log(&log, &job, context.color)?;
			if context.open {
				let _ = open::that(job.web_url);
			}
			Ok(())
		}
	}
}
