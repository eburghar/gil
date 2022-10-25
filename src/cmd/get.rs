use crate::{
	args::{Get, Opts},
	config::Config,
	utils::{get_lock, get_or_create_dir, get_project, save_lock},
};
use anyhow::{Context, Result};
use bytesize::ByteSize;
use flate2::read::GzDecoder;
use gitlab::{ArchiveFormat, Gitlab};
use std::{
	fs::{create_dir, remove_dir_all, File},
	io,
};
use tar::{Archive, EntryType};

pub fn cmd(
	conf: &Config,
	args: &Get,
	Opts {
		verbose, config, ..
	}: &Opts,
) -> Result<()> {
	// connect to gitlab instance using host and token from config file
	let gitlab = Gitlab::with_oauth2(&conf.host, &conf.token)
		//let gitlab = Gitlab::new(&conf.host, &conf.token)
		.with_context(|| format!("Can't connect to {}", &conf.host))?;
	// create the dest directory and save as an Option<Path> for later use
	let dest_dir = get_or_create_dir(&args.dir, args.keep, args.update, *verbose)?;
	// get previous commits from lock file or empty list
	let mut lock = get_lock(config)?;

	// extract archive to specified directory
	// iterate over each project name indicated in the config file
	for (prj, br) in conf.archives.iter() {
		// skip gitlab requests and extraction if a dir with the name of the project already exists
		let i = match prj.rfind('/') {
			Some(i) if (i + 1) < prj.len() => i + 1,
			_ => 0,
		};
		let prj_dir = dest_dir.join(&prj[i..]);
		let is_extracted = prj_dir.exists();

		// skip before any API call in keep mode
		if args.keep && is_extracted {
			log::info!("{} already extracted", &prj);
			continue;
		}

		let proj = match get_project(&gitlab, prj, br) {
			Ok(proj) => proj,
			Err(err) => {
				log::err!("{}", &err);
				continue;
			}
		};

		let project = proj.project;
		let last_commit = proj.commit.id.value();
		// get locked_commit or last_commit
		let mut found = false;
		let mut commit = match lock.get(prj) {
			Some(locked_commit) => {
				found = true;
				locked_commit.to_string()
			}
			None => last_commit.to_string(),
		};

		if args.update && is_extracted {
			if found && commit == *last_commit {
				log::info!("{}-{} already extracted", prj, commit);
				continue;
			} else {
				// remove project dir before update
				remove_dir_all(&prj_dir)
					.with_context(|| format!("Can't remove dir {}", prj_dir.display()))?;
				commit = last_commit.to_string();
			}
		}

		// get the archive.tar.gz from project branch last commit
		let targz = match gitlab.get_archive(project.id, ArchiveFormat::TarGz, &commit) {
			Ok(archive) => archive,
			Err(err) => {
				log::error!("Can't get {} archive: {:?}", &project.name, &err);
				continue;
			}
		};

		log::info!("extracting branch {} of {}", &br, &prj);
		// chain gzip reader and arquive reader
		let tar = GzDecoder::new(targz);
		let mut arquive = Archive::new(tar);

		// for each entry in the arquive
		for entry in arquive.entries().unwrap() {
			let mut entry = match entry {
				Ok(entry) => entry,
				Err(err) => {
					log::error!("  Can't get {} archive entry: {:?}", &project.name, &err);
					continue;
				}
			};

			// get the path
			let mut entry_path = entry.path().unwrap().into_owned();
			// turn into components
			let mut components = entry_path.components();
			// skip first components if indicated in command line args
			if args.strip > 0 {
				for _ in 0..args.strip {
					components.next();
				}
				// and reassemble dest_path
				entry_path = components.as_path().to_path_buf();
			}
			// don't do anything if empty path
			if entry_path.to_string_lossy().is_empty() {
				continue;
			}
			// append destination dir to entry path
			entry_path = dest_dir.join(entry_path);
			// get the entry type
			let file_type = entry.header().entry_type();
			match file_type {
				// if it's a directory, create it if doesn't exist
				EntryType::Directory => {
					if !entry_path.exists() {
						match create_dir(&entry_path) {
							Ok(()) => {
								if *verbose {
									log::info!("  {}", &entry_path.to_string_lossy());
								}
							}
							Err(err) => {
								log::error!(
									"  Can't create dir {}: {:?}",
									&entry_path.to_string_lossy(),
									&err
								);
								continue;
							}
						}
					}
				}

				// if it's a file, extract it to local filesystem
				EntryType::Regular => {
					let mut file = match File::create(&entry_path) {
						Ok(file) => file,
						Err(err) => {
							log: error!(
								"  Can't create file {}: {:?}",
								&entry_path.to_string_lossy(),
								&err
							);
							continue;
						}
					};
					match io::copy(&mut entry, &mut file) {
						Ok(size) => {
							if *verbose {
								log::error!(
									"  {} ({})",
									&entry_path.to_string_lossy(),
									ByteSize(size)
								);
							}
						}
						Err(err) => {
							log::error!(
								"  Can't extract {}: {:?}",
								&entry_path.to_string_lossy(),
								&err
							);
							continue;
						}
					}
				}
				// TODO: support other types (links)
				_ => {
					log::error!(
						"  {} ({:?}) ignored",
						&entry_path.to_string_lossy(),
						&file_type
					);
					continue;
				}
			}
		}
		// insert the commit name in the dictionnary
		lock.entry(prj.clone())
			.and_modify(|e| *e = commit.clone())
			.or_insert_with(|| commit.clone());
	}
	save_lock(config, args.update, &lock)?;

	Ok(())
}
