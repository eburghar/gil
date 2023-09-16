use crate::{args, context::CliContext};

use anyhow::Result;

pub fn cmd(context: &CliContext, args: &args::Project) -> Result<()> {
    let project = context.get_project(args.project.as_ref())?;
    // get a reference (a tag or a branch)
    let ref_ = context.get_ref(args.ref_.as_ref(), &project)?;

    context.print_project(&project, &ref_)?;
    if context.open {
        let _ = open::that(format!("{}/-/tree/{}", &project.web_url, &ref_));
    }
    Ok(())
}
