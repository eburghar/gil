use crate::{
	archive::Archive,
	args,
	config::BatchConfig,
	lockfile::LockFile,
	utils::{get_or_create_dir, get_project, get_tag, get_tag_commit},
};
use anyhow::{Context, Result};
use bytesize::ByteSize;
use flate2::read::GzDecoder;
use gitlab::{
	api::{self, Query},
	Gitlab,
};
use std::{
	fs::{create_dir, create_dir_all, remove_dir_all, File},
	io,
};

pub fn cmd(gitlab: Gitlab, config: &str, verbose: bool, args: &args::Archive) -> Result<()> {
	match &args.cmd {
		args::ArchiveCmd::Extract(args) => {
			// in rename mode we remove the first path component and replace by the project name
			let strip = if args.rename { 1 } else { args.strip };
			// determine the list of project/tag to extract
			let batch = if let Some(ref config) = args.batch {
				// in batch mode, we read from a file
				BatchConfig::read(config)?
			} else {
				// in command line we extract only 1 project given from command line arguments
				let project = get_project(&args.project)?;
				let tag = get_tag(&args.tag)?;
				BatchConfig::singleton(project.to_owned(), tag.to_owned())
			};

			// create the dest directory
			let dest_dir = get_or_create_dir(&args.dir, args.keep, args.update, verbose)?;
			// open lock file (update mode)
			let lock = if let Some(ref batch) = args.batch {
				batch
			} else {
				config
			};
			let mut lock = LockFile::open(lock)?;

			// extract all rchives to specified directory
			for (project, tag) in batch.iter() {
				// check if a dir with the name of the project already exists
				// this works reliably only in rename mode (-r)
				let i = match project.rfind('/') {
					Some(i) if (i + 1) < project.len() => i + 1,
					_ => 0,
				};
				let prj_dir = dest_dir.join(&project[i..]);
				let is_extracted = prj_dir.exists();

				// don't overwrite if we were asked to keep.
				if args.keep && is_extracted {
					println!("{} already extracted", &project);
					// if no entry in lockfile in update mode, there is no garantee that we
					// have an extraction of the right version
					if args.update && !lock.contains_key(project) {
						eprintln!("We couldn't find any entry in the lockfile\n.Remove or run without -k to overwrite.",)
					}
					continue;
				}

				let tag = get_tag_commit(&gitlab, project, tag)?;
				// get locked_commit or tag commit
				let mut found = false;
				let mut commit = match lock.get(project) {
					Some(commit) => {
						found = true;
						commit.to_string()
					}
					None => tag.commit.id.to_owned(),
				};

				if args.update && is_extracted {
					// skip if extracted and locked commit match
					if found && commit == *tag.commit.id {
						println!(
							"{} {} ({}) already extracted",
							project,
							tag.name,
							&commit[..8]
						);
						continue;
					} else {
						// issue a warning when version mismatch before overwriting
						if commit != *tag.commit.id {
							eprintln!(
								"Extracted commit {} and {} commit {} mismatch",
								&commit[..8],
								&tag.name,
								&tag.commit.id[..8]
							);
						}
						// remove project dir before update
						remove_dir_all(&prj_dir)
							.with_context(|| format!("Can't remove dir {}", prj_dir.display()))?;
						commit = tag.commit.id.to_string();
					}
				}

				// create the top level dir when it is to be renamed after the project
				if args.rename {
					create_dir_all(&prj_dir)
						.with_context(|| format!("Can't create dir {:?}", &prj_dir))?;
				}

				let endpoint = Archive::builder()
					.project(project.to_owned())
					.sha(commit.to_owned())
					.build()?;
				// NOTE: api::raw returns a vec<u8>. It would be
				// more memory efficient to return the rewest::Response to read
				// from a stream instead
				let targz = api::raw(endpoint).query(&gitlab)?;

				println!("Extracting {} {} ({})", &project, &tag.name, &commit[..8]);
				// chain gzip reader and arquive reader. turn vec<u8> to a slice
				// to be able to io::Read from it
				let tar = GzDecoder::new(targz.as_slice());
				let mut arquive = tar::Archive::new(tar);

				// for each entry in the arquive
				for entry in arquive.entries().unwrap() {
					let mut entry = match entry {
						Ok(entry) => entry,
						Err(err) => {
							eprintln!("  Can't get {} archive entry: {:?}", &project, &err);
							continue;
						}
					};

					// get the path
					let mut entry_path = entry.path().unwrap().into_owned();
					// turn into components
					let mut components = entry_path.components();
					// skip first components if indicated in command line args
					if strip > 0 {
						for _ in 0..strip {
							components.next();
						}
						// and reassemble dest_path
						entry_path = components.as_path().to_path_buf();
					}
					// don't do anything if empty path
					if entry_path.to_string_lossy().is_empty() {
						continue;
					}
					// append project dir in rename mode otherwise append destination dir
					entry_path = if args.rename {
						prj_dir.join(entry_path)
					} else {
						dest_dir.join(entry_path)
					};
					// get the entry type
					let file_type = entry.header().entry_type();
					match file_type {
						// if it's a directory, create it if doesn't exist
						tar::EntryType::Directory => {
							if !entry_path.exists() {
								match create_dir(&entry_path) {
									Ok(()) => {
										if verbose {
											println!("  {}", &entry_path.to_string_lossy());
										}
									}
									Err(err) => {
										eprintln!(
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
						tar::EntryType::Regular => {
							let mut file = match File::create(&entry_path) {
								Ok(file) => file,
								Err(err) => {
									eprintln!(
										"  Can't create file {}: {:?}",
										&entry_path.to_string_lossy(),
										&err
									);
									continue;
								}
							};
							match io::copy(&mut entry, &mut file) {
								Ok(size) => {
									if verbose {
										println!(
											"  {} ({})",
											&entry_path.to_string_lossy(),
											ByteSize(size)
										);
									}
								}
								Err(err) => {
									eprintln!(
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
							eprintln!(
								"  {} ({:?}) ignored",
								&entry_path.to_string_lossy(),
								&file_type
							);
							continue;
						}
					}
				}

				*lock.entry(project.to_owned()).or_default() = commit.to_owned();
			}
			lock.save(args.update)?;

			Ok(())
		}
	}
}
