use crate::Config;
use anyhow::{Context, Result};
use gitlab::{
	api::{projects::repository::tags::Tag, Query},
	Gitlab,
};
use serde_json::Value;

pub fn cmd(config: &Config) -> Result<()> {
	// connect to gitlab instance using host and token from config file
	let gitlab = Gitlab::with_oauth2(&config.host, &config.token)
		// Gitlab::new(&config.host, &config.token)
		.with_context(|| format!("Can't connect to {}", &config.host))?;

	// print project path and last commit hash
	// iterate over each project name indicated in the config file
	for (prj, br) in config.archives.iter() {
		let endpoint = Tag::builder()
			.project(prj.to_owned())
			.tag_name(br.to_owned())
			.build()
			.unwrap();
		let value: Value = endpoint.query(&gitlab)?;
		let commit = value
			.get("commit")
			.and_then(|value| value.get("id"))
			.map(|value| value.as_str().unwrap_or_default())
			.unwrap();
		log::info!("{}:{}", &prj, commit);
	}
	Ok(())
}
