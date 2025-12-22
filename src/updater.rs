use anyhow::Result;

#[cfg(target_os = "linux")]
mod platform {
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
				if !path.is_empty() {
					return Ok(PathBuf::from(path));
				}
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

		anyhow::bail!(
			"Could not determine sticks installation path. Try running 'which sticks' and ensure sticks is on your PATH."
		)
	}

	fn get_architecture() -> &'static str {
		#[cfg(target_arch = "x86_64")]
		return "x86_64";
		#[cfg(target_arch = "aarch64")]
		return "aarch64";
		#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
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
			.context(
				"Failed to fetch latest release information. Is 'curl' installed and can you reach api.github.com?",
			)?;

		if !output.status.success() {
			anyhow::bail!(
				"Failed to check for latest version (curl exit code {}).",
				output.status
			);
		}

		let json = String::from_utf8_lossy(&output.stdout);
		if json.trim().is_empty() {
			anyhow::bail!("Empty response from GitHub API (api.github.com)");
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
				"Unsupported architecture. Please update manually from {}",
				github::RELEASE_DOWNLOAD_URL
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
					.context(
						"Failed to download .deb package. Install curl, wget, or wget2 and try again.",
					)
			}?;

			if !status.success() {
				fs::remove_dir_all(&temp_dir).ok();
				anyhow::bail!(
					"Failed to download .deb package from {}. Please check your internet connection or download manually.",
					download_url
				);
			}

			println!("📦 Installing .deb package (requires sudo)...");
			let install_status = Command::new("sudo")
				.args(["dpkg", "-i"])
				.arg(&temp_deb)
				.status()
				.context("Failed to install .deb package (sudo dpkg -i ...)")?;

			fs::remove_dir_all(&temp_dir).ok();

			if !install_status.success() {
				anyhow::bail!(
					"Failed to install .deb package. You may need to run 'sudo apt --fix-broken install'"
				);
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

		let temp_binary_str = temp_binary
			.to_str()
			.with_context(|| format!("Failed to convert path to string: {:?}", temp_binary))?;

		let status = Command::new("curl")
			.args(["-L", "-f", "-o", temp_binary_str, &download_url])
			.status()
			.context("Failed to download update. Is curl installed?")?;

		if !status.success() {
			fs::remove_dir_all(&temp_dir).ok();
			anyhow::bail!(
				"Failed to download update from {}. Please check your internet connection or download manually.",
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
}

#[cfg(target_os = "windows")]
mod platform {
	use anyhow::{Context, Result};
	use std::env;
	use std::fs;
	use std::path::PathBuf;
	use std::process::Command;

	use crate::constants::github;

	fn download_latest_binary() -> Result<Vec<u8>> {
		let client = reqwest::blocking::Client::new();
		let response = client
			.get(format!(
				"{}/latest/download/sticks-windows-x86_64.exe",
				github::RELEASE_DOWNLOAD_URL
			))
			.send()
			.context("Failed to download the latest version")?;

		if !response.status().is_success() {
			anyhow::bail!("Failed to download update: {}", response.status());
		}

		let bytes = response.bytes()?.to_vec();
		if bytes.is_empty() {
			anyhow::bail!("Downloaded file is empty");
		}

		Ok(bytes)
	}

	fn get_install_path() -> Result<PathBuf> {
		// Try to find the binary in PATH
		if let Ok(output) = Command::new("where").arg("sticks").output() {
			if output.status.success() {
				let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
				if !path.is_empty() {
					return Ok(PathBuf::from(path));
				}
			}
		}

		// Common installation paths
		if let Some(mut path) = dirs::home_dir() {
			path.push("AppData\\Local\\Programs\\sticks\\sticks.exe");
			if path.exists() {
				return Ok(path);
			}

			path.pop();
			path.pop();
			path.push("scoop\\apps\\sticks\\current\\sticks.exe");
			if path.exists() {
				return Ok(path);
			}
		}

		// Fall back to current executable's directory
		if let Ok(exe_path) = env::current_exe() {
			return Ok(exe_path);
		}

		anyhow::bail!(
			"Could not determine sticks installation path. Please ensure sticks is in your PATH."
		)
	}

	pub fn update_project() -> Result<()> {
		println!("🔄 Checking for updates...");

		let current_version = crate::updater::get_current_version();
		let latest_version = crate::updater::get_latest_version()?;

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
		println!("\nDownloading update...");

		let binary_data = download_latest_binary()?;
		let install_path = get_install_path()?;
		let temp_path = install_path.with_extension("tmp");
		let backup_path = install_path.with_extension("bak");

		// Write new binary to temp file
		fs::write(&temp_path, &binary_data).context("Failed to write temporary file")?;

		// Try to replace the binary
		if let Err(e) = fs::rename(&install_path, &backup_path) {
			if e.kind() == std::io::ErrorKind::PermissionDenied {
				println!("\n⚠️  Permission denied when trying to update. Please run the command as administrator.");
				println!("\nYou can also manually download the latest version from:");
				println!("https://github.com/mAmineChniti/sticks/releases/latest");
				return Ok(());
			}
			return Err(e).context("Failed to create backup of current binary");
		}

		if let Err(e) = fs::rename(&temp_path, &install_path) {
			// Try to restore backup if update fails
			fs::rename(&backup_path, &install_path).ok();
			return Err(e).context("Failed to replace binary");
		}

		// Cleanup backup
		fs::remove_file(backup_path).ok();

		println!("\n✓ Successfully updated to v{}!", latest_version);
		println!("💡 Run 'sticks --version' to verify the installation.");

		Ok(())
	}
}

#[cfg(target_os = "macos")]
mod platform {
	use anyhow::{Context, Result};
	use std::process::Command;

	pub fn update_project() -> Result<()> {
		println!("🔄 Checking for updates...");

		// Check if installed via Homebrew
		let is_homebrew = Command::new("brew")
			.args(["list", "--formula"])
			.output()
			.map(|output| {
				String::from_utf8_lossy(&output.stdout)
					.lines()
					.any(|line| line.contains("sticks"))
			})
			.unwrap_or(false);

		if is_homebrew {
			println!("\n📦 Update available via Homebrew:");
			println!("  1. Run 'brew upgrade sticks' to update to the latest version");
			println!("\nOr install the latest binary manually:");
			println!("  https://github.com/mAmineChniti/sticks/releases/latest");
			return Ok(());
		}

		// Check if installed via Cargo
		let is_cargo = Command::new("cargo")
			.args(["install", "--list"])
			.output()
			.map(|output| {
				String::from_utf8_lossy(&output.stdout)
					.lines()
					.any(|line| line.trim().starts_with("sticks "))
			})
			.unwrap_or(false);

		if is_cargo {
			println!("\n📦 Update available via Cargo:");
			println!("  1. Run 'cargo install sticks --force' to update");
			println!("\nOr install via Homebrew for easier updates:");
			println!("  brew tap mAmineChniti/tap && brew install sticks");
			println!("\nOr download the latest binary manually:");
			println!("  https://github.com/mAmineChniti/sticks/releases/latest");
			return Ok(());
		}

		// Default update instructions
		let current_version = crate::updater::get_current_version();
		let latest_version = match crate::updater::get_latest_version() {
			Ok(ver) => ver,
			Err(_) => return Ok(()),
		};

		if current_version == latest_version {
			println!(
				"✓ You're already on the latest version (v{})!",
				current_version
			);
			return Ok(());
		}

		println!(
			"\n📦 Update available: v{} → v{}",
			current_version, latest_version
		);
		println!("\nRecommended update methods:");
		println!("  1. Install via Homebrew (recommended for easy updates):");
		println!("     brew tap mAmineChniti/tap && brew install sticks");
		println!("\n  2. Download the latest binary manually:");
		println!("     https://github.com/mAmineChniti/sticks/releases/latest");
		println!("     Make sure to replace the binary in your PATH.");

		Ok(())
	}
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
mod platform {
	use anyhow::Result;

	pub fn update_project() -> Result<()> {
		println!("🔄 Self-update is not supported on this platform.");
		println!(
			"Please update manually from: https://github.com/mAmineChniti/sticks/releases/latest"
		);
		Ok(())
	}
}

pub fn update_project() -> Result<()> {
	platform::update_project()
}
