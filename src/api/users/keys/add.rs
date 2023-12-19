use crate::types::keys::KeyUsage;

use chrono::{DateTime, Utc};
use derive_builder::Builder;
use gitlab::api::{Endpoint, QueryParams};
use reqwest::Method;
use std::borrow::Cow;

#[derive(Debug, Builder)]
pub struct AddKey<'a> {
	pub title: &'a str,
	pub key: &'a str,
	#[builder(default)]
	pub expires_at: Option<DateTime<Utc>>,
	#[builder(default)]
	pub usage_type: Option<KeyUsage>,
}

impl<'a> AddKey<'a> {
	/// Create a builder for the endpoint.
	pub fn builder() -> AddKeyBuilder<'a> {
		AddKeyBuilder::default()
	}
}

impl<'a> Endpoint for AddKey<'a> {
	fn method(&self) -> Method {
		Method::POST
	}

	fn endpoint(&self) -> Cow<'static, str> {
		"user/keys".into()
	}

	fn parameters(&self) -> QueryParams {
		let mut params = QueryParams::default();
		params.push("title", self.title);
		params.push("key", self.key);
		params.push_opt("expires_at", self.expires_at);
		params.push_opt("usage_type", self.usage_type);
		params
	}
}
