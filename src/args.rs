use argh::{FromArgs, TopLevelCommand};
use std::path::Path;

#[derive(FromArgs)]
/// Interact with Gitlab API
pub struct Opts {
	#[argh(option, short = 'c')]
	/// configuration file containing gitlab connection parameters
	pub config: String,
	#[argh(switch, short = 'v')]
	/// more detailed output
	pub verbose: bool,
	#[argh(subcommand)]
	pub subcmd: SubCommand,
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
	pub strip: u8,
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
/// hangle project archive
#[argh(subcommand, name = "archive")]
pub struct Archive {
	#[argh(subcommand)]
	/// operate on tags
	pub cmd: ArchiveCmd,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum TagsCmd {
	Protect(TagsProtect),
	Unprotect(TagsUnprotect),
}

#[derive(FromArgs)]
/// protect tags using an expression
#[argh(subcommand, name = "protect")]
pub struct TagsProtect {
	#[argh(option, short = 'p')]
	/// the project to protect tags from
	pub project: Option<String>,
	#[argh(positional)]
	/// tag expression
	pub tag: Option<String>,
}

#[derive(FromArgs)]
/// unprotect tags using an expression
#[argh(subcommand, name = "unprotect")]
pub struct TagsUnprotect {
	#[argh(option, short = 'p')]
	/// the project to protect tags from
	pub project: Option<String>,
	#[argh(positional)]
	/// tag expression
	pub tag: Option<String>,
}

#[derive(FromArgs)]
/// manage project tags
#[argh(subcommand, name = "tags")]
pub struct Tags {
	#[argh(subcommand)]
	/// operate on tags
	pub cmd: TagsCmd,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum PipelineCmd {
	Get(PipelineGet),
	Create(PipelineCreate),
	Cancel(PipelineCancel),
	Retry(PipelineRetry),
}

#[derive(FromArgs)]
/// get pipeline status
#[argh(subcommand, name = "get")]
pub struct PipelineGet {
	#[argh(option, short = 'p')]
	/// the project to build
	pub project: Option<String>,
	#[argh(positional)]
	/// pipeline id
	pub id: Option<u64>,
}

#[derive(FromArgs)]
/// get pipeline status
#[argh(subcommand, name = "create")]
pub struct PipelineCreate {
	#[argh(option, short = 'p')]
	/// the project to build
	pub project: Option<String>,
	#[argh(positional)]
	/// tag
	pub tag: Option<String>,
}

#[derive(FromArgs)]
/// cancel a pipeline
#[argh(subcommand, name = "cancel")]
pub struct PipelineCancel {
	#[argh(option, short = 'p')]
	/// the project to build
	pub project: Option<String>,
	#[argh(positional)]
	/// pipeline id
	pub id: Option<u64>,
}

#[derive(FromArgs)]
/// retry a pipeline
#[argh(subcommand, name = "retry")]
pub struct PipelineRetry {
	#[argh(option, short = 'p')]
	/// the project to build
	pub project: Option<String>,
	#[argh(positional)]
	/// pipeline id
	pub id: Option<u64>,
}

#[derive(FromArgs)]
/// manage project pipeline
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
