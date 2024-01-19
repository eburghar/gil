use crate::{api::user::User, args, context::CliContext};

use anyhow::Result;
use gitlab::{api::Query, UserBasic};

pub fn cmd(args: &args::Users) -> Result<()> {
    match &args.cmd {
        args::UserCmd::Current(_) => {
            let endpoint = User::build();
            let user: UserBasic = endpoint.query(&CliContext::global().gitlab)?;
            CliContext::global().print_username(&user)
        }
    }
}
