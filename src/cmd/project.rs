use crate::{args, context::CliContext};

use anyhow::Result;

pub fn cmd(args: &args::Project) -> Result<()> {
	let project = CliContext::global().get_project(args.project.as_ref())?;
	// get a reference (a tag or a branch)
	let ref_ = CliContext::global().get_ref(args.ref_.as_ref(), &project)?;

	CliContext::global().print_project(&project, &ref_)?;
	if CliContext::global().open {
		let _ = open::that(format!("{}/-/tree/{}", &project.web_url, &ref_));
	}
	Ok(())
}
