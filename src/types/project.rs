use std::fmt::{self, Display, Formatter};

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct ProjectId(u64);

impl ProjectId {
	/// The value of the id.
	pub const fn value(&self) -> u64 {
		self.0
	}
}

impl Display for ProjectId {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}

/// Project information.
#[derive(Deserialize, Debug, Clone)]
pub struct Project {
	/// The ID of the project.
	pub id: ProjectId,
	/// The display name of the project.
	pub name: String,
	/// The URL for the project's homepage.
	pub web_url: String,
	/// The display name of the project with the namespace.
	pub name_with_namespace: String,
	/// The path to the project's repository with its namespace.
	pub path_with_namespace: String,
}
