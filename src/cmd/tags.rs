use crate::{
	args::{self, TagsCmd},
	context::CliContext,
};

use anyhow::{Context, Result};
use gitlab::api::{
	self,
	projects::protected_tags::{ProtectTag, UnprotectTag},
	Query,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Tag {
	name: String,
}

pub fn cmd(context: &CliContext, args: &args::Tags) -> Result<()> {
	match &args.cmd {
		TagsCmd::Unprotect(args) => {
			let project = context.get_project(args.project.as_ref())?;
			let tag = context.get_tag(Some(&args.tag))?;

			let endpoint = UnprotectTag::builder()
				.project(project.to_owned())
				.name(tag.to_owned())
				.build()?;
			api::ignore(endpoint).query(&context.gitlab)?;
			println!(
				"tag '{}' protection has been removed on project {}",
				tag, &project
			);

			Ok(())
		}

		TagsCmd::Protect(args) => {
			let project = context.get_project(args.project.as_ref())?;
			let tag = context.get_tag(Some(&args.tag))?;

			let endpoint = ProtectTag::builder()
				.project(project.to_owned())
				.name(tag.to_owned())
				.build()?;
			let tag: Tag = endpoint.query(&context.gitlab).with_context(|| {
				format!("Failed to protect tag '{}' on project {}", &tag, &project)
			})?;
			println!("tag '{}' is protected on project {}", tag.name, &project);

			Ok(())
		}
	}
}