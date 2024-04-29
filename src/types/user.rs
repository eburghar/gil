use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct UserId(u64);

impl UserId {
	/// The value of the id.
	pub const fn value(&self) -> u64 {
		self.0
	}
}

#[derive(Deserialize, Debug, Clone)]
pub struct User {
	/// The username.
	pub username: String,
	/// The display name.
	// pub name: String,
	/// The user's ID.
	pub id: UserId,
	/// The URL of the user's profile page.
	pub web_url: String,
	/// Only available when talking to GitLab as an admin.
	pub is_admin: Option<bool>,
}
