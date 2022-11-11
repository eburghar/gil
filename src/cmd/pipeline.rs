use crate::{
	args::{self, PipelineCmd},
	context::CliContext,
};

use anyhow::{Context, Result};
use gitlab::{
	api::{
		self,
		projects::{
			jobs::{self, JobScope},
			pipelines,
		},
		Pagination, Query,
	},
	types,
};

/// Command implementation
pub fn cmd(context: &CliContext, args: &args::Pipeline) -> Result<()> {
	match &args.cmd {
		PipelineCmd::List(cmd_args) => {
			// get project from command line or context
			let project = context.get_project(cmd_args.project.as_ref())?;
			let endpoint = pipelines::Pipelines::builder()
				.project(project.path_with_namespace.to_owned())
				.build()?;
			let pipelines: Vec<types::PipelineBasic> =
				api::paged(endpoint, Pagination::Limit(cmd_args.limit))
					.query(&context.gitlab)
					.with_context(|| {
						format!(
							"Failed to list pipelines for {}",
							&project.name_with_namespace
						)
					})?;

			context.print_pipelines(&pipelines, &project)?;

			if context.open {
				let _ = open::that(format!("{}/-/pipelines", &project.web_url));
			}
			Ok(())
		}

		PipelineCmd::Create(cmd_args) => {
			// get project from command line or context
			let project = context.get_project(cmd_args.project.as_ref())?;
			// get a reference (a tag or a branch)
			let ref_ = context.check_ref(cmd_args.ref_.as_ref(), &project)?;

			let endpoint = pipelines::CreatePipeline::builder()
				.project(project.path_with_namespace.to_owned())
				.ref_(&ref_)
				.build()?;
			let pipeline: types::PipelineBasic =
				endpoint.query(&context.gitlab).with_context(|| {
					format!(
						"Failed to create pipeline for {} @ {}",
						&project.path_with_namespace, &ref_
					)
				})?;

			context.print_pipeline(&pipeline, &project)?;
			let jobs = context.get_jobs(&project, pipeline.id.value())?;
			context.print_jobs(&jobs)?;

			if context.open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(())
		}

		PipelineCmd::Status(cmd_args) => {
			// get project from command line or context
			let project = context.get_project(cmd_args.project.as_ref())?;
			// get a reference (a tag or a branch)
			let ref_ = context.check_ref(cmd_args.ref_.as_ref(), &project)?;
			let pipeline = context.get_pipeline(cmd_args.id, &project, &ref_)?;

			context.print_pipeline(&pipeline, &project)?;
			let jobs = context.get_jobs(&project, pipeline.id.value())?;
			context.print_jobs(&jobs)?;

			if context.open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(())
		}

		PipelineCmd::Cancel(cmd_args) => {
			// get project from command line or context
			let project = context.get_project(cmd_args.project.as_ref())?;
			// get a reference (a tag or a branch)
			let ref_ = context.check_ref(cmd_args.ref_.as_ref(), &project)?;
			let pipeline = context.get_pipeline(cmd_args.id, &project, &ref_)?;

			let endpoint = pipelines::CancelPipeline::builder()
				.project(project.path_with_namespace.to_owned())
				.pipeline(pipeline.id.value())
				.build()?;
			let pipeline: types::PipelineBasic =
				endpoint.query(&context.gitlab).with_context(|| {
					format!("Failed to cancel pipeline {}", &pipeline.id.to_string())
				})?;

			context.print_pipeline(&pipeline, &project)?;
			// list jobs after cancel
			let jobs = context.get_jobs(&project, pipeline.id.value())?;
			context.print_jobs(&jobs)?;

			if context.open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(())
		}

		PipelineCmd::Retry(cmd_args) => {
			// get project from command line or context
			let project = context.get_project(cmd_args.project.as_ref())?;
			// get a reference (a tag or a branch)
			let ref_ = context.check_ref(cmd_args.ref_.as_ref(), &project)?;
			let pipeline = context.get_pipeline(cmd_args.id, &project, &ref_)?;

			let endpoint = pipelines::RetryPipeline::builder()
				.project(project.path_with_namespace.to_owned())
				.pipeline(pipeline.id.value())
				.build()?;
			let pipeline: types::PipelineBasic = endpoint
				.query(&context.gitlab)
				.with_context(|| format!("Failed to retry pipeline {}", pipeline.id))?;

			context.print_pipeline(&pipeline, &project)?;
			// list jobs after retry
			let jobs = context.get_jobs(&project, pipeline.id.value())?;
			context.print_jobs(&jobs)?;

			if context.open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(())
		}

		PipelineCmd::Log(cmd_args) => {
			// get project from command line or context
			let project = context.get_project(cmd_args.project.as_ref())?;
			// get a reference (a tag or a branch)
			let ref_ = context.check_ref(cmd_args.ref_.as_ref(), &project)?;

			let scopes = [
				JobScope::Running,
				JobScope::Failed,
				JobScope::Success,
				JobScope::Canceled,
			];
			let job = context.get_job(
				cmd_args.job_id,
				cmd_args.id,
				&project,
				&ref_,
				scopes.into_iter(),
			)?;
			let endpoint = jobs::JobTrace::builder()
				.project(project.path_with_namespace)
				.job(job.id.value())
				.build()?;

			let log = api::raw(endpoint).query(&context.gitlab)?;
			context.print_log(&log, &job, cmd_args)?;
			if context.open {
				let _ = open::that(job.web_url);
			}
			Ok(())
		}
	}
}
