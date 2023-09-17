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

#[derive(Deserialize)]
struct Tag {
    name: String,
}

pub fn cmd(context: &CliContext, args: &args::Branches) -> Result<()> {
    match &args.cmd {
        BranchesCmd::Unprotect(args) => {
            let project = context.get_project(args.project.as_ref())?;
            let branch = context.get_branchexp(args.branch.as_ref())?;

            let endpoint = ProtectedBranches::builder()
                .project(project.path_with_namespace.to_owned())
                .build()?;
            let tags: Vec<types::ProtectedRepoBranch> = endpoint.query(&context.gitlab)?;

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
                api::ignore(endpoint).query(&context.gitlab)?;
                println!(
                    "branch '{}' protection has been removed on project {}",
                    &branch, &project.path_with_namespace
                );
            }

            if context.open {
                let _ = open::that(format!("{}/-/settings/repository", project.web_url));
            }

            Ok(())
        }

        BranchesCmd::Protect(args) => {
            let project = context.get_project(args.project.as_ref())?;
            let branch = context.get_branchexp(args.branch.as_ref())?;

            let endpoint = ProtectedBranches::builder()
                .project(project.path_with_namespace.to_owned())
                .build()?;
            let tags: Vec<types::ProtectedRepoBranch> = endpoint.query(&context.gitlab)?;

            // unprotect if found
            if tags.iter().any(|b| &b.name == branch) {
                let endpoint = UnprotectBranch::builder()
                    .project(project.path_with_namespace.to_owned())
                    .name(branch)
                    .build()?;
                api::ignore(endpoint).query(&context.gitlab)?;
            }
            // an protect again (parameters may have changed)
            let endpoint = ProtectBranch::builder()
                .project(project.path_with_namespace.to_owned())
                .name(branch)
                .allow_force_push(args.force_push)
                .build()?;
            let tag: Tag = endpoint.query(&context.gitlab).with_context(|| {
                format!(
                    "Failed to protect branch '{}' on project {}",
                    &branch, &project.path_with_namespace
                )
            })?;
            println!(
                "branch '{}' is protected on project {}",
                tag.name, &project.path_with_namespace
            );

            if context.open {
                let _ = open::that(format!("{}/-/settings/repository", project.web_url));
            }

            Ok(())
        }
    }
}
