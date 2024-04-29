pub mod keys;
pub mod pipeline;
pub mod project;
pub mod repository;
pub mod token;
pub mod user;

pub use keys::SshKey;
pub use pipeline::{Job, Pipeline, StatusState};
pub use project::Project;
pub use repository::{ProtectedRepoBranch, ProtectedTag, RepoBranch, Tag};
pub use token::PersonalAccessToken;
pub use user::User;

use serde::Deserialize;

/// The ID of a git object.
#[derive(Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct ObjectId(String);

impl ObjectId {
	/// The value of the id.
	pub fn value(&self) -> &String {
		&self.0
	}
}
