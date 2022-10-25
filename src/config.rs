use anyhow::{anyhow, Context, Result};
use core::ops::Deref;
use directories::ProjectDirs;
use serde::Deserialize;
use std::{collections::BTreeMap, env, fs::File, path::PathBuf};

#[derive(Deserialize)]
pub struct Config {
	pub host: String,
	pub token: Option<String>,
	pub oauth2: Option<OAuth2>,
	#[serde(skip)]
	pub path: PathBuf,
}

#[derive(Deserialize)]
pub struct OAuth2 {
	pub id: String,
	pub secret: String,
	// wip: for test purpose
	pub token: String,
}

impl Config {
	pub fn read(config: &Option<String>) -> Result<Self> {
		let config_path = if let Some(config) = config {
			PathBuf::from(config)
		} else {
			// returns the first configuration path that exists from that order
			// - GLCTL_CONFIG
			// - ~/.config/glctl/config.yaml
			// - .glctl_config.yaml
			//
			// first test from env var
			env::var("GLCTL_CONFIG")
				.ok()
				.map(PathBuf::from)
				.filter(|path| path.exists())
				// then test from project dir
				.or(ProjectDirs::from("me", "IT Sufficient", "GlCtl")
					.map(|path| path.config_dir().join("config.yaml"))
					.filter(|path| path.exists()))
				// then test in current directory
				.or(Some(PathBuf::from(".glctl_config.yaml")))
				.filter(|path| path.exists())
				//.and_then(|path| path.into_os_string().into_string().ok())
				// returns an error
				.ok_or(anyhow!("Unable to find a suitable configuration file"))?
		};

		println!("Reading configuration from {:?}", &config_path);
		// open configuration file
		let file =
			File::open(&config_path).with_context(|| format!("Can't open {:?}", &config_path))?;
		// deserialize configuration
		let mut config: Self = serde_yaml::from_reader(file)
			.with_context(|| format!("Can't read {:?}", &config_path))?;

		// save the choosen path
		config.path = config_path;
		Ok(config)
	}
}

#[derive(Deserialize)]
pub struct BatchConfig(BTreeMap<String, String>);

impl BatchConfig {
	pub fn singleton(project: String, tag: String) -> Self {
		let archives: BTreeMap<_, _> = [(project, tag)].into();
		Self(archives)
	}

	pub fn read(config: &str) -> Result<Self> {
		// open configuration file
		let file = File::open(&config).with_context(|| format!("Can't open {}", &config))?;
		// deserialize configuration
		let config: Self =
			serde_yaml::from_reader(file).with_context(|| format!("Can't read {}", &config))?;
		Ok(config)
	}
}

impl Deref for BatchConfig {
	type Target = BTreeMap<String, String>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
