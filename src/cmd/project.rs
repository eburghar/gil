use crate::{
	args::{self, ProjectCmd},
	context::CliContext,
};

use anyhow::{Context, Result};
use gitlab::api::{
	self,
	projects::{ArchiveProject, UnarchiveProject},
	Query,
};
use std::process::ExitCode;

pub fn cmd(args: &args::Project) -> Result<ExitCode> {
	match &args.cmd {
		ProjectCmd::Info(iargs) => {
			let project = CliContext::global().get_project(args.project.as_ref())?;
			// get a reference (a tag or a branch)
			let ref_ = CliContext::global().get_ref(iargs.ref_.as_deref(), &project)?;

			CliContext::global().print_project(&project, &ref_)?;
			if CliContext::global().open {
				let _ = open::that(format!("{}/-/tree/{}", &project.web_url, &ref_));
			}
			Ok(ExitCode::from(0))
		}
		ProjectCmd::Archive(_args) => {
			let project = CliContext::global().get_project(args.project.as_ref())?;
			let endpoint = ArchiveProject::builder()
				.project(project.id.value())
				.build()?;
			api::ignore(endpoint)
				.query(&CliContext::global().gitlab)
				.with_context(|| format!("failed to archive project {}", project.name))?;
			println!(
				"project {}({}) has been archived",
				&project.name, project.id
			);
			Ok(ExitCode::from(0))
		}
		ProjectCmd::Unarchive(_args) => {
			let project = CliContext::global().get_project(args.project.as_ref())?;
			let endpoint = UnarchiveProject::builder()
				.project(project.id.value())
				.build()?;
			api::ignore(endpoint)
				.query(&CliContext::global().gitlab)
				.with_context(|| format!("failed to unarchive project {}", project.name))?;
			println!(
				"project {}({}) has been unarchived",
				&project.name, project.id
			);
			Ok(ExitCode::from(0))
		}
	}
}
