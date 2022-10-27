use crate::{args::Opts, oidc::login};

use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{env, fs::File, path::PathBuf};

/// Root configuration file
#[derive(Deserialize)]
pub struct Config {
	/// gitlab host
	pub host: String,
	// auth type
	pub auth: AuthType,
	#[serde(skip)]
	/// filename associated to the config file
	pub name: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
/// Authentication type supported
pub enum AuthType {
	/// access token
	Token(String),
	/// oauth2 config
	OAuth2(OAuth2),
}

/// Oidc configuration part
#[derive(Deserialize)]
pub struct OAuth2 {
	/// id used to identity ourselves to the oauth2 provider
	pub id: String,
	/// secret used with the oauth2 provider
	pub secret: String,
	#[serde(rename = "redirect-port")]
	/// port used to receive that authentication code
	pub redirect_port: u16,
}

impl Config {
	/// Initialiser from an optional file path.
	/// If no path is given, it will try to find one from
	/// - GLCTL_CONFIG environment variable
	/// - HOME directory: ~/.config/glctl/config.yaml
	/// - Current directory: .glctl_config.yaml
	pub fn from_file(path: Option<&String>, verbose: bool) -> Result<Self> {
		// if a config path was given, try that
		let config_path = if let Some(config) = path {
			PathBuf::from(config)
		// otherwise try to find a configuration file from
		} else {
			// first test from env var
			env::var("GLCTL_CONFIG")
				.ok()
				.map(PathBuf::from)
				.filter(|path| path.exists())
				// then test from project dir
				.or_else(|| {
					ProjectDirs::from("me", "IT Sufficient", "GlCtl")
						.map(|dir| dir.config_dir().join("config.yaml"))
						.filter(|path| path.exists())
				})
				// then test in current directory
				.or_else(|| Some(PathBuf::from(".glctl_config.yaml")))
				.filter(|path| path.exists())
				// finally return an error if nothing worked
				.ok_or_else(|| anyhow!("Unable to find a suitable configuration file"))?
		};

		if verbose {
			println!("Reading configuration from {:?}", &config_path);
		}
		// open configuration file
		let file =
			File::open(&config_path).with_context(|| format!("Can't open {:?}", &config_path))?;
		// deserialize configuration
		let mut config: Self = serde_yaml::from_reader(file)
			.with_context(|| format!("Can't read {:?}", &config_path))?;

		// save the config filename for later use
		// the config has been read from a file so the unwrap is harmles
		config.name = config_path
			.file_name()
			.map(|name| name.to_string_lossy().to_string())
			.unwrap();
		// config.path = config_path;
		Ok(config)
	}
}

/// OAuth2 login token
#[derive(Deserialize, Serialize)]
pub struct OAuth2Token {
	pub token: String,
}

impl OAuth2Token {
	/// Initializer
	pub fn new(token: String) -> Self {
		Self { token }
	}

	/// Try silentely read the cache file
	pub fn from_cache() -> Option<Self> {
		ProjectDirs::from("me", "IT Sufficient", "GlCtl")
			.map(|dir| dir.cache_dir().join("oidc_login"))
			.and_then(|path| {
				File::open(path)
					.ok()
					.and_then(|file| serde_yaml::from_reader(file).ok())
			})
	}

	/// Try to login
	pub fn from_login(host: &String, config: &OAuth2, opts: &Opts) -> Result<Self> {
		login(host, config, opts)
	}

	/// Try to save the cache information to file
	pub fn save(&self) -> Result<()> {
		ProjectDirs::from("me", "IT Sufficient", "GlCtl")
			.ok_or_else(|| anyhow!("Unable to find a suitable cache file path for oidc login"))
			.map(|dir| dir.cache_dir().join("oidc_login"))
			.and_then(|path| {
				File::create(path)
					.with_context(|| "Unable to open the cache file")
					.and_then(|file| {
						serde_yaml::to_writer(file, &self)
							.with_context(|| "Unable to serialize oidc login informations")
					})
			})
	}
}
