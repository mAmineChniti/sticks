use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

const GITHUB_REPO: &str = "https://github.com/mAmineChniti/sticks/releases/latest/download";

fn get_install_path() -> Result<PathBuf> {
	if let Ok(output) = Command::new("which").arg("sticks").output() {
		if output.status.success() {
			let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
			return Ok(PathBuf::from(path));
		}
	}

	let paths = vec![
		PathBuf::from("/usr/local/bin/sticks"),
		PathBuf::from("/usr/bin/sticks"),
		dirs::home_dir()
			.map(|h| h.join(".cargo/bin/sticks"))
			.unwrap_or_default(),
	];

	for path in paths {
		if path.exists() {
			return Ok(path);
		}
	}

	anyhow::bail!("Could not determine sticks installation path")
}

fn get_architecture() -> &'static str {
	#[cfg(target_arch = "x86_64")]
	return "x86_64";
	#[cfg(target_arch = "aarch64")]
	return "aarch64";
	#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
	return "unknown";
}

pub fn update_project() -> Result<()> {
	println!("🔄 Checking for updates...");

	let arch = get_architecture();
	if arch == "unknown" {
		anyhow::bail!("Unsupported architecture. Please update manually.");
	}

	let install_path = get_install_path()?;
	let is_system_install = install_path.starts_with("/usr/bin")
		|| install_path.starts_with("/usr/local/bin");

	if is_system_install {
		println!("ℹ️  System installation detected.");
		println!("📦 Please use your package manager to update:");
		println!();
		println!("  Arch Linux:     sudo pacman -Syu sticks");
		println!("  Debian/Ubuntu:  sudo apt update && sudo apt upgrade sticks");
		println!();
		println!("💡 Or download the latest package from:");
		println!("   {}/download", GITHUB_REPO);
		return Ok(());
	}

	println!("📥 Downloading latest version from GitHub releases...");

	let temp_dir = env::temp_dir().join(format!("sticks-update-{}", std::process::id()));
	fs::create_dir_all(&temp_dir).context("Failed to create temp directory")?;

	let binary_name = format!("sticks-linux-{}", arch);
	let download_url = format!("{}/{}", GITHUB_REPO, binary_name);
	let temp_binary = temp_dir.join("sticks");

	let status = Command::new("curl")
		.args([
			"-L",
			"-f",
			"-o",
			temp_binary.to_str().unwrap(),
			&download_url,
		])
		.status()
		.context("Failed to download update. Is curl installed?")?;

	if !status.success() {
		fs::remove_dir_all(&temp_dir).ok();
		anyhow::bail!(
			"Failed to download update from {}. \
			Please check your internet connection or download manually.",
			download_url
		);
	}

	let mut perms = fs::metadata(&temp_binary)
		.context("Failed to get file permissions")?
		.permissions();
	perms.set_mode(0o755);
	fs::set_permissions(&temp_binary, perms).context("Failed to set executable permissions")?;

	fs::copy(&temp_binary, &install_path)
		.with_context(|| format!("Failed to replace binary at {:?}", install_path))?;

	fs::remove_dir_all(&temp_dir).ok();

	println!("✓ Successfully updated sticks to the latest version!");
	println!("💡 Run 'sticks --version' to verify the new version.");

	Ok(())
}

fn dirs_home_dir() -> Option<PathBuf> {
	env::var_os("HOME").map(PathBuf::from)
}

mod dirs {
	use super::*;
	pub fn home_dir() -> Option<PathBuf> {
		dirs_home_dir()
	}
}
