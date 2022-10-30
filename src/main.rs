mod archive;
mod args;
mod cmd;
mod color;
mod config;
mod context;
mod fmt;
mod git;
mod lockfile;
mod oidc;
mod utils;

use crate::{
	args::{Opts, SubCommand},
	cmd::{archive::cmd as archive, pipeline::cmd as pipeline, tags::cmd as tags},
	context::CliContext,
};

use anyhow::Result;

fn main() -> Result<()> {
	// parse command line arguments
	let opts: Opts = args::from_env();
	// construct context
	let context = CliContext::from_args(&opts)?;

	match &opts.cmd {
		SubCommand::Tags(args) => tags(&context, args),
		SubCommand::Build(args) => pipeline(&context, args),
		SubCommand::Archive(args) => archive(&context, args),
	}
}
