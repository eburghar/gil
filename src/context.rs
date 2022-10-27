use crate::{
	args::Opts,
	config::{AuthType, Config, OAuth2Token},
	git::GitProject,
};

use anyhow::{anyhow, Context, Result};
use gitlab::Gitlab;

/// Structure to pass around functions containing informations
/// about execution context
pub struct CliContext {
	/// verbose mode
	pub verbose: bool,
	/// open links automatically
	pub open: bool,
	/// the gitlab connexion
	pub gitlab: Gitlab,
	/// the configuration file
	pub config: Config,
	/// information about the current git repo
	pub repo: Option<GitProject>,
}

impl CliContext {
	/// Inializer from cli arguments
	pub fn from_args(opts: &Opts) -> Result<Self> {
		// read yaml config
		let config = Config::from_file(opts.config.as_ref(), opts.verbose)?;

		// get information from git
		let repo = GitProject::from_currentdir();

		// connect to gitlab
		let gitlab = match &config.auth {
			AuthType::OAuth2(oauth2) => {
				// try to get the token from cache
				if let Some(token) = OAuth2Token::from_cache() {
					// check if we can login with that
					if let Ok(gitlab) = Gitlab::with_oauth2(&config.host, &token.token) {
						Ok(gitlab)
					// otherwise try relogin
					} else {
						let cache = OAuth2Token::from_login(&config.host, oauth2, opts)?;
						Gitlab::with_oauth2(&config.host, &cache.token)
					}
				// otherwise try to login
				} else {
					let cache = crate::oidc::login(&config.host, oauth2, opts)?;
					Gitlab::with_oauth2(&config.host, &cache.token)
				}
			}

			AuthType::Token(token) => Gitlab::new(&config.host, token),
		}
		.with_context(|| format!("Can't connect to {}", &config.host))?;

		Ok(Self {
			verbose: opts.verbose,
			open: opts.open,
			gitlab,
			config,
			repo,
		})
	}

	/// Returns the provided project name (default) or the one extracted from the repo url
	/// or raises an error
	pub fn project<'a>(&'a self, default: Option<&'a String>) -> Result<&'a String> {
		default
			.or_else(|| self.repo.as_ref().and_then(|repo| repo.name.as_ref()))
			.ok_or_else(|| {
				anyhow!("Can't find a project name. Specify one manually on the command line")
			})
	}

	/// Returns the provided tag name (default) or the one extracted from the repo
	/// or raises an error
	pub fn tag<'a>(&'a self, default: Option<&'a String>) -> Result<&'a String> {
		default
			.or_else(|| self.repo.as_ref().and_then(|repo| repo.tag.as_ref()))
			.ok_or_else(|| {
				anyhow!("Can'f find a project tag. Specify one manually on the command line")
			})
	}
}
