use crate::{
    args::Opts,
    config::{OAuth2, OAuth2Token},
};

use anyhow::{anyhow, bail, Context, Result};
use indoc::formatdoc;
use openidconnect::reqwest::Error;
use openidconnect::url::Url;
use openidconnect::{
    core::{CoreClient, CoreIdTokenVerifier, CoreProviderMetadata, CoreResponseType},
    AdditionalClaims, AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    HttpRequest, HttpResponse, IssuerUrl, Nonce, OAuth2TokenResponse, RedirectUrl, Scope,
};
use reqwest::{blocking, Certificate};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
};

#[derive(Debug, Deserialize, Serialize)]
struct GitLabClaims {
    // Deprecated and thus optional as it might be removed in the future
    sub_legacy: Option<String>,
    groups: Vec<String>,
}
impl AdditionalClaims for GitLabClaims {}

struct HttpClient {
    ca: Option<Certificate>,
}

impl HttpClient {
    pub fn try_new(ca: &Option<String>) -> Result<Self, anyhow::Error> {
        let ca = if let Some(ca) = ca {
            let mut buf = Vec::new();
            File::open(ca)
                .with_context(|| format!("Error opening {}", ca))?
                .read_to_end(&mut buf)
                .with_context(|| format!("Error reading {}", ca))?;
            Some(
                reqwest::Certificate::from_pem(&buf)
                    .map_err(Error::Reqwest)
                    .with_context(|| format!("Reading certificate {}", ca))?,
            )
        } else {
            None
        };
        Ok(HttpClient { ca })
    }

    pub fn http_client(
        self,
    ) -> impl Fn(HttpRequest) -> Result<HttpResponse, Error<reqwest::Error>> {
        move |request: HttpRequest| {
            let mut builder = blocking::Client::builder()
                // Following redirects opens the client up to SSRF vulnerabilities.
                .redirect(reqwest::redirect::Policy::none());
            builder = if let Some(cert) = &self.ca {
                builder.add_root_certificate(cert.to_owned())
            } else {
                builder
            };
            let client = builder.build().map_err(Error::Reqwest)?;
            let mut request_builder = client
                .request(request.method, request.url.as_str())
                .body(request.body);

            for (name, value) in &request.headers {
                request_builder = request_builder.header(name.as_str(), value.as_bytes());
            }
            let mut response = client
                .execute(request_builder.build().map_err(Error::Reqwest)?)
                .map_err(Error::Reqwest)?;

            let mut body = Vec::new();
            response.read_to_end(&mut body).map_err(Error::Io)?;
            Ok(HttpResponse {
                status_code: response.status(),
                headers: response.headers().to_owned(),
                body,
            })
        }
    }
}

// Try to login to gitlab using oidc
// save the token to cache file and return the login information in case of success
pub fn login(host: &str, ca: &Option<String>, config: &OAuth2, opts: &Opts) -> Result<OAuth2Token> {
    let gitlab_client_id = ClientId::new(config.id.to_string());
    let gitlab_client_secret = ClientSecret::new(config.secret.to_string());
    let issuer_url =
        IssuerUrl::new(format!("https://{}", host)).with_context(|| "Invalid issuer URL")?;
    let http_client = HttpClient::try_new(ca)?.http_client();

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
                let (key, _) = pair;
                key == "code"
            })
            .unwrap();

        let (_, value) = code_pair;
        code = AuthorizationCode::new(value.into_owned());

        let state_pair = url
            .query_pairs()
            .find(|pair| {
                let (key, _) = pair;
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
				<style>
					body {{
						background-color: #eee;
						margin: 0;
						padding: 0;
						font-family: sans-serif;
					}}
					.placeholder {{
						margin: 2em;
						padding: 2em;
						background-color: #fff;
						border-radius: 1em;
					}}
				</style>
			</head>
			<body>
				<div class=\"placeholder\">
					<h1>Authenticated</h1>
					<p>{name} {version} authenticated successfully. You can close this window.</p>
					<script>
						window.close();
					</script>
				</div>
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

    let http_client = HttpClient::try_new(ca)?.http_client();
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
    let cache = OAuth2Token::new(token_response);
    if !opts.no_cache {
        let _ = cache.save(host);
    }

    Ok(cache)
}
