use crate::{
	api::users::keys::{AddKey, DeleteKey, ListKeys},
	args::{self, KeyIdType, KeysCmd},
	context::CliContext,
	types::SshKey,
};

use anyhow::{Context, Result};
use gitlab::api::{self, Query};
use ssh_key::{HashAlg, PublicKey};
use std::fs::read_to_string;

pub fn cmd(context: &CliContext, args: &args::Keys) -> Result<()> {
	match &args.cmd {
		KeysCmd::Add(args) => {
			// read ssh key on disk
			let key = read_to_string(&args.key)?;
			let ssh_key = PublicKey::from_openssh(&key)?;
			let fingerprint = ssh_key.fingerprint(HashAlg::Sha256);

			// get title from args or key
			let title = args
				.title
				.as_ref()
				.map(String::as_str)
				.unwrap_or_else(|| ssh_key.comment());

			// try to delete existing key with same fingerprint on overwrite mode
			if args.overwrite {
				if let Ok(key) = context.get_key(&KeyIdType::FingerPrint(fingerprint)) {
					let endpoint = DeleteKey::builder().key_id(key.id.value()).build()?;
					api::ignore(endpoint).query(&context.gitlab)?;
				}
			}

			// try to add the key
			let endpoint = AddKey::builder().key(&key).title(title).build()?;
			api::ignore(endpoint)
				.query(&context.gitlab)
				.with_context(|| format!("Failed to add ssh key {}", &args.key))?;
			println!(
				"Key {} has been {}",
				&title,
				if args.overwrite {
					"overwritten"
				} else {
					"added"
				}
			);

			if context.open {
				let url = if let Ok(key) = context.get_key(&KeyIdType::FingerPrint(fingerprint)) {
					format!(
						"https://{}/-/profile/keys/{}",
						context.config.host.name,
						key.id.value()
					)
				} else {
					format!("https://{}/-/profile/keys", context.config.host.name)
				};
				let _ = open::that(url);
			}

			Ok(())
		}

		KeysCmd::List(args) => {
			let user = context.get_user(args.user.as_ref())?;
			let endpoint = ListKeys::builder().user(&user.username).build()?;
			let keys: Vec<SshKey> = endpoint.query(&context.gitlab)?;

			if context.open {
				let _ = open::that(format!(
					"https://{}/-/profile/keys",
					context.config.host.name
				));
			}

			context.print_keys(&keys, &user)
		}

		KeysCmd::Delete(args) => {
			let key = context.get_key(&args.id)?;
			let endpoint = DeleteKey::builder().key_id(key.id.value()).build()?;
			api::ignore(endpoint)
				.query(&context.gitlab)
				.with_context(|| format!("Failed to delete key {}", args.id))?;
			if let KeyIdType::Id(id) = args.id {
				println!("Key {} deleted", id);
			} else {
				println!("Key {}({}) deleted", args.id, key.id.value());
			}

			if context.open {
				let _ = open::that(format!(
					"https://{}/-/profile/keys",
					context.config.host.name
				));
			}

			Ok(())
		}
	}
}
