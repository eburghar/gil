use crate::context::Job;

use anyhow::{Context, Result};
use std::{
	fs::{create_dir_all, remove_dir_all},
	path::PathBuf,
};

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

/// Print the provided jobs list in reverse order (run order)
pub fn print_jobs(message: String, jobs: &[Job]) {
	println!("{}", message);
	for job in jobs.iter().rev() {
		println!("- #{} {} [{}]: {}", job.id, job.name, job.stage, job.status);
	}
	println!("\n");
}
