use crate::{
	archive::Archive,
	args::{self, ArchiveCmd},
	context::CliContext,
	lockfile::LockFile,
	utils::get_or_create_dir,
};

use anyhow::{Context, Result};
use bytesize::ByteSize;
use flate2::read::GzDecoder;
use gitlab::api::{self, Query};
use serde::Deserialize;
use std::{
	collections::BTreeMap,
	fs::{create_dir, create_dir_all, remove_dir_all, File},
	io,
	ops::Deref,
	path::PathBuf,
};

#[derive(Deserialize)]
/// Configuration for batch mode (extract sub command)
pub struct BatchConfig(BTreeMap<String, String>);

impl BatchConfig {
	/// Initializer from parameters
	pub fn singleton(project: String, tag: String) -> Self {
		let archives: BTreeMap<_, _> = [(project, tag)].into();
		Self(archives)
	}

	/// Initialize from a file
	pub fn from_file(config: &str) -> Result<Self> {
		// open configuration file
		let file = File::open(&config).with_context(|| format!("Can't open {}", &config))?;
		// deserialize configuration
		let config: Self =
			serde_yaml::from_reader(file).with_context(|| format!("Can't read {}", &config))?;
		Ok(config)
	}
}

/// Direct access to the map
impl Deref for BatchConfig {
	type Target = BTreeMap<String, String>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

/// Command implementaton
pub fn cmd(context: &CliContext, args: &args::Archive) -> Result<()> {
	match &args.cmd {
		ArchiveCmd::Extract(args) => {
			// rename mode is like -s 1 (we remove the first path component) + replace by the project name
			let strip = if args.rename { 1 } else { args.strip };
			// determine the list of project/tag to extract
			let batch = if let Some(ref config) = args.batch {
				// in batch mode, we read from a file
				BatchConfig::from_file(config)?
			} else {
				// in command line we extract only 1 project given from command line arguments
				// let project = get_project(&args.project)?;
				// let tag = get_tag(&args.tag)?;
				let project = context.get_project(args.project.as_ref())?;
				let tag = context.get_tag(args.tag.as_ref())?;
				BatchConfig::singleton(project.to_owned(), tag.to_owned())
			};

			// create the dest directory
			let dest_dir = get_or_create_dir(&args.dir, args.keep, args.update, context.verbose)?;
			// open lock file (update mode)
			let lock_name = if let Some(ref batch) = args.batch {
				batch
			} else {
				&context.config.name
			};
			let mut lock = LockFile::open(lock_name)?;

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

				let tag = context.get_tag_commit(project, tag)?;
				// get locked_commit or tag commit
				let mut found = false;
				let mut commit = match lock.get(project) {
					Some(commit) => {
						found = true;
						commit
					}
					None => tag.commit.id.value(),
				};

				if args.update && is_extracted {
					// skip if extracted and locked commit match
					if found && commit == tag.commit.id.value() {
						println!(
							"{} {} ({}) already extracted",
							project,
							tag.name,
							&commit[..8]
						);
						continue;
					} else {
						// issue a warning when version mismatch before overwriting
						if commit != tag.commit.id.value() {
							eprintln!(
								"Extracted commit {} and {} commit {} mismatch",
								&commit[..8],
								&tag.name,
								&tag.commit.short_id.value()
							);
						}
						// remove project dir before update
						remove_dir_all(&prj_dir)
							.with_context(|| format!("Can't remove dir {}", prj_dir.display()))?;
						commit = tag.commit.id.value();
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
				let targz = api::raw(endpoint).query(&context.gitlab)?;

				println!("Extracting {} {} ({})", &project, &tag.name, &commit[..8]);
				// chain gzip reader and arquive reader. turn vec<u8> to a slice
				// to be able to io::Read from it
				let tar = GzDecoder::new(targz.as_slice());
				let mut arquive = tar::Archive::new(tar);

				// for each entry in the arquive
				for entry in arquive.entries()? {
					let mut entry = match entry {
						Ok(entry) => entry,
						Err(err) => {
							eprintln!("  Can't get {} archive entry: {:?}", &project, &err);
							continue;
						}
					};

					// strip leading path components if necessary
					let entry_path: PathBuf = entry.path()?.components().skip(strip).collect();
					// don't do anything if empty path
					if entry_path.to_str().filter(|s| s.is_empty()).is_some() {
						continue;
					}

					// append project dir in rename mode otherwise append destination dir
					let entry_path = if args.rename {
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
										if context.verbose {
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
									if context.verbose {
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