mod api;
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
mod types;
mod utils;

use std::process::ExitCode;

use crate::{
	args::SubCommand,
	cmd::{
		archive::cmd as archive, branches::cmd as branches, keys::cmd as keys,
		pipeline::cmd as pipeline, project::cmd as project, tags::cmd as tags, token::cmd as token,
		users::cmd as users,
	},
	context::{CliContext, CONTEXT},
};

use anyhow::{anyhow, Result};

fn main() -> Result<ExitCode> {
	CONTEXT
		.set(CliContext::from_args(args::from_env())?)
		.map_err(|_| anyhow!("Can't set global context"))?;

	match &CliContext::global().cmd {
		SubCommand::Tags(args) => tags(args),
		SubCommand::Pipeline(args) => pipeline(args),
		SubCommand::Archive(args) => archive(args),
		SubCommand::Project(args) => project(args),
		SubCommand::Branches(args) => branches(args),
		SubCommand::Token(args) => token(args),
		SubCommand::Keys(args) => keys(args),
		SubCommand::Users(args) => users(args),
	}
}
