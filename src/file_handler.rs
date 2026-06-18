use anyhow::{Context, Result, bail};
use std::{
	env, fs,
	fs::OpenOptions,
	io::Write,
	path::Path,
	time::{SystemTime, UNIX_EPOCH},
};

pub fn validate_project_name(name: &str) -> Result<()> {
	if name.is_empty() || name == "." || name == ".." {
		bail!("Project name must be a non-empty directory name");
	}

	if name.starts_with('-') || name.contains('/') || name.contains('\\') {
		bail!("Project name contains an invalid path character: {}", name);
	}

	if name.chars().count() > 128 {
		bail!("Project name cannot exceed 128 characters");
	}

	if !name
		.chars()
		.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
	{
		bail!(
			"Project name can only contain ASCII letters, numbers, '_' or '-': {}",
			name
		);
	}

	let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
	if matches!(
		stem.as_str(),
		"CON"
			| "PRN" | "AUX"
			| "NUL" | "COM1"
			| "COM2" | "COM3"
			| "COM4" | "COM5"
			| "COM6" | "COM7"
			| "COM8" | "COM9"
			| "LPT1" | "LPT2"
			| "LPT3" | "LPT4"
			| "LPT5" | "LPT6"
			| "LPT7" | "LPT8"
			| "LPT9"
	) {
		bail!("Project name is reserved on Windows: {}", name);
	}

	Ok(())
}

pub fn create_dir(project_name: &str) -> Result<()> {
	validate_project_name(project_name)?;
	let path = project_path(project_name)?;

	if path.exists() {
		bail!("Directory '{}' already exists", project_name);
	}

	fs::create_dir(&path)
		.with_context(|| format!("Failed to create directory '{}'", project_name))?;
	Ok(())
}

pub fn project_path(project_name: &str) -> Result<std::path::PathBuf> {
	validate_project_name(project_name)?;
	Ok(env::current_dir()
		.context("Failed to get current directory")?
		.join(project_name))
}

pub fn write_new_file(path: &Path, content: &str) -> Result<()> {
	if path.exists() {
		bail!("Refusing to overwrite existing file: {}", path.display());
	}

	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)
			.with_context(|| format!("Failed to create directory {}", parent.display()))?;
	}
	let mut file = fs::OpenOptions::new()
		.write(true)
		.create_new(true)
		.open(path)
		.with_context(|| format!("Failed to create {}", path.display()))?;
	file.write_all(content.as_bytes())
		.with_context(|| format!("Failed to write {}", path.display()))
}

pub fn ensure_path_available(path: &Path) -> Result<()> {
	match fs::symlink_metadata(path) {
		Ok(_) => bail!("Refusing to overwrite existing path: {}", path.display()),
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
		Err(error) => {
			Err(error).with_context(|| format!("Failed to inspect path {}", path.display()))
		}
	}
}

pub fn atomic_write_new(path: &Path, content: &str) -> Result<()> {
	write_new_file(path, content)
}

pub fn write_atomic(path: &Path, content: &str) -> Result<()> {
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)
			.with_context(|| format!("Failed to create directory {}", parent.display()))?;
	}

	let suffix = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|duration| duration.as_nanos())
		.unwrap_or_default();
	let temp_path = path.with_file_name(format!(
		".{}.{}.{}.tmp",
		path.file_name()
			.and_then(|name| name.to_str())
			.unwrap_or("file"),
		std::process::id(),
		suffix
	));

	let mut temporary_file = OpenOptions::new()
		.write(true)
		.create_new(true)
		.open(&temp_path)
		.with_context(|| format!("Failed to create temporary file {}", temp_path.display()))?;
	temporary_file
		.write_all(content.as_bytes())
		.with_context(|| format!("Failed to write temporary file {}", temp_path.display()))?;
	temporary_file
		.sync_all()
		.with_context(|| format!("Failed to flush temporary file {}", temp_path.display()))?;

	#[cfg(target_os = "windows")]
	if path.exists() {
		fs::remove_file(path).with_context(|| format!("Failed to replace {}", path.display()))?;
	}

	if let Err(error) = fs::rename(&temp_path, path) {
		let _ = fs::remove_file(&temp_path);
		return Err(error).with_context(|| format!("Failed to replace {}", path.display()));
	}

	Ok(())
}
