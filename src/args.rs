use anyhow::Error;
#[cfg(feature = "color")]
use argh::FromArgValue;
use argh::{FromArgs, TopLevelCommand};
use chrono::NaiveDate;
use gitlab::api::common::NameOrId;
use ssh_key::Fingerprint;
use std::{env, fmt::Display, path::Path, str::FromStr};

use crate::types::{keys::KeyUsage, token::PersonalAccessTokenScope};

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
    Branches(Branches),
    Pipeline(Pipeline),
    Archive(Archive),
    Project(Project),
    Token(Token),
    Keys(Keys),
    Users(Users),
}

/// Get and extract archives
#[derive(FromArgs)]
#[argh(subcommand, name = "extract")]
pub struct ArchiveExtract {
    /// the project to extract archive from
    #[argh(option, short = 'p')]
    pub project: Option<OwnedNameOrId>,

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
    /// the project to protect tags on
    #[argh(option, short = 'p')]
    pub project: Option<OwnedNameOrId>,

    /// tag expression: '*' (default)
    #[argh(positional, default = "\"*\".to_string()")]
    pub tag: String,
}

/// Unprotect a project tag(s)
#[derive(FromArgs)]
#[argh(subcommand, name = "unprotect")]
pub struct TagsUnprotect {
    /// the project to protect tags on
    #[argh(option, short = 'p')]
    pub project: Option<OwnedNameOrId>,

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

/// Protect a project branche(s)
#[derive(FromArgs)]
#[argh(subcommand, name = "protect")]
pub struct BranchesProtect {
    /// the project to protect branches on
    #[argh(option, short = 'p')]
    pub project: Option<OwnedNameOrId>,

    /// allow force push
    #[argh(switch, short = 'f')]
    pub force_push: bool,

    /// branch expression
    #[argh(positional)]
    pub branch: Option<String>,
}

/// Unprotect a project brnache(s)
#[derive(FromArgs)]
#[argh(subcommand, name = "unprotect")]
pub struct BranchesUnprotect {
    /// the project to protect tags from
    #[argh(option, short = 'p')]
    pub project: Option<OwnedNameOrId>,

    /// branch expression
    #[argh(positional)]
    pub branch: Option<String>,
}

/// Manage project branches
#[derive(FromArgs)]
#[argh(subcommand, name = "branches")]
pub struct Branches {
    /// operate on branches
    #[argh(subcommand)]
    pub cmd: BranchesCmd,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum BranchesCmd {
    Protect(BranchesProtect),
    Unprotect(BranchesUnprotect),
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum PipelineCmd {
    List(PipelineList),
    Status(PipelineStatus),
    Create(PipelineCreate),
    Cancel(PipelineCancel),
    Retry(PipelineRetry),
    Log(PipelineLog),
}

/// list pipelines
#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
pub struct PipelineList {
    /// the project which owns the pipeline
    #[argh(option, short = 'p')]
    pub project: Option<OwnedNameOrId>,

    /// limit
    #[argh(option, short = 'l', default = "10")]
    pub limit: usize,

    /// pipeline id
    #[argh(positional)]
    pub id: Option<u64>,
}

/// Get pipeline status
#[derive(FromArgs)]
#[argh(subcommand, name = "status")]
pub struct PipelineStatus {
    /// the project which owns the pipeline
    #[argh(option, short = 'p')]
    pub project: Option<OwnedNameOrId>,

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
    pub project: Option<OwnedNameOrId>,

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
    pub project: Option<OwnedNameOrId>,

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
    pub project: Option<OwnedNameOrId>,

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
    pub project: Option<OwnedNameOrId>,

    /// reference (tag or branch)
    #[argh(option, short = 'r')]
    pub ref_: Option<String>,

    /// a name that partially match the section name(s) to show in the log: step_script (default)
    #[argh(option, short = 's', default = "\"step_script\".to_string()")]
    pub section: String,

    /// the job id to extract the job log from
    #[argh(option, short = 'j')]
    pub job_id: Option<u64>,

    /// show all sections
    #[argh(switch, short = 'a')]
    pub all: bool,

    /// show section headers
    #[argh(switch, short = 'h')]
    pub headers: bool,

    /// show only section headers (all collapsed)
    #[argh(switch, short = 'H')]
    pub only_headers: bool,

    /// the pipeline id
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
    pub project: Option<OwnedNameOrId>,

    /// reference (tag or branch)
    #[argh(positional)]
    pub ref_: Option<String>,
}

/// Manage user tokens
#[derive(FromArgs)]
#[argh(subcommand, name = "token")]
pub struct Token {
    #[argh(subcommand)]
    /// operate on tokens
    pub cmd: TokenCmd,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum TokenCmd {
    List(TokenList),
    Create(TokenCreate),
    Revoke(TokenRevoke),
    Rotate(TokenRotate),
}

/// Owned version of gitlab::api::common::NameOrId
pub enum OwnedNameOrId {
    Name(String),
    Id(u64),
}

/// Construct an OwnedNameOrId from a String.
// Try to parse an u64 and fallback to string
impl FromStr for OwnedNameOrId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if let Ok(id) = s.parse::<u64>() {
            Self::Id(id)
        } else {
            Self::Name(s.to_owned())
        })
    }
}

impl Display for OwnedNameOrId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Id(id) => write!(f, "{}", id),
            Self::Name(name) => write!(f, "{}", name),
        }
    }
}

/// Convert a borrowed OwnedNameOrId to a NameOrId
/// Auto-implement into()
impl<'a> From<&'a OwnedNameOrId> for NameOrId<'a> {
    fn from(value: &'a OwnedNameOrId) -> Self {
        match value {
            OwnedNameOrId::Name(name) => NameOrId::Name(name.into()),
            OwnedNameOrId::Id(id) => NameOrId::Id(*id),
        }
    }
}

impl OwnedNameOrId {
    /// Like Option::as_ref but for OwnedNameOrId.
    /// Returns a NameOrId which is a kind of borrowed version of OwnedNameOrId
    /// It is more readable to call .as_ref than NameOrId::From(&name_or_id)
    pub fn as_ref(&self) -> NameOrId {
        self.into()
    }
}

/// Create a new token
#[derive(FromArgs)]
#[argh(subcommand, name = "create")]
pub struct TokenCreate {
    /// the user which owns the token
    #[argh(option, short = 'u')]
    pub username: Option<String>,

    /// the token scopes
    #[argh(option, short = 's')]
    pub scopes: Vec<PersonalAccessTokenScope>,

    /// the expiration date
    #[argh(option, short = 'e')]
    pub expires_at: Option<NaiveDate>,

    /// revoke a token with same name if it exists
    #[argh(switch, short = 'r')]
    pub revoke: bool,

    /// the token name
    #[argh(positional)]
    pub name: OwnedNameOrId,
}

/// Delete a token
#[derive(FromArgs)]
#[argh(subcommand, name = "revoke")]
pub struct TokenRevoke {
    /// the user which owns the token
    #[argh(option, short = 'u')]
    pub username: Option<String>,

    /// the token name
    #[argh(positional)]
    pub name: OwnedNameOrId,
}

/// List tokens
#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
pub struct TokenList {
    /// the username to list the tokens that belong to (only for admin and by default current user)
    #[argh(option, short = 'u')]
    pub username: Option<String>,

    /// list revoked tokens (implies --all)
    #[argh(switch, short = 'r')]
    pub revoked: bool,

    /// list all tokens (only active ones per default)
    #[argh(switch, short = 'a')]
    pub all: bool,

    /// the pattern of token to search names for
    #[argh(positional)]
    pub search: Option<String>,
}

/// Rotate token
#[derive(FromArgs)]
#[argh(subcommand, name = "rotate")]
pub struct TokenRotate {
    /// the user which owns the token
    #[argh(option, short = 'u')]
    pub username: Option<String>,

    /// the expiration date
    #[argh(option, short = 'e')]
    pub expires_at: Option<NaiveDate>,

    /// the token name
    #[argh(positional)]
    pub name: OwnedNameOrId,
}

/// Manage user keys
#[derive(FromArgs)]
#[argh(subcommand, name = "keys")]
pub struct Keys {
    #[argh(subcommand)]
    /// operate on keys
    pub cmd: KeysCmd,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum KeysCmd {
    List(ListKeys),
    Add(AddKey),
    Delete(DeleteKey),
}

/// Add a key
#[derive(FromArgs)]
#[argh(subcommand, name = "add")]
pub struct AddKey {
    /// title of the key
    #[argh(option, short = 't')]
    pub title: Option<String>,

    /// expiration date of the key
    #[argh(option, short = 'e')]
    pub expires_at: Option<NaiveDate>,

    /// usage type for the key
    #[argh(option, short = 'u')]
    pub usage_type: Option<KeyUsage>,

    /// overwrite a key with the same title
    #[argh(switch, short = 'w')]
    pub overwrite: bool,

    /// key path
    #[argh(positional)]
    pub key: String,
}

/// List keys
#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
pub struct ListKeys {
    /// username of user id to list the keys from
    #[argh(option, short = 'u')]
    pub user: Option<String>,
}

/// Identification of a key for the delete subcommand
pub enum KeyIdType {
    Id(u64),
    Name(String),
    FingerPrint(Fingerprint),
}

impl Display for KeyIdType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyIdType::Id(id) => write!(f, "Id({})", id),
            KeyIdType::Name(name) => write!(f, "\"{}\"", name),
            KeyIdType::FingerPrint(fingerprint) => write!(f, "{}", fingerprint),
        }
    }
}

/// Try to parse KeyId by fingerprint, id(u64) or fallback to name
impl FromStr for KeyIdType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if let Ok(fingerprint) = Fingerprint::from_str(s) {
            Self::FingerPrint(fingerprint)
        } else if let Ok(id) = s.parse::<u64>() {
            Self::Id(id)
        } else {
            Self::Name(s.to_owned())
        })
    }
}

/// Delete a key
#[derive(FromArgs)]
#[argh(subcommand, name = "delete")]
pub struct DeleteKey {
    /// the key id (db id, fingerprint or name) do delete
    #[argh(positional)]
    pub id: KeyIdType,
}

/// Manage users
#[derive(FromArgs)]
#[argh(subcommand, name = "users")]
pub struct Users {
    #[argh(subcommand)]
    /// operate on user
    pub cmd: UserCmd,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum UserCmd {
    Current(Current),
}

/// Get current user name
#[derive(FromArgs)]
#[argh(subcommand, name = "current")]
pub struct Current {}
