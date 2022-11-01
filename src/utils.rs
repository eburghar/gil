use anyhow::{Context, Result};
use gitlab::{types, StatusState};
use std::{
	fs::{create_dir_all, remove_dir_all},
	path::PathBuf,
	str::FromStr,
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

pub fn print_log(
	log: &[u8],
	job: &types::Job,
	step: Option<&String>,
	mode: ColorChoice,
) -> Result<()> {
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

	_print_log(log, step, mode)
}

#[derive(Debug, PartialEq)]
/// Marker for section start and end
enum SectionType {
	Start,
	End,
}

impl FromStr for SectionType {
	type Err = SectionError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if s == "section_start" {
			Ok(SectionType::Start)
		} else if s == "section_end" {
			Ok(SectionType::End)
		} else {
			Err(SectionError)
		}
	}
}

#[derive(Clone, Debug)]
struct SectionError;

#[derive(Debug)]
/// Parsing result of a log section
struct Section {
	type_: SectionType,
	// id: String,
	name: String,
}

impl FromStr for Section {
	type Err = SectionError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if s.starts_with("section_start:") || s.starts_with("section_end:") {
			// section_type:id:name
			let info: [&str; 3] = s
				// remove leading \r
				.trim()
				.splitn(3, ':')
				.collect::<Vec<&str>>()
				.try_into()
				.unwrap();
			let type_ = SectionType::from_str(info[0])?;
			Ok(Self {
				type_,
				// id: info[1].to_owned(),
				name: info[2].to_owned(),
			})
		} else {
			Err(SectionError)
		}
	}
}

/// Type of current line in the parser
enum State {
	Text,
	Section(Section),
}

/// Structure to drive the sections parsing
struct StateMachine {
	pub state: State,
	pub sections: Vec<String>,
}

impl Default for StateMachine {
	fn default() -> Self {
		Self {
			state: State::Text,
			sections: Vec::<String>::default(),
		}
	}
}

impl StateMachine {
	fn show_line(&self, step: Option<&String>) -> bool {
		// show line if we have no filter or we are outside of any section
		// or inside a section of the name provided
		step.is_none()
			|| self.sections.is_empty()
			|| self.sections.iter().any(|name| name == step.unwrap())
	}
}

fn print_section(title: &str, section: &Section, mode: ColorChoice) -> Result<()> {
	let mut msg = StyledStr::new();

	msg.warning(format!("\n> {} [", title));
	msg.literal(&section.name);
	msg.warning("]");
	msg.none("\n");

	print_msg(msg, mode)
}

/// parse the log coming from gitlab and filter sections if necessary
fn _print_log(log: &[u8], step: Option<&String>, mode: ColorChoice) -> Result<()> {
	use std::io::{BufRead, BufReader};

	let colored =
		mode == ColorChoice::Always || mode == ColorChoice::Auto && atty::is(atty::Stream::Stdout);

	let mut reader = BufReader::new(log).lines();
	let mut state = StateMachine::default();
	while let Some(Ok(line)) = reader.next() {
		// evaluate show_line for each line
		let mut show_line = state.show_line(step);
		for (_effect, s) in yew_ansi::get_sgr_segments(&line) {
			// reevaluate show_line for each segments
			show_line = state.show_line(step);
			match state.state {
				State::Text => {
					if let Ok(section) = Section::from_str(s) {
						state.sections.push(section.name.clone());
						state.state = State::Section(section);
					} else {
						// when not in color mode we need to print the segment without style
						if show_line && !colored {
							let mut msg = StyledStr::new();
							msg.none(s);
							print_msg(msg, mode)?;
						}
					}
				}
				State::Section(ref section) => {
					// start of new section
					if section.type_ == SectionType::Start {
						print_section(s, section, mode)?;
						state.state = State::Text;
						// line has already been printed so force to skip in colored mode
						if colored {
							if show_line {
								print_msg("\n".into(), mode)?;
							}
							show_line = false;
						}
					// end of a section
					} else {
						state.sections.pop();
						if let Ok(section) = Section::from_str(s) {
							state.sections.push(section.name.clone());
							state.state = State::Section(section);
						} else {
							state.state = State::Text;
						}
						// line has already been printed so force to skip in colored mode
						if colored {
							show_line = false;
						}
					}
				}
			}
		}
		if show_line {
			let mut msg = StyledStr::new();
			if colored {
				msg.none(line);
			}
			msg.none("\n");
			print_msg(msg, mode)?;
		}
	}

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
