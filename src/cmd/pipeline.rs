use std::process::ExitCode;

use crate::{
	args::{self, PipelineCmd},
	context::CliContext,
	types,
};

use anyhow::{Context, Result};
use gitlab::api::{
	self,
	projects::{
		jobs::{self, JobScope},
		pipelines,
	},
	Pagination, Query,
};

/// Command implementation
pub fn cmd(args: &args::Pipeline) -> Result<ExitCode> {
	match &args.cmd {
		PipelineCmd::List(cmd_args) => {
			// get project from command line or context
			let project = CliContext::global().get_project(cmd_args.project.as_ref())?;
			let endpoint = pipelines::Pipelines::builder()
				.project(project.path_with_namespace.to_owned())
				.build()?;
			let pipelines: Vec<types::Pipeline> =
				api::paged(endpoint, Pagination::Limit(cmd_args.limit))
					.query(&CliContext::global().gitlab)
					.with_context(|| {
						format!(
							"Failed to list pipelines for {}",
							&project.name_with_namespace
						)
					})?;

			CliContext::global().print_pipelines(&pipelines, &project)?;

			if CliContext::global().open {
				let _ = open::that(format!("{}/-/pipelines", &project.web_url));
			}
			Ok(ExitCode::from(0))
		}

		PipelineCmd::Create(cmd_args) => {
			// get project from command line or context
			let project = CliContext::global().get_project(cmd_args.project.as_ref())?;
			// get a reference (a tag or a branch)
			let ref_ = CliContext::global().check_ref(cmd_args.ref_.as_deref(), &project)?;

			let endpoint = pipelines::CreatePipeline::builder()
				.project(project.path_with_namespace.to_owned())
				.ref_(&ref_)
				.build()?;
			let pipeline: types::Pipeline = endpoint
				.query(&CliContext::global().gitlab)
				.with_context(|| {
					format!(
						"Failed to create pipeline for {} @ {}",
						&project.path_with_namespace, &ref_
					)
				})?;

			CliContext::global().print_pipeline(&pipeline, &project)?;
			let jobs = CliContext::global().get_jobs(&project, pipeline.id.value())?;
			CliContext::global().print_jobs(&jobs)?;

			if CliContext::global().open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(ExitCode::from(0))
		}

		PipelineCmd::Status(cmd_args) => {
			// get project from command line or context
			let project = CliContext::global().get_project(cmd_args.project.as_ref())?;
			// get a reference (a tag or a branch)
			let ref_ = CliContext::global().check_ref(cmd_args.ref_.as_deref(), &project)?;
			let pipeline = CliContext::global().get_pipeline(cmd_args.id, &project, &ref_)?;

			CliContext::global().print_pipeline(&pipeline, &project)?;
			let jobs = CliContext::global().get_jobs(&project, pipeline.id.value())?;
			CliContext::global().print_jobs(&jobs)?;

			if CliContext::global().open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(ExitCode::from(0))
		}

		PipelineCmd::Cancel(cmd_args) => {
			// get project from command line or context
			let project = CliContext::global().get_project(cmd_args.project.as_ref())?;
			// get a reference (a tag or a branch)
			let ref_ = CliContext::global().check_ref(cmd_args.ref_.as_deref(), &project)?;
			let pipeline = CliContext::global().get_pipeline(cmd_args.id, &project, &ref_)?;

			let endpoint = pipelines::CancelPipeline::builder()
				.project(project.path_with_namespace.to_owned())
				.pipeline(pipeline.id.value())
				.build()?;
			let pipeline: types::Pipeline = endpoint
				.query(&CliContext::global().gitlab)
				.with_context(|| {
					format!("Failed to cancel pipeline {}", &pipeline.id.to_string())
				})?;

			CliContext::global().print_pipeline(&pipeline, &project)?;
			// list jobs after cancel
			let jobs = CliContext::global().get_jobs(&project, pipeline.id.value())?;
			CliContext::global().print_jobs(&jobs)?;

			if CliContext::global().open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(ExitCode::from(0))
		}

		PipelineCmd::Retry(cmd_args) => {
			// get project from command line or context
			let project = CliContext::global().get_project(cmd_args.project.as_ref())?;
			// get a reference (a tag or a branch)
			let ref_ = CliContext::global().check_ref(cmd_args.ref_.as_deref(), &project)?;
			let pipeline = CliContext::global().get_pipeline(cmd_args.id, &project, &ref_)?;

			let endpoint = pipelines::RetryPipeline::builder()
				.project(project.path_with_namespace.to_owned())
				.pipeline(pipeline.id.value())
				.build()?;
			let pipeline: types::Pipeline = endpoint
				.query(&CliContext::global().gitlab)
				.with_context(|| format!("Failed to retry pipeline {}", pipeline.id))?;

			CliContext::global().print_pipeline(&pipeline, &project)?;
			// list jobs after retry
			let jobs = CliContext::global().get_jobs(&project, pipeline.id.value())?;
			CliContext::global().print_jobs(&jobs)?;

			if CliContext::global().open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(ExitCode::from(0))
		}

		PipelineCmd::Log(cmd_args) => {
			// get project from command line or context
			let project = CliContext::global().get_project(cmd_args.project.as_ref())?;
			// get a reference (a tag or a branch)
			let ref_ = CliContext::global().check_ref(cmd_args.ref_.as_deref(), &project)?;

			let scopes = [
				JobScope::Running,
				JobScope::Failed,
				JobScope::Success,
				JobScope::Canceled,
			];
			let job = CliContext::global().get_job(
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

			let log = api::raw(endpoint).query(&CliContext::global().gitlab)?;
			CliContext::global().print_log(&log, &job, cmd_args)?;
			if CliContext::global().open {
				let _ = open::that(job.web_url);
			}
			Ok(ExitCode::from(0))
		}
	}
}
