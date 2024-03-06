use crate::{
	api::{
		personal_access_tokens::{
			PersonalAccessTokenState, PersonalAccessTokens, RevokePersonalAccessToken,
			RotatePersonalAccessToken,
		},
		users::personal_access_tokens::CreatePersonalAccessToken,
	},
	args,
	args::TokenCmd,
	context::CliContext,
	types::token::PersonalAccessToken,
};

use anyhow::{bail, Context, Result};
use gitlab::api::{self, Query};

pub fn cmd(args: &args::Token) -> Result<()> {
	match &args.cmd {
		TokenCmd::Create(args) => {
			// try to revoke a token with the the same name
			if args.revoke {
				if let Ok(token) = CliContext::global().get_token(args.name.as_ref()) {
					let endpoint = RevokePersonalAccessToken::builder()
						.token_id(token.id)
						.build()?;
					api::ignore(endpoint)
						.query(&CliContext::global().gitlab)
						.with_context(|| {
							format!("Failed to revoke existing token {}", args.name)
						})?;
				}
			}

			// try to create the requested token
			let user = CliContext::global().get_user(args.username.as_deref())?;
			let endpoint = CreatePersonalAccessToken::builder()
				.user_id(user.id.value())
				.name(args.name.as_ref())
				.expires_at(args.expires_at)
				.scopes(args.scopes.iter())
				.build()?;
			let token: PersonalAccessToken = endpoint.query(&CliContext::global().gitlab)?;
			if let Some(token) = token.token {
				println!("{}", token);
			} else {
				bail!("Token not found in response");
			}
		}

		TokenCmd::Revoke(args) => {
			let token = CliContext::global().get_token(args.name.as_ref())?;
			let endpoint = RevokePersonalAccessToken::builder()
				.token_id(token.id)
				.build()?;
			api::ignore(endpoint).query(&CliContext::global().gitlab)?;
			println!("token {}({}) has been revoked", args.name, token.id);
		}

		TokenCmd::List(args) => {
			let user = CliContext::global().get_user(args.username.as_deref())?;
			let mut builder = PersonalAccessTokens::builder();
			builder
				.user_id(user.id.value())
				.search(args.search.as_deref());
			if args.revoked {
				builder.revoked(Some(true));
			}
			if !args.revoked && !args.all {
				builder.state(Some(PersonalAccessTokenState::Active));
			}
			let endpoint = builder.build()?;
			let tokens: Vec<PersonalAccessToken> = endpoint.query(&CliContext::global().gitlab)?;
			if tokens.is_empty() {
				bail!("No token found matching criterias");
			}
			CliContext::global().print_tokens(&tokens, &user)?;
		}

		TokenCmd::Rotate(args) => {
			let token = CliContext::global().get_token(args.name.as_ref())?;
			let endpoint = RotatePersonalAccessToken::builder()
				.token_id(token.id)
				.expires_at(args.expires_at)
				.build()?;
			let token: PersonalAccessToken = endpoint.query(&CliContext::global().gitlab)?;
			if let Some(token) = token.token {
				println!("{}", token);
			} else {
				bail!("Token not found in response");
			}
		}
	}

	if CliContext::global().open {
		let _ = open::that(format!(
			"https://{}/-/profile/personal_access_tokens",
			CliContext::global().repo.host
		));
	}

	Ok(())
}
