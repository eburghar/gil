use git_repository::{commit::describe::SelectRef, discover, remote::Direction};
use std::env::current_dir;

#[derive(Debug)]
pub struct GitProject {
	/// project path
	pub name: Option<String>,
	/// remote host
	pub host: Option<String>,
	/// remote branch
	pub branch: Option<String>,
	/// tag
	pub tag: Option<String>,
	// commit
	pub commit: Option<String>,
}

impl GitProject {
	pub fn from_currentdir() -> Option<Self> {
		if let Some(repo) = current_dir().ok().and_then(|dir| discover(dir).ok()) {
			// get the head id
			let commit = repo.head_id().ok().map(|id| id.to_hex().to_string());
			// get the local branch name
			let branch = repo
				.head_name()
				.ok()
				.flatten()
				.map(|head| head.shorten().to_string());
			// find the remote associated to the remote branch
			let remote = branch
				.as_ref()
				.and_then(|branch| repo.branch_remote_name(branch))
				.map(|branch_remote| branch_remote.to_string())
				.and_then(|branch_remote| repo.find_remote(&branch_remote).ok());

			// get the host form the remote url
			let host = remote
				.as_ref()
				.and_then(|remote| remote.url(Direction::Push))
				.and_then(|url| url.host().map(|host| host.to_owned()));

			// get the project name from the remote url
			let name = remote
				.as_ref()
				.and_then(|remote| remote.url(Direction::Push))
				.map(|url| url.path.to_string())
				.as_ref()
				// strip the leading / and the .git prefix
				.and_then(|path| {
					path.strip_prefix('/')
						.or(Some(path))
						.and_then(|path| path.strip_suffix(".git").or(Some(path)))
				})
				.map(|path| path.to_owned());

			// get the first tag by traversing the git tree from current branch
			let tag = repo.head_commit().ok().and_then(|commit| {
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
			});

			Some(GitProject {
				name,
				host,
				branch,
				tag,
				commit,
			})
		} else {
			None
		}
	}
}
