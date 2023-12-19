use crate::{
	api::users::keys::{AddKey, DeleteKey, ListKeys},
	args::{self, KeysCmd},
	context::CliContext,
	types::SshKey,
};

use anyhow::{Context, Result};
use gitlab::api::{self, Query};
use ssh_key::PublicKey;
use std::fs::read_to_string;

pub fn cmd(context: &CliContext, args: &args::Keys) -> Result<()> {
	match &args.cmd {
		KeysCmd::Add(args) => {
			let key = read_to_string(&args.key)?;
			let ssh_key = PublicKey::from_openssh(&key)?;
			let title = args
				.title
				.as_ref()
				.map(String::as_str)
				.unwrap_or_else(|| ssh_key.comment());
			let endpoint = AddKey::builder().key(&key).title(title).build()?;
			api::ignore(endpoint)
				.query(&context.gitlab)
				.with_context(|| format!("Failed to add ssh key {}", &args.key))?;
			println!("Key {} has been added", &ssh_key.comment());
			Ok(())
		}

		KeysCmd::List(args) => {
			let user = context.get_user(args.user.as_ref())?;
			let endpoint = ListKeys::builder().user(&user.username).build()?;
			let keys: Vec<SshKey> = endpoint.query(&context.gitlab)?;
			context.print_keys(&keys, &user)
		}

		KeysCmd::Delete(args) => {
			let endpoint = DeleteKey::builder().key_id(args.key_id).build()?;
			api::ignore(endpoint)
				.query(&context.gitlab)
				.with_context(|| format!("Failed to delete key {}", args.key_id))?;
			println!("Key {} deleted", args.key_id);
			Ok(())
		}
	}
}