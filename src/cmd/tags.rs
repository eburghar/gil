use crate::{
	args,
	utils::{get_project, get_tagexpr},
};
use anyhow::{Context, Result};
use gitlab::{
	api::{
		self,
		projects::protected_tags::{ProtectTag, UnprotectTag},
		Query,
	},
	Gitlab,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Tag {
	name: String,
}

pub fn cmd(
	gitlab: Gitlab,
	args::Opts {
		verbose: _,
		config: _,
		..
	}: &args::Opts,
	args: &args::Tags,
) -> Result<()> {
	match &args.cmd {
		args::TagsCmd::Unprotect(args) => {
			let project = get_project(&args.project)?;
			let tag = get_tagexpr(&args.tag)?;
			let endpoint = UnprotectTag::builder()
				.project(project.to_owned())
				.name(tag.to_owned())
				.build()?;
			api::ignore(endpoint).query(&gitlab)?;

			Ok(())
		}

		args::TagsCmd::Protect(args) => {
			let project = get_project(&args.project)?;
			let tag = get_tagexpr(&args.tag)?;
			let endpoint = ProtectTag::builder()
				.project(project.to_owned())
				.name(tag.to_owned())
				.build()?;
			let tag: Tag = endpoint.query(&gitlab).context(format!(
				"Failed to protect tag '{}' on project {}",
				&tag, &project
			))?;
			log::info!("tag '{}' is protected on project {}", tag.name, &project);

			Ok(())
		}
	}
}
