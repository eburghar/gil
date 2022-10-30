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
			let tag = context.get_tag(Some(&args.tag), &project)?;

			let endpoint = UnprotectTag::builder()
				.project(project.path_with_namespace.to_string())
				.name(tag.name.to_owned())
				.build()?;
			api::ignore(endpoint).query(&context.gitlab)?;
			println!(
				"tag '{}' protection has been removed on project {}",
				&tag.name, &project.path_with_namespace
			);

			Ok(())
		}

		TagsCmd::Protect(args) => {
			let project = context.get_project(args.project.as_ref())?;
			let tag = context.get_tag(Some(&args.tag), &project)?;

			let endpoint = ProtectTag::builder()
				.project(project.path_with_namespace.to_string())
				.name(tag.name.to_owned())
				.build()?;
			let tag: Tag = endpoint.query(&context.gitlab).with_context(|| {
				format!(
					"Failed to protect tag '{}' on project {}",
					&tag.name, &project.path_with_namespace
				)
			})?;
			println!(
				"tag '{}' is protected on project {}",
				tag.name, &project.path_with_namespace
			);

			Ok(())
		}
	}
}
