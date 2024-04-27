use crate::{
	api::users::keys::{AddKey, DeleteKey, ListKeys},
	args::{self, KeyIdType, KeysCmd},
	context::CliContext,
	types::SshKey,
};

use anyhow::{Context, Result};
use gitlab::api::{self, Query};
use ssh_key::{HashAlg, PublicKey};
use std::{fs::read_to_string, process::ExitCode};

pub fn cmd(args: &args::Keys) -> Result<ExitCode> {
	match &args.cmd {
		KeysCmd::Add(args) => {
			// read ssh key on disk
			let key = read_to_string(&args.key)?;
			let ssh_key = PublicKey::from_openssh(&key)?;
			let fingerprint = ssh_key.fingerprint(HashAlg::Sha256);

			// get title from args or key
			let title = args.title.as_deref().unwrap_or_else(|| ssh_key.comment());

			// try to delete existing key with same fingerprint on overwrite mode
			if args.overwrite {
				if let Ok(key) = CliContext::global().get_key(&KeyIdType::FingerPrint(fingerprint))
				{
					let endpoint = DeleteKey::builder().key_id(key.id.value()).build()?;
					api::ignore(endpoint).query(&CliContext::global().gitlab)?;
				}
			}

			// try to add the key
			let endpoint = AddKey::builder().key(&key).title(title).build()?;
			api::ignore(endpoint)
				.query(&CliContext::global().gitlab)
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

			if CliContext::global().open {
				let url = if let Ok(key) =
					CliContext::global().get_key(&KeyIdType::FingerPrint(fingerprint))
				{
					format!(
						"https://{}/-/profile/keys/{}",
						CliContext::global().repo.host,
						key.id.value()
					)
				} else {
					format!("https://{}/-/profile/keys", CliContext::global().repo.host)
				};
				let _ = open::that(url);
			}

			Ok(ExitCode::from(0))
		}

		KeysCmd::List(args) => {
			let user = CliContext::global().get_user(args.user.as_deref())?;
			let endpoint = ListKeys::builder().user(&user.username).build()?;
			let keys: Vec<SshKey> = endpoint.query(&CliContext::global().gitlab)?;

			if CliContext::global().open {
				let _ = open::that(format!(
					"https://{}/-/profile/keys",
					CliContext::global().repo.host
				));
			}

			CliContext::global().print_keys(&keys, &user)
		}

		KeysCmd::Delete(args) => {
			let key = CliContext::global().get_key(&args.id)?;
			let endpoint = DeleteKey::builder().key_id(key.id.value()).build()?;
			api::ignore(endpoint)
				.query(&CliContext::global().gitlab)
				.with_context(|| format!("Failed to delete key {}", args.id))?;
			if let KeyIdType::Id(id) = args.id {
				println!("Key {} deleted", id);
			} else {
				println!("Key {}({}) deleted", args.id, key.id.value());
			}

			if CliContext::global().open {
				let _ = open::that(format!(
					"https://{}/-/profile/keys",
					CliContext::global().repo.host
				));
			}

			Ok(ExitCode::from(0))
		}
	}
}
