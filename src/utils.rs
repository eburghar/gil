use anyhow::{bail, Context, Result};
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
	let res: TagRes = endpoint.query(gitlab).context(format!(
		"Failed to get commit info for tag {} on project {}",
		&tag, &project
	))?;
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

pub fn get_project<'a>(project: &'a Option<String>) -> Result<&'a String> {
	if let Some(project) = project {
		Ok(&project)
	} else {
		// TODO: try to extract the project name from current git repo when none is given
		bail!("Unable to determine the project automatically. Specify one manually.")
	}
}

pub fn get_tagexpr<'a>(tag: &'a Option<String>) -> Result<&'a str> {
	if let Some(tag) = tag {
		Ok(&tag)
	} else {
		// TODO: try to extract the project latest tag from current git or gitlab api repo
		// when none is given before failback to *
		Ok("*")
	}
}

pub fn get_tag<'a>(tag: &'a Option<String>) -> Result<&'a str> {
	if let Some(tag) = tag {
		Ok(&tag)
	} else {
		// TODO: try to extract the project latest tag from current git repo when none is given
		bail!("Unable to determine the project latest tag automatically. Specify one manually.")
	}
}

pub fn get_pipeline<'a>(id: Option<u64>) -> Result<u64> {
	if let Some(id) = id {
		Ok(id)
	} else {
		// TODO: try to extract the project name from current git repo when none is given
		bail!("Unable to determine the project latest pipeline id automatically. Specify one manually.")
	}
}
