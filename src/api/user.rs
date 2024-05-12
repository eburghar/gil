use gitlab::api::{endpoint_prelude::Method, Endpoint};

pub struct User;

impl User {
	pub fn build() -> Self {
		User {}
	}
}

impl Endpoint for User {
	fn method(&self) -> Method {
		Method::GET
	}

	fn endpoint(&self) -> std::borrow::Cow<'static, str> {
		"user".into()
	}
}
