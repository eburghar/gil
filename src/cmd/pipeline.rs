use crate::{
	args,
	utils::{get_pipeline, get_project, get_tag},
};
use anyhow::{Context, Result};
use gitlab::{
	api::{
		projects::pipelines::{CancelPipeline, CreatePipeline, Pipeline, RetryPipeline},
		Query,
	},
	Gitlab,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CreatePipelineRes {
	id: u64,
	web_url: String,
}

#[derive(Debug, Deserialize)]
struct PipelineRes {
	status: String,
	web_url: String,
}

pub fn cmd(
	gitlab: Gitlab,
	args::Opts {
		verbose: _,
		config: _,
		..
	}: &args::Opts,
	args: &args::Pipeline,
) -> Result<()> {
	let pipeline_cmd = &args.cmd;
	match pipeline_cmd {
		args::PipelineCmd::Create(cmd_args) => {
			let project = get_project(&cmd_args.project)?;
			let tag = get_tag(&cmd_args.tag)?;
			let endpoint = CreatePipeline::builder()
				.project(project.to_owned())
				.ref_(tag.to_owned())
				.build()?;
			let pipeline: CreatePipelineRes = endpoint.query(&gitlab).context(format!(
				"Failed to create pipeline for {}/{}",
				&project, &tag
			))?;
			log::info!("[{}]({})", pipeline.id, pipeline.web_url);
			Ok(())
		}

		args::PipelineCmd::Get(cmd_args) => {
			let project = get_project(&cmd_args.project)?;
			let id = get_pipeline(cmd_args.id)?;
			let endpoint = Pipeline::builder()
				.project(project.to_owned())
				.pipeline(id)
				.build()?;
			let pipeline: PipelineRes = endpoint
				.query(&gitlab)
				.context(format!("Failed get pipeline #{}", &id))?;
			log::info!("[{}]({}): {}", id, pipeline.web_url, pipeline.status);
			Ok(())
		}

		args::PipelineCmd::Cancel(cmd_args) => {
			let project = get_project(&cmd_args.project)?;
			let id = get_pipeline(cmd_args.id)?;
			let endpoint = CancelPipeline::builder()
				.project(project.to_owned())
				.pipeline(id)
				.build()?;
			let pipeline: PipelineRes = endpoint
				.query(&gitlab)
				.context(format!("Failed cancel pipeline #{}", &id))?;
			log::info!("[{}]({}): {}", id, pipeline.web_url, pipeline.status);
			Ok(())
		}

		args::PipelineCmd::Retry(cmd_args) => {
			let project = get_project(&cmd_args.project)?;
			let id = get_pipeline(cmd_args.id)?;
			let endpoint = RetryPipeline::builder()
				.project(project.to_owned())
				.pipeline(id)
				.build()?;
			let pipeline: PipelineRes = endpoint
				.query(&gitlab)
				.context(format!("Failed cancel pipeline #{}", &id))?;
			log::info!("[{}]({}): {}", id, pipeline.web_url, pipeline.status);
			Ok(())
		}
	}
}
