use std::{borrow::Cow, str::FromStr};

use anyhow::{anyhow, Error};
use derive_builder::Builder;
use gitlab::api::{Endpoint, QueryParams};
use reqwest::Method;

#[derive(Debug, Clone, Copy)]
pub enum PersonalAccessTokenState {
	Active,
	Inactive,
}

impl PersonalAccessTokenState {
	pub(crate) fn as_str(&self) -> &'static str {
		match self {
			PersonalAccessTokenState::Active => "active",
			PersonalAccessTokenState::Inactive => "inactive",
		}
	}
}

impl FromStr for PersonalAccessTokenState {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if s == "active" {
			Ok(PersonalAccessTokenState::Active)
		} else if s == "inactive" {
			Ok(PersonalAccessTokenState::Inactive)
		} else {
			Err(anyhow!(
				"Unsupported token state value (active or inactive): {}",
				s
			))
		}
	}
}

/// Get a list of personal access tokens
#[derive(Debug, Builder)]
pub struct PersonalAccessTokens<'a> {
	/// The user to get the personal access tokens from
	#[builder(default)]
	user_id: u64,
	#[builder(default)]
	revoked: Option<bool>,
	#[builder(default)]
	state: Option<PersonalAccessTokenState>,
	#[builder(default)]
	search: Option<&'a str>,
}

impl<'a> PersonalAccessTokens<'a> {
	/// Create a builder for the endpoint.
	pub fn builder() -> PersonalAccessTokensBuilder<'a> {
		PersonalAccessTokensBuilder::default()
	}
}

impl<'a> Endpoint for PersonalAccessTokens<'a> {
	fn method(&self) -> Method {
		Method::GET
	}

	fn endpoint(&self) -> Cow<'static, str> {
		"personal_access_tokens".into()
	}

	fn parameters(&self) -> QueryParams {
		let mut params = QueryParams::default();
		params.push("user_id", self.user_id);
		params.push_opt("revoked", self.revoked);
		params.push_opt("search", self.search);
		params.push_opt(
			"state",
			self.state.as_ref().map(PersonalAccessTokenState::as_str),
		);

		params
	}
}
