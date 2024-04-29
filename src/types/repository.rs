use serde::Deserialize;

use super::ObjectId;

/// A commit in a project.
#[derive(Deserialize, Debug, Clone)]
pub struct RepoCommit {
	/// The ID of the commit.
	pub id: ObjectId,
	/// The short ID of the commit.
	pub short_id: ObjectId,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Tag {
	// Commit message
	pub commit: RepoCommit,
	// // Release tag
	// pub release: Option<ReleaseTag>,
	// Tag name
	pub name: String,
	// Target sha
	// pub target: String,
	// pub message: Option<String>,
	// // Is tag protected
	// pub protected: bool,
}

/// Reponse of a project variable
#[derive(Deserialize, Debug, Clone)]
pub struct ProtectedTag {
	/// The name or wildcard
	pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PRBAccessLevel {
	pub access_level: u64,
	pub access_level_description: String,
}

/// A protected branch on a repository
#[derive(Deserialize, Debug, Clone)]
pub struct ProtectedRepoBranch {
	pub name: String,
	pub push_access_levels: Vec<PRBAccessLevel>,
	pub merge_access_levels: Vec<PRBAccessLevel>,
	pub code_owner_approval_required: Option<bool>,
}

/// A branch on a repository.
#[derive(Deserialize, Debug, Clone)]
pub struct RepoBranch {
	/// The name of the branch.
	pub name: String,
}
