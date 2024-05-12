use derive_builder::Builder;
use gitlab::api::endpoint_prelude::Method;
use gitlab::api::{Endpoint, QueryParams};
use std::borrow::Cow;

#[derive(Debug, Builder)]
pub struct GetKey<'a> {
	/// The key's fingerprint to search for
	pub fingerprint: &'a String,
}

impl<'a> GetKey<'a> {
	/// Create a builder for the endpoint.
	pub fn builder() -> GetKeyBuilder<'a> {
		GetKeyBuilder::default()
	}
}

impl<'a> Endpoint for GetKey<'a> {
	fn method(&self) -> Method {
		Method::GET
	}

	fn endpoint(&self) -> Cow<'static, str> {
		"keys".into()
	}

	fn parameters(&self) -> QueryParams {
		let mut params = QueryParams::default();
		params.push("fingerprint", self.fingerprint);

		params
	}
}
