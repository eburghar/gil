mod archive;
mod args;
mod cmd;
mod config;
mod lockfile;
mod utils;

use crate::{
	args::{Opts, SubCommand},
	cmd::{archive::cmd as archive, pipeline::cmd as pipeline, tags::cmd as tags},
	config::Config,
};
use anyhow::{bail, Context, Result};
use gitlab::Gitlab;

fn main() -> Result<()> {
	// parse command line arguments
	let opts: Opts = args::from_env();

	// read yaml config
	let config = Config::read(&opts.config)?;
	let token = if let Some(token) = config.token {
		token
	} else if let Some(oauth2) = config.oauth2 {
		// wip: implement oidc connexion to get the token
		oauth2.token
	} else {
		bail!("Set either a token or a oauth2 key in configuration file")
	};

	let gitlab = Gitlab::with_oauth2(&config.host, &token)
		.with_context(|| format!("Can't connect to {}", &config.host))?;

	match &opts.subcmd {
		SubCommand::Tags(args) => tags(gitlab, args),
		SubCommand::Build(args) => pipeline(gitlab, args),
		SubCommand::Archive(args) => {
			// extract the filename from the path to use as a local lock filename
			// we are sure at this point that the path is valid (has a filename component
			// and is convertible to string), so the unwrap are harmles
			let config_name = config
				.path
				.file_name()
				.unwrap()
				.to_owned()
				.into_string()
				.unwrap();
			archive(gitlab, &config_name, opts.verbose, args)
		}
	}
}
