use crate::{
    api::user::User, args, config::OAuth2Token, context::CliContext, git::GitProject, types,
};

use anyhow::Result;
use gitlab::api::Query;
use openidconnect::IdToken;
use openidconnect::OAuth2TokenResponse;
use std::process::ExitCode;

pub fn cmd(args: &args::Users) -> Result<ExitCode> {
    match &args.cmd {
        args::UserCmd::Current(_) => {
            let endpoint = User::build();
            let user: types::User = endpoint.query(&CliContext::global().gitlab)?;
            CliContext::global().print_username(&user)
        }

        args::UserCmd::IsAdmin(_) => {
            let endpoint = User::build();
            let user: types::User = endpoint.query(&CliContext::global().gitlab)?;
            Ok(user
                .is_admin
                .map(|b| {
                    if b {
                        ExitCode::from(0)
                    } else {
                        ExitCode::from(1)
                    }
                })
                .unwrap_or(ExitCode::from(1)))
        }

        args::UserCmd::Token(_) => {
            // get information from git
            let repo = GitProject::from_currentdir()?;
            // println!("{}", &CliContext::global().repo.host);
            let jwt = OAuth2Token::from_cache(&repo.host);
            if let Some(jwt) = jwt {
                println!("token {}", jwt.access_token().secret());
                println!("{:#?}", jwt.extra_fields().id_token().unwrap());
            }
            Ok(ExitCode::from(0))
        }
    }
}
