use std::borrow::Cow;

use chrono::NaiveDate;
use derive_builder::Builder;
use gitlab::api::{Endpoint, QueryParams};
use reqwest::Method;

#[derive(Debug, Builder)]
pub struct RotatePersonalAccessToken {
    /// The token_id to rotate
    token_id: u64,
    /// date of expiration
    #[builder(default)]
    expires_at: Option<NaiveDate>,
}

impl RotatePersonalAccessToken {
    /// Create a builder for the endpoint.
    pub fn builder() -> RotatePersonalAccessTokenBuilder {
        RotatePersonalAccessTokenBuilder::default()
    }
}

impl Endpoint for RotatePersonalAccessToken {
    fn method(&self) -> Method {
        Method::POST
    }

    fn endpoint(&self) -> Cow<'static, str> {
        format!("personal_access_tokens/{}/rotate", self.token_id).into()
    }

    fn parameters(&self) -> QueryParams {
        let mut params = QueryParams::default();
        params.push_opt("expires_at", self.expires_at);

        params
    }
}
