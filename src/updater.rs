use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

use crate::constants::{github, install_paths};

fn get_install_path() -> Result<PathBuf> {
	if let Ok(output) = Command::new("which").arg("sticks").output() {
		if output.status.success() {
			let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
			return Ok(PathBuf::from(path));
		}
	}

	let paths = vec![
		PathBuf::from(install_paths::USR_LOCAL_BIN),
		PathBuf::from(install_paths::USR_BIN),
		dirs::home_dir()
			.map(|h| h.join(install_paths::CARGO_BIN_SUFFIX))
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
	#[cfg(not(target_arch = "x86_64"))]
	return "unsupported";
}

fn get_current_version() -> String {
	env!("CARGO_PKG_VERSION").to_string()
}

fn get_latest_version() -> Result<String> {
	let output = Command::new("curl")
		.args([
			"-s",
			"-H",
			"Accept: application/vnd.github.v3+json",
			github::RELEASE_API_URL,
		])
		.output()
		.context("Failed to fetch latest release information")?;

	if !output.status.success() {
		anyhow::bail!("Failed to check for latest version");
	}

	let json = String::from_utf8_lossy(&output.stdout);

	if json.trim().is_empty() {
		anyhow::bail!("Empty response from GitHub API");
	}

	if let Some(tag_start) = json.find("\"tag_name\":") {
		let after_tag = &json[tag_start + 11..].trim_start();
		if let Some(quote_start) = after_tag.find('"') {
			let version_str = &after_tag[quote_start + 1..];
			if let Some(quote_end) = version_str.find('"') {
				let version = version_str[..quote_end].trim_start_matches('v').to_string();
				if !version.is_empty() {
					return Ok(version);
				}
			}
		}
	}

	anyhow::bail!(
		"Could not parse version from GitHub API response. Response: {}",
		if json.len() > 200 {
			&json[..200]
		} else {
			&json
		}
	)
}

pub fn update_project() -> Result<()> {
	println!("🔄 Checking for updates...");

	let current_version = get_current_version();
	let arch = get_architecture();
	if arch == "unsupported" {
		anyhow::bail!(
			"Unsupported architecture: only x86_64 is supported. Please update manually."
		);
	}

	let install_path = get_install_path()?;
	let is_system_install =
		install_path.starts_with("/usr/bin") || install_path.starts_with("/usr/local/bin");

	let is_aur_install = PathBuf::from("/usr/bin/sticks").exists()
		&& Command::new("pacman")
			.arg("-Qi")
			.arg("sticks-aur")
			.output()
			.map(|o| o.status.success())
			.unwrap_or(false);

	let is_deb_install = PathBuf::from("/usr/bin/dpkg").exists();

	let latest_version = get_latest_version().context("Failed to check for updates")?;

	if current_version == latest_version {
		println!(
			"✓ You're already on the latest version (v{})!",
			current_version
		);
		return Ok(());
	}

	println!(
		"📦 Update available: v{} → v{}",
		current_version, latest_version
	);

	if is_system_install && is_aur_install {
		println!();
		println!("ℹ️  AUR installation detected.");
		println!("📦 Please use your AUR helper to update:");
		println!();
		println!("  yay -Syu sticks-aur");
		println!("  paru -Syu sticks-aur");
		println!();
		println!("💡 Or manually update:");
		println!("  cd sticks-aur && git pull && makepkg -si");
		return Ok(());
	}

	if is_system_install && is_deb_install {
		println!();
		println!(
			"📥 Downloading .deb package v{} from GitHub releases...",
			latest_version
		);

		let temp_dir = env::temp_dir().join(format!("sticks-update-{}", std::process::id()));
		fs::create_dir_all(&temp_dir).context("Failed to create temp directory")?;

		let deb_name = format!("sticks_{}-1_amd64.deb", latest_version);
		let download_url = format!("{}/{}", github::RELEASE_DOWNLOAD_URL, deb_name);
		let temp_deb = temp_dir.join(&deb_name);

		let status = if Command::new("wget2").arg("--version").output().is_ok() {
			Command::new("wget2")
				.args(["-c", "-O"])
				.arg(&temp_deb)
				.arg(&download_url)
				.status()
				.context("Failed to download .deb package with wget2")
		} else if Command::new("wget").arg("--version").output().is_ok() {
			Command::new("wget")
				.args(["-c", "-O"])
				.arg(&temp_deb)
				.arg(&download_url)
				.status()
				.context("Failed to download .deb package with wget")
		} else {
			Command::new("curl")
				.args(["-L", "-C", "-", "-o"])
				.arg(&temp_deb)
				.arg(&download_url)
				.status()
				.context("Failed to download .deb package. Is curl, wget, or wget2 installed?")
		}?;

		if !status.success() {
			fs::remove_dir_all(&temp_dir).ok();
			anyhow::bail!(
				"Failed to download .deb package from {}. \
				Please check your internet connection or download manually.",
				download_url
			);
		}

		println!("📦 Installing .deb package (requires sudo)...");
		let install_status = Command::new("sudo")
			.args(["dpkg", "-i", temp_deb.to_str().unwrap()])
			.status()
			.context("Failed to install .deb package")?;

		fs::remove_dir_all(&temp_dir).ok();

		if !install_status.success() {
			anyhow::bail!("Failed to install .deb package. You may need to run 'sudo apt --fix-broken install'");
		}

		println!();
		println!(
			"✓ Successfully upgraded from v{} to v{}!",
			current_version, latest_version
		);
		println!("💡 Run 'sticks --version' to verify the installation.");
		return Ok(());
	}

	println!("📥 Downloading v{} from GitHub releases...", latest_version);

	let temp_dir = env::temp_dir().join(format!("sticks-update-{}", std::process::id()));
	fs::create_dir_all(&temp_dir).context("Failed to create temp directory")?;

	let binary_name = format!("sticks-linux-{}", arch);
	let download_url = format!("{}/{}", github::RELEASE_DOWNLOAD_URL, binary_name);
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

	println!();
	println!(
		"✓ Successfully upgraded from v{} to v{}!",
		current_version, latest_version
	);
	println!("💡 Run 'sticks --version' to verify the installation.");

	Ok(())
}
