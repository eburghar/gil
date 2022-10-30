use crate::{
	args::{self, PipelineCmd},
	color::StyledStr,
	context::CliContext,
	fmt::{Colorizer, Stream},
	utils::{print_jobs, status_style},
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
use std::io::{stdout, Write};

/// Command implementation
pub fn cmd(context: &CliContext, args: &args::Pipeline) -> Result<()> {
	match &args.cmd {
		PipelineCmd::Create(cmd_args) => {
			// get project from command line or context
			let project = context.get_project(cmd_args.project.as_ref())?;
			// get tag from command line or context
			let tag = context.get_tag(cmd_args.tag.as_ref())?;

			let endpoint = pipelines::CreatePipeline::builder()
				.project(project.to_owned())
				.ref_(tag.to_owned())
				.build()?;
			let pipeline: types::Pipeline = endpoint.query(&context.gitlab).with_context(|| {
				format!("Failed to create pipeline for {} @ {}", &project, &tag)
			})?;

			let jobs = context.get_jobs(project, pipeline.id.value())?;
			let mut msg = StyledStr::new();
			msg.none("Pipeline");
			msg.literal(format!(" {}", pipeline.id.value()));
			msg.none(":");
			msg.stylize(
				status_style(pipeline.status),
				format!(" {:?}", pipeline.status),
			);
			msg.hint(format!(" ({})", pipeline.web_url));
			print_jobs(msg, context.color, &jobs)?;

			if context.open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(())
		}

		PipelineCmd::Status(cmd_args) => {
			// get project from command line or context
			let project = context.get_project(cmd_args.project.as_ref())?;
			let tag = context.get_tag(None)?;
			let pipeline = context.get_pipeline(cmd_args.id, project, tag)?;
			let jobs = context.get_jobs(project, pipeline.id.value())?;

			let mut msg = StyledStr::new();
			msg.none("Pipeline");
			msg.literal(format!(" {}", pipeline.id.value()));
			msg.none(": ");
			msg.stylize(
				status_style(pipeline.status),
				format!("{:?}", pipeline.status),
			);
			msg.hint(format!(" ({})", pipeline.web_url));
			print_jobs(msg, context.color, &jobs)?;

			if context.open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(())
		}

		PipelineCmd::Cancel(cmd_args) => {
			// get project from command line or context
			let project = context.get_project(cmd_args.project.as_ref())?;
			let tag = context.get_tag(None)?;
			let pipeline = context.get_pipeline(cmd_args.id, project, tag)?;

			let endpoint = pipelines::CancelPipeline::builder()
				.project(project.to_owned())
				.pipeline(pipeline.id.value())
				.build()?;
			let pipeline: types::Pipeline = endpoint
				.query(&context.gitlab)
				.with_context(|| format!("Failed to cancel pipeline #{}", &pipeline.id))?;

			// list jobs after cancel
			let jobs = context.get_jobs(project, pipeline.id.value())?;
			let mut msg = StyledStr::new();
			msg.none("Pipeline");
			msg.literal(format!(" {}", pipeline.id.value()));
			msg.none(": ");
			msg.stylize(
				status_style(pipeline.status),
				format!("{:?}", pipeline.status),
			);
			msg.hint(format!(" ({})", pipeline.web_url));
			print_jobs(msg, context.color, &jobs)?;

			if context.open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(())
		}

		PipelineCmd::Retry(cmd_args) => {
			// get project from command line or context
			let project = context.get_project(cmd_args.project.as_ref())?;
			let tag = context.get_tag(None)?;
			let pipeline = context.get_pipeline(cmd_args.id, project, tag)?;

			let endpoint = pipelines::RetryPipeline::builder()
				.project(project.to_owned())
				.pipeline(pipeline.id.value())
				.build()?;
			let pipeline: types::Pipeline = endpoint
				.query(&context.gitlab)
				.with_context(|| format!("Failed to retry pipeline #{}", &pipeline.id))?;

			// list jobs after retry
			let jobs = context.get_jobs(project, pipeline.id.value())?;
			let mut msg = StyledStr::new();
			msg.none("Pipeline");
			msg.literal(format!(" {}", pipeline.id.value()));
			msg.none(": ");
			msg.stylize(
				status_style(pipeline.status),
				format!("{:?}", pipeline.status),
			);
			msg.hint(format!(" ({})", pipeline.web_url));
			print_jobs(msg, context.color, &jobs)?;

			if context.open {
				let _ = open::that(pipeline.web_url);
			}
			Ok(())
		}

		PipelineCmd::Log(cmd_args) => {
			// get project from command line or context
			let project = context.get_project(cmd_args.project.as_ref())?;
			let tag = context.get_tag(None)?;
			let scopes = [
				JobScope::Running,
				JobScope::Failed,
				JobScope::Success,
				JobScope::Canceled,
			];
			let job = context.get_job(cmd_args.id, project, tag, scopes.into_iter())?;
			let endpoint = jobs::JobTrace::builder()
				.project(project.to_owned())
				.job(job.id.value())
				.build()?;

			let mut msg = StyledStr::new();
			msg.none(format!("Log for job {}: ", job.id));
			msg.stylize(status_style(job.status), format!("{:?}", job.status));
			msg.none(format!(" - {} @ {} ", project, tag));
			msg.hint(format!("({})", job.web_url));
			msg.none("\n\n");
			Colorizer::new(Stream::Stdout, context.color)
				.with_content(msg)
				.print()?;
			let log = api::raw(endpoint).query(&context.gitlab)?;
			stdout().write_all(&log)?;

			if context.open {
				let _ = open::that(job.web_url);
			}
			Ok(())
		}
	}
}