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
            let ssh_key_id: Option<u64> = None;
            let title = args
                .title
                .as_ref()
                .map(String::as_str)
                .unwrap_or_else(|| ssh_key.comment());

            if args.overwrite {
                // TODO: search key by fingerprint using /keys api instead of title
                // try to delete existing key with same title on overwrite mode
                if let Ok(key) = context.get_key(&title.to_string()) {
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
                // TODO: search key by fingerprint using /keys api and set ssh_key_id
                let url = if let Some(key_id) = ssh_key_id {
                    format!(
                        "https://{}/-/profile/keys/{}",
                        context.config.host.name, key_id
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
            let key = context.get_key(&args.name)?;
            let endpoint = DeleteKey::builder().key_id(key.id.value()).build()?;
            api::ignore(endpoint)
                .query(&context.gitlab)
                .with_context(|| format!("Failed to delete key {}", args.name))?;
            println!("Key {}({}) deleted", args.name, key.id.value());

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
