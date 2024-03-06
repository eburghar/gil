use crate::types::token::PersonalAccessTokenScope;

use chrono::NaiveDate;
use derive_builder::Builder;
use gitlab::api::{common::NameOrId, Endpoint, QueryParams};
use reqwest::Method;
use std::{borrow::Cow, collections::HashSet};

#[derive(Debug, Builder)]
pub struct CreatePersonalAccessToken<'a> {
    /// the user which owns the token
    user_id: u64,
    /// The token name to create
    name: NameOrId<'a>,
    /// date of expiration
    #[builder(default)]
    expires_at: Option<NaiveDate>,
    /// The scopes of the token to create
    #[builder(setter(name = "_scopes"), default, private)]
    scopes: HashSet<PersonalAccessTokenScope>,
}

impl<'a> CreatePersonalAccessTokenBuilder<'a> {
    /// implement scopes method to extend scopes instead of setting
    pub fn scopes<I>(&mut self, scopes: I) -> &mut Self
    where
        I: Iterator<Item = &'a PersonalAccessTokenScope>,
    {
        self.scopes.get_or_insert_with(HashSet::new).extend(scopes);
        self
    }
}

impl<'a> CreatePersonalAccessToken<'a> {
    /// Create a builder for the endpoint.
    pub fn builder() -> CreatePersonalAccessTokenBuilder<'a> {
        CreatePersonalAccessTokenBuilder::default()
    }
}

impl<'a> Endpoint for CreatePersonalAccessToken<'a> {
    fn method(&self) -> Method {
        Method::POST
    }

    fn endpoint(&self) -> Cow<'static, str> {
        format!("users/{}/personal_access_tokens", self.user_id).into()
    }

    fn parameters(&self) -> QueryParams {
        let mut params = QueryParams::default();
        params.push("user_id", self.user_id);
        params.push("name", self.name.to_string());
        params.push_opt("expires_at", self.expires_at);
        params.extend(self.scopes.iter().map(|&value| ("scopes[]", value)));

        params
    }
}
