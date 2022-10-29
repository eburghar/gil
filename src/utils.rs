use anyhow::{Context, Result};
use gitlab::{types, StatusState};
use std::{
	fs::{create_dir_all, remove_dir_all},
	path::PathBuf,
};

use crate::{
	args::ColorChoice,
	color::{Style, StyledStr},
	fmt::{Colorizer, Stream},
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
pub fn print_jobs(mut msg: StyledStr, mode: ColorChoice, jobs: &[types::Job]) -> Result<()> {
	if !jobs.is_empty() {
		msg.none("\n");
		for job in jobs.iter().rev() {
			msg.none("- Job ");
			msg.literal(format!("{}", job.id));
			msg.none(format!(" {} ", job.name));
			msg.hint(format!("[{}]", job.stage));
			msg.none(": ");
			msg.stylize(status_style(job.status), format!("{:?}", job.status));
			msg.none("\n");
		}
		msg.none("\n");
	}
	Colorizer::new(Stream::Stdout, mode)
		.with_content(msg)
		.print()
		.with_context(|| "Failed to print")
}

pub(crate) fn status_style(status: StatusState) -> Option<Style> {
	Some(match status {
		StatusState::Success | StatusState::Running => Style::Good,
		StatusState::Canceled | StatusState::Failed => Style::Error,
		StatusState::WaitingForResource | StatusState::Skipped | StatusState::Pending => {
			Style::Warning
		}
		StatusState::Created
		| StatusState::Manual
		| StatusState::Preparing
		| StatusState::Scheduled => Style::Placeholder,
	})
}
