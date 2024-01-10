use crate::{args::Opts, oidc::login};

use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
	collections::HashMap,
	env,
	ffi::OsStr,
	fs::{create_dir_all, File},
	ops::Deref,
	path::PathBuf,
};

static ORG: &str = "ITSufficient";

/// Root configuration file
#[derive(Deserialize)]
pub struct Config {
	pub hosts: HashMap<String, HostConfig>,
	#[serde(skip)]
	/// filename associated to the config file
	pub name: String,
}

/// Root configuration file
#[derive(Deserialize)]
pub struct HostConfig {
	/// remote name
	#[serde(default = "default_remote")]
	pub remote: String,
	/// host CA
	pub ca: Option<String>,
	/// auth type
	pub auth: AuthType,
}

fn default_remote() -> String {
	"origin".to_owned()
}

/// Authentication type supported
#[derive(Deserialize)]
#[serde(untagged)]
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
	/// - GIL_CONFIG environment variable
	/// - HOME directory: ~/.config/gil/config.yaml
	/// - Current directory: .gil_config.yaml
	pub fn from_file<T>(path: Option<&T>, verbose: bool) -> Result<Self>
	where
		T: AsRef<OsStr>,
	{
		// if a config path was given, try that
		let config_path = if let Some(config) = path {
			PathBuf::from(config)
		// otherwise try to find a configuration file from
		} else {
			// first test from env var
			env::var("GIL_CONFIG")
				.ok()
				.map(PathBuf::from)
				.filter(|path| path.exists())
				// then test from project dir
				.or_else(|| {
					ProjectDirs::from("me", ORG, env!("CARGO_BIN_NAME"))
						.map(|dir| dir.config_dir().join("config.yaml"))
						.filter(|path| path.exists())
				})
				// then test in current directory
				.or_else(|| Some(PathBuf::from(".gil_config.yaml")))
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
pub struct OAuth2Token(String);

impl OAuth2Token {
	/// Initializer
	pub fn new(token: String) -> Self {
		Self(token)
	}

	/// Try silently read the cache file
	pub fn from_cache() -> Option<Self> {
		ProjectDirs::from("me", ORG, env!("CARGO_BIN_NAME"))
			.map(|dir| dir.cache_dir().join("oidc_login"))
			.and_then(|path| {
				File::open(path)
					.ok()
					.and_then(|file| serde_yaml::from_reader(file).ok())
			})
	}

	/// Try to login
	pub fn from_login(
		host: &String,
		ca: &Option<String>,
		config: &OAuth2,
		opts: &Opts,
	) -> Result<Self> {
		login(host, ca, config, opts)
	}

	/// Try to save the cache information to file
	pub fn save(&self) -> Result<()> {
		ProjectDirs::from("me", ORG, env!("CARGO_BIN_NAME"))
			.ok_or_else(|| anyhow!("Unable to find a suitable cache file path for oidc login"))
			.map(|dir| dir.cache_dir().join("oidc_login"))
			.and_then(|path| {
				// create directories
				if let Some(p) = path.parent() {
					create_dir_all(p)?
				}
				File::create(path)
					.with_context(|| "Unable to open the cache file")
					.and_then(|file| {
						serde_yaml::to_writer(file, &self)
							.with_context(|| "Unable to serialize oidc login informations")
					})
			})
	}
}

#[allow(clippy::from_over_into)]
impl Into<String> for OAuth2Token {
	fn into(self) -> String {
		self.0
	}
}

impl Deref for OAuth2Token {
	type Target = String;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
