use crate::{args, context::CONTEXT};

use anyhow::Result;

pub fn cmd(args: &args::Project) -> Result<()> {
	let project = CONTEXT.get_project(args.project.as_ref())?;
	// get a reference (a tag or a branch)
	let ref_ = CONTEXT.get_ref(args.ref_.as_ref(), &project)?;

	CONTEXT.print_project(&project, &ref_)?;
	if CONTEXT.open {
		let _ = open::that(format!("{}/-/tree/{}", &project.web_url, &ref_));
	}
	Ok(())
}
