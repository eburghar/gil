use std::borrow::Cow;

use derive_builder::Builder;
use gitlab::api::{endpoint_prelude::Method, Endpoint};

#[derive(Debug, Builder)]
pub struct RevokePersonalAccessToken {
	/// The token_id to revoke
	token_id: u64,
}

impl RevokePersonalAccessToken {
	/// Create a builder for the endpoint.
	pub fn builder() -> RevokePersonalAccessTokenBuilder {
		RevokePersonalAccessTokenBuilder::default()
	}
}

impl Endpoint for RevokePersonalAccessToken {
	fn method(&self) -> Method {
		Method::DELETE
	}

	fn endpoint(&self) -> Cow<'static, str> {
		format!("personal_access_tokens/{}", self.token_id).into()
	}
}
