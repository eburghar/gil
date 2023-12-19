use derive_builder::Builder;
use gitlab::api::Endpoint;
use reqwest::Method;
use std::borrow::Cow;

#[derive(Debug, Builder)]
pub struct ListKeys<'a> {
    /// The user to list the keys from
    pub user: &'a String,
}

impl<'a> ListKeys<'a> {
    /// Create a builder for the endpoint.
    pub fn builder() -> ListKeysBuilder<'a> {
        ListKeysBuilder::default()
    }
}

impl<'a> Endpoint for ListKeys<'a> {
    fn method(&self) -> Method {
        Method::GET
    }

    fn endpoint(&self) -> Cow<'static, str> {
        format!("users/{}/keys", self.user).into()
    }
}
