use gitlab::api::Endpoint;
use reqwest::Method;

pub struct User;

impl User {
    pub fn build() -> Self {
        User {}
    }
}

impl Endpoint for User {
    fn method(&self) -> reqwest::Method {
        Method::GET
    }

    fn endpoint(&self) -> std::borrow::Cow<'static, str> {
        "user".into()
    }
}
