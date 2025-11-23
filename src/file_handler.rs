use anyhow::{Context, Result};
use std::{env, fs};

pub fn create_dir(project_name: &str) -> Result<()> {
	let path = env::current_dir()
		.context("Failed to get current directory")?
		.join(project_name);

	if path.exists() {
		anyhow::bail!("Directory '{}' already exists", project_name);
	}

	fs::create_dir(&path)
		.with_context(|| format!("Failed to create directory '{}'", project_name))?;

	env::set_current_dir(&path)
		.with_context(|| format!("Failed to change to directory '{}'", project_name))?;

	Ok(())
}
