use anyhow::{Context, Result};
use gitlab::{types, StatusState};
use std::{
	fs::{create_dir_all, remove_dir_all},
	io::{stdout, Write},
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
		remove_dir_all(&path).with_context(|| format!("Can't remove dir {}", dir))?;
		if verbose {
			println!("{} removed", &dir)
		}
	}
	// create destination dir if necessary
	if !path.exists() {
		create_dir_all(&path).with_context(|| format!("Can't create dir {}", dir))?;
		if verbose {
			println!("Creating dir {}", &dir);
		}
	}
	Ok(path)
}

pub fn print_log(log: &[u8], job: &types::Job, mode: ColorChoice) -> Result<()> {
	let mut msg = StyledStr::new();
	msg.none("Log for job ");
	msg.literal(job.id.to_string());
	msg.none(": ");
	msg.stylize(status_style(job.status), format!("{:?}", job.status));
	msg.hint(format!(" ({})", job.web_url));
	msg.none("\n\n");
	Colorizer::new(Stream::Stdout, mode)
		.with_content(msg)
		.print()?;
	// TODO: remove ansi-codes depending on color mode
	stdout().write_all(log)?;
	Ok(())
}

pub fn print_msg(msg: StyledStr, mode: ColorChoice) -> Result<()> {
	Colorizer::new(Stream::Stdout, mode)
		.with_content(msg)
		.print()
		.with_context(|| "Failed to print")
}

pub fn print_pipeline(
	pipeline: &types::PipelineBasic,
	project: &types::Project,
	ref_: &String,
	mode: ColorChoice,
) -> Result<()> {
	let mut msg = StyledStr::new();
	msg.none("Pipeline ");
	msg.literal(pipeline.id.value().to_string());
	msg.none(format!(
		" ({} @ {}): ",
		project.name_with_namespace.as_str(),
		&ref_
	));
	msg.stylize(
		status_style(pipeline.status),
		format!("{:?}", pipeline.status),
	);
	msg.hint(format!(" ({})", pipeline.web_url));
	msg.none("\n");
	print_msg(msg, mode)
}

/// Print the provided jobs list in reverse order (run order)
pub fn print_jobs(jobs: &[types::Job], mode: ColorChoice) -> Result<()> {
	let mut msg = StyledStr::new();
	if !jobs.is_empty() {
		for job in jobs.iter().rev() {
			msg.none("- Job ");
			msg.literal(job.id.to_string());
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
		| StatusState::Scheduled => Style::Literal,
	})
}
