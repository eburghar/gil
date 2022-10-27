use anyhow::{Context, Result};
use gitlab::{
	api::{projects::repository::tags::Tag, Query},
	Gitlab,
};
use serde::Deserialize;
use std::{
	fs::{create_dir_all, remove_dir_all},
	path::PathBuf,
};

#[derive(Debug, Deserialize)]
pub struct TagCommitRes {
	pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct TagRes {
	pub name: String,
	pub commit: TagCommitRes,
}

pub fn get_tag_commit(gitlab: &Gitlab, project: &str, tag: &str) -> Result<TagRes> {
	// get commit sha associated with tag
	let endpoint = Tag::builder()
		.project(project.to_owned())
		.tag_name(tag.to_owned())
		.build()?;
	let res: TagRes = endpoint.query(gitlab).with_context(|| {
		format!(
			"Failed to get commit info for tag {} on project {}",
			&tag, &project
		)
	})?;
	Ok(res)
}

pub fn get_or_create_dir(dir: &str, keep: bool, update: bool, verbose: bool) -> Result<PathBuf> {
	let path = PathBuf::from(dir);
	// remove destination dir if requested
	if !keep && !update && path.exists() {
		remove_dir_all(&path).with_context(|| format!("Can't remove dir {}", &dir))?;
		if verbose {
			println!("{} removed", &dir)
		}
	}
	// create destination dir if necessary
	if !path.exists() {
		create_dir_all(&path).with_context(|| format!("Can't create dir {}", &dir))?;
		if verbose {
			println!("Creating dir {}", &dir);
		}
	}
	Ok(path)
}
