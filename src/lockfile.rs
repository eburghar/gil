use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fs::File;
use std::ops::{Deref, DerefMut};
use std::path::Path;

pub struct LockFile {
	name: String,
	commits: BTreeMap<String, String>,
}

impl LockFile {
	pub fn open(name: &str) -> Result<Self> {
		// open lock file
		let lock = Path::new(&name).with_extension("lock");
		let commits: BTreeMap<String, String> = if let Ok(file) = File::open(&lock) {
			// deserialize lock
			serde_yaml::from_reader(file).with_context(|| format!("Can't read {:?}", &lock))?
		} else {
			// create empty commits list
			BTreeMap::default()
		};
		Ok(Self {
			name: name.to_owned(),
			commits,
		})
	}

	pub fn save(&self, update: bool) -> Result<()> {
		// save lock file if update mode or file doesn't exists
		let lock = Path::new(&self.name).with_extension("lock");
		if update || !Path::new(&lock).exists() {
			if let Ok(file) = File::create(&lock) {
				serde_yaml::to_writer(file, &self.commits)
					.with_context(|| format!("Can't write {:?}", &lock))?;
			}
		}
		Ok(())
	}
}

impl Deref for LockFile {
	type Target = BTreeMap<String, String>;

	fn deref(&self) -> &Self::Target {
		&self.commits
	}
}

impl DerefMut for LockFile {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.commits
	}
}
