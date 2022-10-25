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
	// initialize env_logger in info mode for rconfd by default
	env_logger::init_from_env(env_logger::Env::new().default_filter_or("glctl=info"));
	log::info!("{} v{}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));

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
		SubCommand::Tags(args) => tags(gitlab, &opts, args),
		SubCommand::Build(args) => pipeline(gitlab, &opts, args),
		SubCommand::Archive(args) => archive(gitlab, &opts, args),
	}
}
