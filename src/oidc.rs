use crate::{
	args::Opts,
	config::{OAuth2, OAuth2Token},
};

use anyhow::{anyhow, bail, Context, Result};
use indoc::formatdoc;
use openidconnect::url::Url;
use openidconnect::{
	core::{CoreClient, CoreIdTokenVerifier, CoreProviderMetadata, CoreResponseType},
	reqwest::http_client,
	AdditionalClaims, AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
	IssuerUrl, Nonce, OAuth2TokenResponse, RedirectUrl, Scope,
};
use serde::{Deserialize, Serialize};
use std::{
	io::{BufRead, BufReader, Write},
	net::TcpListener,
};

#[derive(Debug, Deserialize, Serialize)]
struct GitLabClaims {
	// Deprecated and thus optional as it might be removed in the futre
	sub_legacy: Option<String>,
	groups: Vec<String>,
}
impl AdditionalClaims for GitLabClaims {}

// Try to login to gitlab using oidc
// save the token to cache file and return the login information in case of success
pub fn login(host: &String, config: &OAuth2, opts: &Opts) -> Result<OAuth2Token> {
	let gitlab_client_id = ClientId::new(config.id.to_string());
	let gitlab_client_secret = ClientSecret::new(config.secret.to_string());
	let issuer_url =
		IssuerUrl::new(format!("https://{}", host)).with_context(|| "Invalid issuer URL")?;

	// Fetch GitLab's OpenID Connect discovery document.
	let provider_metadata = CoreProviderMetadata::discover(&issuer_url, http_client)
		.with_context(|| "Failed to discover OpenID Provider")?;

	// Set up the config for the GitLab OAuth2 process.
	let client = CoreClient::from_provider_metadata(
		provider_metadata,
		gitlab_client_id,
		Some(gitlab_client_secret),
	)
	// set the redirect url to where we will be listening
	.set_redirect_uri(
		RedirectUrl::new(format!("http://localhost:{}", config.redirect_port))
			.with_context(|| "Invalid redirect URL")?,
	);

	// Generate the authorization URL to which we'll redirect the user.
	let (authorize_url, csrf_state, nonce) = client
		.authorize_url(
			AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
			CsrfToken::new_random,
			Nonce::new_random,
		)
		.add_scope(Scope::new("api".to_string()))
		.url();

	// ask the OS to open the url
	let url = authorize_url.to_string();
	if opts.verbose {
		println!("redirect to {}", &url)
	}
	open::that(url)?;

	// A very naive implementation of the redirect server.
	let listener = TcpListener::bind(format!("127.0.0.1:{}", config.redirect_port))
		.with_context(|| "Failed to listen to redirect url")?;

	// Accept one connection
	let (mut stream, _) = listener.accept()?;
	let code;
	let state;
	{
		let mut reader = BufReader::new(&stream);

		let mut request_line = String::new();
		reader.read_line(&mut request_line)?;

		let redirect_url = request_line.split_whitespace().nth(1).unwrap();
		let url = Url::parse(&("http://localhost".to_string() + redirect_url))?;

		let code_pair = url
			.query_pairs()
			.find(|pair| {
				let &(ref key, _) = pair;
				key == "code"
			})
			.unwrap();

		let (_, value) = code_pair;
		code = AuthorizationCode::new(value.into_owned());

		let state_pair = url
			.query_pairs()
			.find(|pair| {
				let &(ref key, _) = pair;
				key == "state"
			})
			.unwrap();

		let (_, value) = state_pair;
		state = CsrfToken::new(value.into_owned());
	}

	let page = formatdoc! {"
        <!DOCTYPE HTML>
        <html>
		<head>
		<title>{name}</title>
		</head>
        <body>
		  <h2>{name} {version} connexion successful</h2>
		  <p>You can close this window.</p>
          <script>
			window.close();
          </script>
        </body>
        </html>"
	,
	version = env!("CARGO_PKG_VERSION"),
	name = env!("CARGO_BIN_NAME") };
	let response = format!(
		"HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
		page.len(),
		page
	);
	stream.write_all(response.as_bytes())?;

	if state.secret() != csrf_state.secret() {
		bail!("CSRF test failed")
	}

	// Exchange the code with a token.
	let token_response = client
		.exchange_code(code)
		.request(http_client)
		.with_context(|| "Failed to contact token endpoint")?;

	let id_token_verifier: CoreIdTokenVerifier = client.id_token_verifier();
	// verify the claims
	token_response
		.extra_fields()
		.id_token()
		.ok_or_else(|| anyhow!("Server did not return an ID token"))?
		.claims(&id_token_verifier, &nonce)
		.with_context(|| "Failed to verify ID token")?;

	// save into cache
	let cache = OAuth2Token::new(token_response.access_token().secret().to_owned());
	if !opts.no_cache {
		let _ = cache.save();
	}

	Ok(cache)
}
