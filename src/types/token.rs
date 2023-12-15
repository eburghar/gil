use anyhow::{anyhow, Error};
use chrono::{DateTime, NaiveDate, Utc};
use gitlab::api::ParamValue;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, str::FromStr};

/// Scopes for personal access tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalAccessTokenScope {
	/// Access the API and perform git reads and writes.
	Api,
	/// Access to read the user information.
	ReadUser,
	/// Access to API read-only.
	ReadApi,
	/// Access to read private repository
	ReadRepository,
	/// Access to write private repository
	WriteRepository,
	/// Access to read container registry
	ReadRegistry,
	/// Access to write container registry
	WriteRegistry,
	/// Access to perform API actions as any user if authenticated as administrator
	Sudo,
	/// Access to perform API actions as administrator when Admin Mode is enabled
	AdminMode,
	/// Access to creation of runners
	CreateRunner,
	/// Access to API Action on Gitlab Duo
	AiFeatures,
	/// Access to k8s API call
	K8sFeatures,
}

// TODO: impl Display with as_str and use Display with serde
// impl<'de> Deserialize<'de> for PersonalAccessTokenScope {
// 	fn deserialize<D>(deserializer: D) -> Result<T, D::Error>
// 	where
// 		T: FromStr,
// 		T::Err: Display,
// 		D: Deserializer<'de>,
// 	{
// 		String::deserialize(deserializer)?
// 			.parse()
// 			.map_err(de::Error::custom)
// 	}
// }

impl PersonalAccessTokenScope {
	/// The scope as a query parameter.
	pub(crate) fn as_str(self) -> &'static str {
		match self {
			Self::Api => "api",
			Self::ReadUser => "read_user",
			Self::ReadApi => "read_api",
			Self::ReadRepository => "read_repository",
			Self::WriteRepository => "write_repository",
			Self::ReadRegistry => "read_registry",
			Self::WriteRegistry => "write_registry",
			Self::Sudo => "sudo",
			Self::AdminMode => "admin_mode",
			Self::CreateRunner => "create_runner",
			Self::AiFeatures => "ai_features",
			Self::K8sFeatures => "k8s_features",
		}
	}
}

impl ParamValue<'static> for PersonalAccessTokenScope {
	fn as_value(&self) -> Cow<'static, str> {
		self.as_str().into()
	}
}

// TODO: use fromstr with serde
impl FromStr for PersonalAccessTokenScope {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if s == "api" {
			Ok(Self::Api)
		} else if s == "read_user" {
			Ok(Self::ReadUser)
		} else if s == "read_api" {
			Ok(Self::ReadApi)
		} else if s == "read_repository" {
			Ok(Self::ReadRepository)
		} else if s == "write_repository" {
			Ok(Self::WriteRepository)
		} else if s == "read_registry" {
			Ok(Self::ReadRegistry)
		} else if s == "write_registry" {
			Ok(Self::WriteRegistry)
		} else if s == "sudo" {
			Ok(Self::Sudo)
		} else if s == "admin_mode" {
			Ok(Self::AdminMode)
		} else if s == "create_runner" {
			Ok(Self::CreateRunner)
		} else if s == "ai_features" {
			Ok(Self::AiFeatures)
		} else if s == "k8s_features" {
			Ok(Self::K8sFeatures)
		} else {
			Err(anyhow!("Unknown scope {}", s))
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PersonalAccessToken {
	pub id: u64,
	pub user_id: u64,
	pub name: String,
	pub scopes: Vec<PersonalAccessTokenScope>,
	pub active: bool,
	pub revoked: bool,
	pub created_at: DateTime<Utc>,
	pub last_used_at: Option<DateTime<Utc>>,
	pub expires_at: Option<NaiveDate>,
	pub token: Option<String>,
}

impl PersonalAccessToken {
	pub fn expired(&self) -> bool {
		if let Some(d) = self.expires_at {
			Utc::now().date_naive() > d
		} else {
			false
		}
	}
}
