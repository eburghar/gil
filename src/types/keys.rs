use std::{borrow::Cow, str::FromStr};

use anyhow::{anyhow, Error};
use chrono::{DateTime, Utc};
use gitlab::{api::ParamValue, SshKeyId, UserBasic};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone)]
pub enum KeyUsage {
	Auth,
	Signing,
	AuthAndSigning,
}

impl KeyUsage {
	/// The scope as a query parameter.
	pub(crate) fn as_str(self) -> &'static str {
		match self {
			Self::Auth => "auth",
			Self::Signing => "signing",
			Self::AuthAndSigning => "auth_and_signing",
		}
	}
}

impl ParamValue<'static> for KeyUsage {
	fn as_value(&self) -> Cow<'static, str> {
		self.as_str().into()
	}
}

impl FromStr for KeyUsage {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if s == "auth" {
			Ok(Self::Auth)
		} else if s == "signing" {
			Ok(Self::Signing)
		} else if s == "auth_and_signing" {
			Ok(Self::AuthAndSigning)
		} else {
			Err(anyhow!(
				"Usage types are \"auth\", \"signing\" or \"auth_and_signing\" not {}",
				s
			))
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SshKey {
	pub id: SshKeyId,
	pub title: String,
	pub key: String,
	pub created_at: DateTime<Utc>,
	pub user: Option<UserBasic>,
}
