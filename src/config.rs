use anyhow::{Context, Result};
use core::ops::Deref;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;

#[derive(Deserialize)]
pub struct Config {
	pub host: String,
	pub token: Option<String>,
	pub oauth2: Option<OAuth2>,
}

#[derive(Deserialize)]
pub struct OAuth2 {
	pub id: String,
	pub secret: String,
	// wip: for test purpose
	pub token: String,
}

impl Config {
	pub fn read(config: &str) -> Result<Self> {
		// open configuration file
		let file = File::open(&config).with_context(|| format!("Can't open {}", &config))?;
		// deserialize configuration
		let config: Self =
			serde_yaml::from_reader(file).with_context(|| format!("Can't read {}", &config))?;
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
