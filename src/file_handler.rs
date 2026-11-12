use anyhow::{Context, Result, bail};
use std::{
	env, fs,
	fs::OpenOptions,
	io::Write,
	path::{Path, PathBuf},
	sync::atomic::{AtomicBool, Ordering},
	time::{SystemTime, UNIX_EPOCH},
};

static DRY_RUN: AtomicBool = AtomicBool::new(false);
static FORCE: AtomicBool = AtomicBool::new(false);

/// Backup suffix appended to files sticks modifies or removes, e.g.
/// `Makefile.sticks.bak`. Keeps edits reversible.
pub const BACKUP_SUFFIX: &str = ".sticks.bak";

pub fn set_dry_run(enabled: bool) {
	DRY_RUN.store(enabled, Ordering::Relaxed);
}

pub fn dry_run() -> bool {
	DRY_RUN.load(Ordering::Relaxed)
}

pub fn set_force(enabled: bool) {
	FORCE.store(enabled, Ordering::Relaxed);
}

pub fn force() -> bool {
	FORCE.load(Ordering::Relaxed)
}

/// `<path>.sticks.bak` — where the pre-edit contents are preserved.
pub fn backup_path(path: &Path) -> PathBuf {
	let mut name = path
		.file_name()
		.map(|name| name.to_string_lossy().into_owned())
		.unwrap_or_else(|| "file".to_string());
	name.push_str(BACKUP_SUFFIX);
	path.with_file_name(name)
}

fn print_diff(path: &Path, old: Option<&str>, new: &str) {
	let old = old.unwrap_or("");
	let diff = similar::TextDiff::from_lines(old, new)
		.unified_diff()
		.header(
			&format!("a/{}", path.display()),
			&format!("b/{}", path.display()),
		)
		.to_string();
	if diff.trim().is_empty() {
		println!("would write {} (no changes)", path.display());
	} else {
		print!("{diff}");
	}
}

/// Copy `path` over its `.sticks.bak` backup before a modifying write.
/// Best-effort for non-UTF8 files (falls back to a byte copy).
pub fn backup_existing(path: &Path) -> Result<()> {
	let backup = backup_path(path);
	if let Some(parent) = backup.parent() {
		fs::create_dir_all(parent)?;
	}
	fs::copy(path, &backup).with_context(|| {
		format!(
			"Failed to back up {} to {}",
			path.display(),
			backup.display()
		)
	})?;
	Ok(())
}

/// Rename `path` to its `.sticks.bak` backup instead of deleting it, so
/// removals stay reversible. Any previous backup is replaced.
pub fn remove_with_backup(path: &Path) -> Result<()> {
	if dry_run() {
		println!(
			"would move {} to {}",
			path.display(),
			backup_path(path).display()
		);
		return Ok(());
	}
	let backup = backup_path(path);
	if backup.is_dir() {
		fs::remove_dir_all(&backup)
			.with_context(|| format!("Failed to replace backup {}", backup.display()))?;
	} else if backup.exists() || backup.symlink_metadata().is_ok() {
		fs::remove_file(&backup)
			.with_context(|| format!("Failed to replace backup {}", backup.display()))?;
	}
	fs::rename(path, &backup).with_context(|| {
		format!(
			"Failed to move {} to backup {}",
			path.display(),
			backup.display()
		)
	})?;
	println!("Moved {} to {}", path.display(), backup.display());
	Ok(())
}

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

/// CWD-independent variant of [`project_path`].
pub fn project_path_in(base_dir: &Path, project_name: &str) -> Result<std::path::PathBuf> {
	validate_project_name(project_name)?;
	Ok(base_dir.join(project_name))
}

fn fsync_parent(path: &Path) {
	// Best-effort durability: fsync the parent directory so the rename is
	// persisted. Failures are ignored (e.g. Windows, read-only mounts).
	#[cfg(unix)]
	{
		use std::os::unix::io::AsRawFd;
		if let Some(parent) = path.parent()
			&& let Ok(file) = fs::File::open(parent)
		{
			unsafe {
				libc::fsync(file.as_raw_fd());
			}
		}
	}
}

pub fn write_new_file(path: &Path, content: &str) -> Result<()> {
	if path.exists() {
		if dry_run() {
			print_diff(path, fs::read_to_string(path).ok().as_deref(), content);
			return Ok(());
		}
		if force() {
			return write_atomic(path, content);
		}
		bail!("Refusing to overwrite existing file: {}", path.display());
	}

	if dry_run() {
		println!("would create {}", path.display());
		print_diff(path, None, content);
		return Ok(());
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
		.with_context(|| format!("Failed to write {}", path.display()))?;
	file.sync_all()
		.with_context(|| format!("Failed to flush {}", path.display()))?;
	drop(file);
	fsync_parent(path);
	Ok(())
}

pub fn ensure_path_available(path: &Path) -> Result<()> {
	match fs::symlink_metadata(path) {
		Ok(_) => {
			if dry_run() {
				println!("would replace existing path {}", path.display());
				return Ok(());
			}
			if force() {
				return Ok(());
			}
			bail!("Refusing to overwrite existing path: {}", path.display());
		}
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
		Err(error) => {
			Err(error).with_context(|| format!("Failed to inspect path {}", path.display()))
		}
	}
}

pub fn atomic_write_new(path: &Path, content: &str) -> Result<()> {
	// Kept for backwards compatibility: creates a *new* file and refuses to
	// overwrite, like `write_new_file`. For atomic replacement of an existing
	// file, use `write_atomic`.
	write_new_file(path, content)
}

pub fn write_atomic(path: &Path, content: &str) -> Result<()> {
	let previous = match fs::read_to_string(path) {
		Ok(old) => Some(old),
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
		Err(_) => None, // non-UTF8 file: still writable, just no diff/backup text
	};

	// Idempotent no-op: identical content means nothing to write, so no
	// backup churn and no mtime bump.
	if previous.as_deref() == Some(content) {
		return Ok(());
	}

	if dry_run() {
		print_diff(path, previous.as_deref(), content);
		return Ok(());
	}

	// Preserve the pre-edit contents before touching the file. Backup is
	// keyed on existence, not readability, so non-UTF8 files are covered.
	if path.exists() {
		backup_existing(path)?;
	}

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

	// Whatever goes wrong below, the temporary file must not be left behind.
	let write_result = (|| -> Result<()> {
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
		Ok(())
	})();
	if let Err(error) = write_result {
		let _ = fs::remove_file(&temp_path);
		return Err(error);
	}

	// `fs::rename` cannot replace an existing file on Windows, so the original
	// is moved aside first. Keep its contents in memory so a failed rename can
	// be rolled back instead of destroying the user's build file.
	#[cfg(target_os = "windows")]
	let rollback: Option<String> = if path.exists() {
		match fs::read_to_string(path) {
			Ok(previous) => Some(previous),
			Err(_) => None,
		}
	} else {
		None
	};
	#[cfg(target_os = "windows")]
	if path.exists() {
		fs::remove_file(path).with_context(|| format!("Failed to replace {}", path.display()))?;
	}

	if let Err(error) = fs::rename(&temp_path, path) {
		let _ = fs::remove_file(&temp_path);
		// Put the original content back so the failed write is not destructive.
		#[cfg(target_os = "windows")]
		if let Some(previous) = rollback {
			let _ = fs::write(path, previous);
		}
		return Err(error).with_context(|| format!("Failed to replace {}", path.display()));
	}

	fsync_parent(path);

	Ok(())
}
