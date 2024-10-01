use std::io::{Error, ErrorKind, Result};
use std::{env, fs};

pub fn create_dir(project_name: &str) -> Result<()> {
	let path = env::current_dir()?.join(project_name);

	if path.exists() {
		return Err(Error::new(
			ErrorKind::AlreadyExists,
			format!("Directory '{}' already exists", project_name),
		));
	}

	fs::create_dir(&path)?;

	env::set_current_dir(&path)?;

	Ok(())
}
