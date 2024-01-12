use anyhow::{anyhow, Context, Result};
use git_repository::{commit::describe::SelectRef, discover, remote::Direction};
use semver::Version;
use std::env::current_dir;

#[derive(Debug)]
pub struct GitProject {
	/// project path
	pub name: Option<String>,
	/// remote host
	pub host: String,
	/// remote branch
	pub branch: String,
	/// tag
	pub tag: Option<String>,
	// commit
	pub commit: String,
}

impl GitProject {
	pub fn from_currentdir() -> Result<Self> {
		let repo = current_dir()
			.with_context(|| "Unable to get the current directory")
			.and_then(|dir| discover(dir).with_context(|| "Not inside a git repository"))?;

		// get the head id
		let commit = repo
			.head_id()
			.map(|id| id.to_hex().to_string())
			.with_context(|| "Unable to get the current commit")?;
		// get the local branch name
		let branch = repo
			.head_name()
			.with_context(|| "Unable to get repo's HEAD")?
			.map(|head| head.shorten().to_string())
			.ok_or_else(|| anyhow!("Unable to get repo's HEAD"))?;
		// find the remote associated to the branch
		let remote = repo
			.branch_remote_name(&branch)
			.map(|branch_remote| branch_remote.to_string())
			.ok_or_else(|| anyhow!("Unable to get remote tracking for branch"))
			.and_then(|branch_remote| {
				repo.find_remote(&branch_remote)
					.with_context(|| format!("No remote found with name {}", &branch_remote))
			})?;

		// get the host form the remote url
		let host = remote
			.url(Direction::Push)
			.and_then(|url| url.host().map(str::to_owned))
			.with_context(|| format!("Unable to get hostname for the remote {:?}", &remote))?;

		// try to get the project name from the remote url
		let name = remote
			.url(Direction::Push)
			.map(|url| url.path.to_string())
			.as_ref()
			// strip the leading / and the .git prefix
			.and_then(|path| {
				path.strip_prefix('/')
					.or(Some(path))
					.and_then(|path| path.strip_suffix(".git").or(Some(path)))
			})
			.map(str::to_owned);

		// try to get the greatest semver tag that is pointing to the head commit
		let head_commit = repo.head_commit().unwrap();
		let tag = repo
			// get iterator for all references
			.references()
			.ok()
			.and_then(|platform| {
				// browse all repo tags
				platform.tags().ok().map(|tags| {
					// an filter the ones
					tags.filter_map(|r| {
						r.ok()
							// that are pointing to head commit
							.and_then(|r| {
								(r.id() == head_commit.id)
									.then_some(r.name().file_name().to_string())
							})
							// and that can be parsed to semver
							.and_then(|tag| Version::parse(&tag).ok())
					})
					.collect::<Vec<Version>>()
				})
			})
			// get the latest semver tag
			.and_then(|mut tags| {
				tags.sort();
				tags.pop().map(|version| version.to_string())
			});

		// if this is not working then get the latest tag with describe
		let tag = tag.or_else(|| {
			repo.head_commit().ok().and_then(|commit| {
				commit
					.describe()
					.names(SelectRef::AllTags)
					.max_candidates(1)
					.traverse_first_parent(true)
					.try_format()
					.ok()
					.flatten()
					.and_then(|format| format.name)
					.map(|name| name.to_string())
			})
		});

		Ok(GitProject {
			name,
			host,
			branch,
			tag,
			commit,
		})
	}
}
