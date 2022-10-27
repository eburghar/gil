use argh::{FromArgs, TopLevelCommand};
use std::{env, path::Path};

#[derive(FromArgs)]
/// Interact with Gitlab API
pub struct Opts {
	#[argh(option, short = 'c')]
	/// configuration file containing gitlab connection parameters
	pub config: Option<String>,
	#[argh(switch, short = 'v')]
	/// more detailed output
	pub verbose: bool,
	#[argh(switch, short = 'o')]
	/// try to open links whenever possible
	pub open: bool,
	#[argh(switch)]
	/// don't save oidc login to cache
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
}

#[derive(FromArgs)]
/// Get and extract archives
#[argh(subcommand, name = "extract")]
pub struct ArchiveExtract {
	#[argh(option, short = 'p')]
	/// the project to extract archive from
	pub project: Option<String>,
	#[argh(option, short = 'b')]
	/// batch mode: yaml file containing a list of project and tag to extract
	pub batch: Option<String>,
	#[argh(positional)]
	/// tag to extract archive from
	pub tag: Option<String>,
	#[argh(option, short = 's', default = "0")]
	/// strip first n path components of every entries in archive before extraction
	pub strip: usize,
	#[argh(switch, short = 'r')]
	/// rename first directory of the archive to the name of the project
	pub rename: bool,
	#[argh(option, short = 'd', default = "\"tmp\".to_string()")]
	/// destination directory
	pub dir: String,
	#[argh(switch, short = 'k')]
	/// skip extraction of projects if a directory with same name already exists. by default destination directory is removed before extraction
	pub keep: bool,
	#[argh(switch, short = 'u')]
	/// update based on packages.lock file
	pub update: bool,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum ArchiveCmd {
	Extract(ArchiveExtract),
}

#[derive(FromArgs)]
/// Handle project archives
#[argh(subcommand, name = "archive")]
pub struct Archive {
	#[argh(subcommand)]
	/// operate on archive
	pub cmd: ArchiveCmd,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum TagsCmd {
	Protect(TagsProtect),
	Unprotect(TagsUnprotect),
}

#[derive(FromArgs)]
/// Protect a project tag(s)
#[argh(subcommand, name = "protect")]
pub struct TagsProtect {
	#[argh(option, short = 'p')]
	/// the project to protect tags from
	pub project: Option<String>,
	#[argh(positional, default = "\"*\".to_string()")]
	/// tag expression (default: *)
	pub tag: String,
}

#[derive(FromArgs)]
/// Unprotect a project tag(s)
#[argh(subcommand, name = "unprotect")]
pub struct TagsUnprotect {
	#[argh(option, short = 'p')]
	/// the project to protect tags from
	pub project: Option<String>,
	#[argh(positional, default = "\"*\".to_string()")]
	/// tag expression (defautl: *)
	pub tag: String,
}

#[derive(FromArgs)]
/// Manage project tags
#[argh(subcommand, name = "tags")]
pub struct Tags {
	#[argh(subcommand)]
	/// operate on tags
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

#[derive(FromArgs)]
/// Get pipeline status
#[argh(subcommand, name = "status")]
pub struct PipelineStatus {
	#[argh(option, short = 'p')]
	/// the project which owns the pipeline
	pub project: Option<String>,
	#[argh(positional)]
	/// pipeline id
	pub id: Option<u64>,
}

#[derive(FromArgs)]
/// Create a new pipeline
#[argh(subcommand, name = "create")]
pub struct PipelineCreate {
	#[argh(option, short = 'p')]
	/// the project which owns the pipeline
	pub project: Option<String>,
	#[argh(positional)]
	/// tag
	pub tag: Option<String>,
}

#[derive(FromArgs)]
/// Cancel a pipeline
#[argh(subcommand, name = "cancel")]
pub struct PipelineCancel {
	#[argh(option, short = 'p')]
	/// the project which owns the pipeline
	pub project: Option<String>,
	#[argh(positional)]
	/// pipeline id
	pub id: Option<u64>,
}

#[derive(FromArgs)]
/// Retry a pipeline
#[argh(subcommand, name = "retry")]
pub struct PipelineRetry {
	#[argh(option, short = 'p')]
	/// the project which owns the pipeline
	pub project: Option<String>,
	#[argh(positional)]
	/// pipeline id
	pub id: Option<u64>,
}

#[derive(FromArgs)]
/// Get log from a job
#[argh(subcommand, name = "log")]
pub struct PipelineLog {
	#[argh(option, short = 'p')]
	/// the project which owns the pipeline
	pub project: Option<String>,
	#[argh(positional)]
	/// the job id to extract the job log from
	pub id: Option<u64>,
}

#[derive(FromArgs)]
/// Manage project pipeline
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
