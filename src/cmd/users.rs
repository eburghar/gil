use crate::{api::user::User, args, context::CliContext, types};

use anyhow::Result;
use gitlab::api::Query;
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
	}
}
