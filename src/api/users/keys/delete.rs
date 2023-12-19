use derive_builder::Builder;
use gitlab::api::Endpoint;
use reqwest::Method;
use std::borrow::Cow;

#[derive(Debug, Builder)]
pub struct DeleteKey {
	pub key_id: u64,
}

impl DeleteKey {
	/// Create a builder for the endpoint.
	pub fn builder() -> DeleteKeyBuilder {
		DeleteKeyBuilder::default()
	}
}

impl Endpoint for DeleteKey {
	fn method(&self) -> Method {
		Method::DELETE
	}

	fn endpoint(&self) -> Cow<'static, str> {
		format!("user/keys/{}", self.key_id).into()
	}
}
