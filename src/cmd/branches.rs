use crate::{
	args::{self, BranchesCmd},
	context::CliContext,
};

use anyhow::{Context, Result};
use gitlab::{
	api::{
		self,
		projects::protected_branches::{ProtectBranch, ProtectedBranches, UnprotectBranch},
		Query,
	},
	types,
};
use serde::Deserialize;
use std::process::ExitCode;

#[derive(Deserialize)]
struct Tag {
	name: String,
}

pub fn cmd(args: &args::Branches) -> Result<ExitCode> {
	match &args.cmd {
		BranchesCmd::Unprotect(args) => {
			let project = CliContext::global().get_project(args.project.as_ref())?;
			let branch = CliContext::global().get_branchexp(args.branch.as_ref())?;

			let endpoint = ProtectedBranches::builder()
				.project(project.path_with_namespace.to_owned())
				.build()?;
			let tags: Vec<types::ProtectedRepoBranch> =
				endpoint.query(&CliContext::global().gitlab)?;

			if !tags.iter().any(|b| &b.name == branch) {
				println!(
					"branch '{}' protection not found on project {}",
					&branch, &project.path_with_namespace
				);
			} else {
				let endpoint = UnprotectBranch::builder()
					.project(project.path_with_namespace.to_owned())
					.name(branch)
					.build()?;
				api::ignore(endpoint).query(&CliContext::global().gitlab)?;
				println!(
					"branch '{}' protection has been removed on project {}",
					&branch, &project.path_with_namespace
				);
			}

			if CliContext::global().open {
				let _ = open::that(format!("{}/-/settings/repository", project.web_url));
			}

			Ok(ExitCode::from(0))
		}

		BranchesCmd::Protect(args) => {
			let project = CliContext::global().get_project(args.project.as_ref())?;
			let branch = CliContext::global().get_branchexp(args.branch.as_ref())?;

			let endpoint = ProtectedBranches::builder()
				.project(project.path_with_namespace.to_owned())
				.build()?;
			let tags: Vec<types::ProtectedRepoBranch> =
				endpoint.query(&CliContext::global().gitlab)?;

			// unprotect if found
			if tags.iter().any(|b| &b.name == branch) {
				let endpoint = UnprotectBranch::builder()
					.project(project.path_with_namespace.to_owned())
					.name(branch)
					.build()?;
				api::ignore(endpoint).query(&CliContext::global().gitlab)?;
			}
			// an protect again (parameters may have changed)
			let endpoint = ProtectBranch::builder()
				.project(project.path_with_namespace.to_owned())
				.name(branch)
				.allow_force_push(args.force_push)
				.build()?;
			let tag: Tag = endpoint
				.query(&CliContext::global().gitlab)
				.with_context(|| {
					format!(
						"Failed to protect branch '{}' on project {}",
						&branch, &project.path_with_namespace
					)
				})?;
			println!(
				"branch '{}' is protected on project {}",
				tag.name, &project.path_with_namespace
			);

			if CliContext::global().open {
				let _ = open::that(format!("{}/-/settings/repository", project.web_url));
			}

			Ok(ExitCode::from(0))
		}
	}
}
