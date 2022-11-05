#[cfg(feature = "color")]
use argh::FromArgValue;
use argh::{FromArgs, TopLevelCommand};
use std::{env, path::Path};

/// Color mode
#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ColorChoice {
	Auto,
	Always,
	Never,
}

#[cfg(feature = "color")]
impl FromArgValue for ColorChoice {
	fn from_arg_value(value: &str) -> Result<Self, String> {
		if value == "auto" {
			Ok(Self::Auto)
		} else if value == "always" {
			Ok(Self::Always)
		} else if value == "never" {
			Ok(Self::Never)
		} else {
			Err(format!(
				"{} not supported for --color. Use either \"auto\", \"always\" or \"never\"",
				value
			))
		}
	}
}

/// Interact with Gitlab API
#[derive(FromArgs)]
pub struct Opts {
	/// configuration file containing gitlab connection parameters
	#[argh(option, short = 'c')]
	pub config: Option<String>,

	/// more detailed output
	#[argh(switch, short = 'v')]
	pub verbose: bool,

	/// try to open links whenever possible
	#[argh(switch, short = 'o')]
	pub open: bool,

	/// show urls
	#[argh(switch, short = 'u')]
	pub url: bool,

	#[cfg(feature = "color")]
	/// color mode: auto (default), always or never
	#[argh(option, default = "ColorChoice::Auto")]
	pub color: ColorChoice,

	/// don't save oidc login to cache
	#[argh(switch)]
	pub no_cache: bool,

	#[argh(subcommand)]
	pub cmd: SubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum SubCommand {
	Tags(Tags),
	Build(Pipeline),
	Archive(Archive),
	Project(Project),
}

/// Get and extract archives
#[derive(FromArgs)]
#[argh(subcommand, name = "extract")]
pub struct ArchiveExtract {
	/// the project to extract archive from
	#[argh(option, short = 'p')]
	pub project: Option<String>,

	/// batch mode: yaml file containing a list of project and tag to extract
	#[argh(option, short = 'b')]
	pub batch: Option<String>,

	/// strip first n path components of every entries in archive before extraction
	#[argh(option, short = 's', default = "0")]
	pub strip: usize,

	/// rename first directory of the archive to the name of the project
	#[argh(switch, short = 'r')]
	pub rename: bool,

	/// destination directory
	#[argh(option, short = 'd', default = "\"tmp\".to_string()")]
	pub dir: String,

	/// skip extraction of projects if a directory with same name already exists. by default destination directory is removed before extraction
	#[argh(switch, short = 'k')]
	pub keep: bool,

	/// update based on packages.lock file
	#[argh(switch, short = 'u')]
	pub update: bool,

	/// reference (tag or branch) to extract an archive from
	#[argh(positional)]
	pub ref_: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum ArchiveCmd {
	Extract(ArchiveExtract),
}

/// Handle project archives
#[derive(FromArgs)]
#[argh(subcommand, name = "archive")]
pub struct Archive {
	/// operate on archive
	#[argh(subcommand)]
	pub cmd: ArchiveCmd,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum TagsCmd {
	Protect(TagsProtect),
	Unprotect(TagsUnprotect),
}

/// Protect a project tag(s)
#[derive(FromArgs)]
#[argh(subcommand, name = "protect")]
pub struct TagsProtect {
	/// the project to protect tags from
	#[argh(option, short = 'p')]
	pub project: Option<String>,

	/// tag expression: '*' (default)
	#[argh(positional, default = "\"*\".to_string()")]
	pub tag: String,
}

/// Unprotect a project tag(s)
#[derive(FromArgs)]
#[argh(subcommand, name = "unprotect")]
pub struct TagsUnprotect {
	/// the project to protect tags from
	#[argh(option, short = 'p')]
	pub project: Option<String>,

	/// tag expression: '*' (default)
	#[argh(positional, default = "\"*\".to_string()")]
	pub tag: String,
}

/// Manage project tags
#[derive(FromArgs)]
#[argh(subcommand, name = "tags")]
pub struct Tags {
	/// operate on tags
	#[argh(subcommand)]
	pub cmd: TagsCmd,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum PipelineCmd {
	Status(PipelineStatus),
	Create(PipelineCreate),
	Cancel(PipelineCancel),
	Retry(PipelineRetry),
	Log(PipelineLog),
}

/// Get pipeline status
#[derive(FromArgs)]
#[argh(subcommand, name = "status")]
pub struct PipelineStatus {
	/// the project which owns the pipeline
	#[argh(option, short = 'p')]
	pub project: Option<String>,

	/// reference (tag or branch)
	#[argh(option, short = 'r')]
	pub ref_: Option<String>,

	/// pipeline id
	#[argh(positional)]
	pub id: Option<u64>,
}

/// Create a new pipeline
#[derive(FromArgs)]
#[argh(subcommand, name = "create")]
pub struct PipelineCreate {
	/// the project which owns the pipeline
	#[argh(option, short = 'p')]
	pub project: Option<String>,

	/// reference (tag or branch)
	#[argh(positional)]
	pub ref_: Option<String>,
}

/// Cancel a pipeline
#[derive(FromArgs)]
#[argh(subcommand, name = "cancel")]
pub struct PipelineCancel {
	/// the project which owns the pipeline
	#[argh(option, short = 'p')]
	pub project: Option<String>,

	/// reference (tag or branch)
	#[argh(option, short = 'r')]
	pub ref_: Option<String>,

	/// pipeline id
	#[argh(positional)]
	pub id: Option<u64>,
}

/// Retry a pipeline
#[derive(FromArgs)]
#[argh(subcommand, name = "retry")]
pub struct PipelineRetry {
	/// the project which owns the pipeline
	#[argh(option, short = 'p')]
	pub project: Option<String>,

	/// reference (tag or branch)
	#[argh(option, short = 'r')]
	pub ref_: Option<String>,

	/// pipeline id
	#[argh(positional)]
	pub id: Option<u64>,
}

/// Get log from a job
#[derive(FromArgs)]
#[argh(subcommand, name = "log")]
pub struct PipelineLog {
	/// the project which owns the pipeline
	#[argh(option, short = 'p')]
	pub project: Option<String>,

	/// reference (tag or branch)
	#[argh(option, short = 'r')]
	pub ref_: Option<String>,

	/// a name that partially match the section name(s) to show in the log: step_script (default)
	#[argh(option, short = 's', default = "\"step_script\".to_string()")]
	pub section: String,

	/// show all sections
	#[argh(switch, short = 'a')]
	pub all: bool,

	/// show section headers
	#[argh(switch, short = 'h')]
	pub headers: bool,

	/// show only section headers (all collapsed)
	#[argh(switch, short = 'H')]
	pub only_headers: bool,

	/// the job id to extract the job log from
	#[argh(positional)]
	pub id: Option<u64>,
}

/// Manage project pipeline
#[derive(FromArgs)]
#[argh(subcommand, name = "pipeline")]
pub struct Pipeline {
	#[argh(subcommand)]
	/// operate on pipeline
	pub cmd: PipelineCmd,
}

/// copy of argh::from_env to insert command name and version in help text
pub fn from_env<T: TopLevelCommand>() -> T {
	let args: Vec<String> = std::env::args().collect();
	let cmd = Path::new(&args[0])
		.file_name()
		.and_then(|s| s.to_str())
		.unwrap_or(&args[0]);
	let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
	T::from_args(&[cmd], &args_str[1..]).unwrap_or_else(|early_exit| {
		println!("{} {}\n", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));
		println!("{}", early_exit.output);
		std::process::exit(match early_exit.status {
			Ok(()) => 0,
			Err(()) => 1,
		})
	})
}

/// Display information about project
#[derive(FromArgs)]
#[argh(subcommand, name = "project")]
pub struct Project {
	/// the project to protect tags from
	#[argh(option, short = 'p')]
	pub project: Option<String>,

	/// reference (tag or branch)
	#[argh(positional)]
	pub ref_: Option<String>,
}
