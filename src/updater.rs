use anyhow::{Context, Result};
use serde_json::Value;
use std::cmp::Ordering;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::constants::{github, install_paths};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Os {
	Linux,
	Windows,
	Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Architecture {
	X86_64,
	Aarch64,
	ArmV7,
}

impl Architecture {
	fn release_name(self) -> &'static str {
		match self {
			Self::X86_64 => "x86_64",
			Self::Aarch64 => "aarch64",
			Self::ArmV7 => "armv7",
		}
	}

	fn debian_name(self) -> &'static str {
		match self {
			Self::X86_64 => "amd64",
			Self::Aarch64 => "arm64",
			Self::ArmV7 => "armhf",
		}
	}

	fn binary_name(self, os: Os) -> Result<String> {
		match (os, self) {
			(Os::Linux, _) => Ok(format!("sticks-linux-{}", self.release_name())),
			(Os::Windows, Self::X86_64 | Self::Aarch64) => {
				Ok(format!("sticks-windows-{}.exe", self.release_name()))
			}
			(Os::Windows, Self::ArmV7) => {
				anyhow::bail!("No Windows release artifact is available for armv7")
			}
			(Os::Unknown, _) => anyhow::bail!("Unsupported operating system"),
		}
	}
}

fn get_install_path() -> Result<PathBuf> {
	if let Ok(current_exe) = env::current_exe() {
		return validate_install_path(&current_exe).with_context(|| {
			format!(
				"The current executable is not a safe sticks installation: {:?}",
				current_exe
			)
		});
	}

	let mut candidates = Vec::new();
	#[cfg(unix)]
	{
		candidates.push(PathBuf::from(install_paths::USR_LOCAL_BIN));
		candidates.push(PathBuf::from(install_paths::USR_BIN));
		if let Some(home) = dirs::home_dir() {
			candidates.push(home.join(install_paths::CARGO_BIN_SUFFIX));
		}
	}
	#[cfg(target_os = "windows")]
	{
		if let Some(local_app_data) = dirs::data_local_dir() {
			candidates.push(local_app_data.join("sticks").join("sticks.exe"));
		}
		if let Some(home) = dirs::home_dir() {
			candidates.push(home.join(".cargo").join("bin").join("sticks.exe"));
		}
	}

	for candidate in candidates {
		if let Ok(path) = validate_install_path(&candidate) {
			return Ok(path);
		}
	}

	anyhow::bail!("Could not determine a safe sticks installation path")
}

fn validate_install_path(path: &Path) -> Result<PathBuf> {
	if !path.is_absolute() {
		anyhow::bail!("The sticks installation path must be absolute");
	}

	let canonical_path =
		fs::canonicalize(path).context("Failed to resolve sticks installation path")?;
	let metadata =
		fs::metadata(&canonical_path).context("Failed to inspect sticks installation path")?;
	if !metadata.is_file() {
		anyhow::bail!("The sticks installation path is not a regular file");
	}

	let file_name = canonical_path
		.file_name()
		.and_then(|name| name.to_str())
		.context("The sticks installation path has no valid file name")?;
	let expected_name = if cfg!(target_os = "windows") {
		"sticks.exe"
	} else {
		"sticks"
	};
	if !file_name.eq_ignore_ascii_case(expected_name) {
		anyhow::bail!("Unexpected sticks executable name: {}", file_name);
	}

	#[cfg(unix)]
	if metadata.permissions().mode() & 0o111 == 0 {
		anyhow::bail!("The sticks installation path is not executable");
	}

	Ok(canonical_path)
}

fn get_os() -> Os {
	if cfg!(target_os = "linux") {
		Os::Linux
	} else if cfg!(target_os = "windows") {
		Os::Windows
	} else {
		Os::Unknown
	}
}

fn get_architecture() -> Result<Architecture> {
	if cfg!(target_arch = "x86_64") {
		Ok(Architecture::X86_64)
	} else if cfg!(target_arch = "aarch64") {
		Ok(Architecture::Aarch64)
	} else if cfg!(target_arch = "arm") {
		Ok(Architecture::ArmV7)
	} else {
		anyhow::bail!("Unsupported architecture. Please update manually.")
	}
}

fn get_current_version() -> String {
	env!("CARGO_PKG_VERSION").to_string()
}

fn get_latest_version() -> Result<String> {
	let output = Command::new("curl")
		.args([
			"--fail",
			"--silent",
			"--show-error",
			"--location",
			"--proto",
			"=https",
			"--tlsv1.2",
			"--max-time",
			"30",
			"-H",
			"Accept: application/vnd.github.v3+json",
			github::RELEASE_API_URL,
		])
		.output()
		.context("Failed to fetch latest release information")?;

	if !output.status.success() {
		anyhow::bail!("Failed to check for the latest version");
	}

	let response: Value = serde_json::from_slice(&output.stdout)
		.context("GitHub returned invalid release metadata")?;
	let tag_name = response
		.get("tag_name")
		.and_then(Value::as_str)
		.ok_or_else(|| anyhow::anyhow!("GitHub release metadata did not contain tag_name"))?;
	let version = tag_name.strip_prefix('v').unwrap_or(tag_name);
	parse_version(version).context("GitHub returned an invalid release version")?;
	Ok(version.to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedVersion {
	core: [u64; 3],
	prerelease: Option<Vec<VersionIdentifier>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum VersionIdentifier {
	Number(u64),
	Text(String),
}

fn parse_version(value: &str) -> Result<ParsedVersion> {
	let value = value.strip_prefix('v').unwrap_or(value);
	if value.is_empty() || value.len() > 128 {
		anyhow::bail!("Invalid version");
	}

	let (version_and_prerelease, build_metadata) = value.split_once('+').unwrap_or((value, ""));
	if !build_metadata.is_empty() {
		validate_build_metadata(build_metadata)?;
	}

	let (core_text, prerelease_text) = match version_and_prerelease.split_once('-') {
		Some((core, prerelease)) => (core, Some(prerelease)),
		None => (version_and_prerelease, None),
	};
	let core_components: Vec<&str> = core_text.split('.').collect();
	if core_components.is_empty() || core_components.len() > 3 {
		anyhow::bail!("Invalid version");
	}

	let mut core = [0; 3];
	for (index, component) in core_components.iter().enumerate() {
		if component.is_empty()
			|| component.len() > 1 && component.starts_with('0')
			|| !component.bytes().all(|byte| byte.is_ascii_digit())
		{
			anyhow::bail!("Invalid version");
		}
		core[index] = component
			.parse::<u64>()
			.map_err(|_| anyhow::anyhow!("Invalid version component"))?;
	}

	let prerelease = match prerelease_text {
		Some("") => anyhow::bail!("Invalid prerelease version"),
		Some(text) => {
			let mut identifiers = Vec::new();
			for identifier in text.split('.') {
				identifiers.push(parse_prerelease_identifier(identifier)?);
			}
			Some(identifiers)
		}
		None => None,
	};

	Ok(ParsedVersion { core, prerelease })
}

fn validate_build_metadata(metadata: &str) -> Result<()> {
	for identifier in metadata.split('.') {
		if identifier.is_empty()
			|| identifier.len() > 128
			|| !identifier
				.bytes()
				.all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
		{
			anyhow::bail!("Invalid version build metadata");
		}
	}
	Ok(())
}

fn parse_prerelease_identifier(identifier: &str) -> Result<VersionIdentifier> {
	if identifier.is_empty()
		|| identifier.len() > 128
		|| !identifier
			.bytes()
			.all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
	{
		anyhow::bail!("Invalid prerelease version");
	}

	if identifier.bytes().all(|byte| byte.is_ascii_digit()) {
		if identifier.len() > 1 && identifier.starts_with('0') {
			anyhow::bail!("Invalid prerelease version");
		}
		let number = identifier
			.parse::<u64>()
			.map_err(|_| anyhow::anyhow!("Invalid prerelease version"))?;
		Ok(VersionIdentifier::Number(number))
	} else {
		Ok(VersionIdentifier::Text(identifier.to_owned()))
	}
}

fn compare_versions(current: &str, latest: &str) -> Result<Ordering> {
	let current = parse_version(current)?;
	let latest = parse_version(latest)?;
	Ok(compare_parsed_versions(&current, &latest))
}

fn compare_parsed_versions(current: &ParsedVersion, latest: &ParsedVersion) -> Ordering {
	for index in 0..current.core.len() {
		match current.core[index].cmp(&latest.core[index]) {
			Ordering::Equal => {}
			ordering => return ordering,
		}
	}

	match (&current.prerelease, &latest.prerelease) {
		(None, None) => Ordering::Equal,
		(None, Some(_)) => Ordering::Greater,
		(Some(_), None) => Ordering::Less,
		(Some(current), Some(latest)) => compare_prerelease(current, latest),
	}
}

fn compare_prerelease(current: &[VersionIdentifier], latest: &[VersionIdentifier]) -> Ordering {
	let mut index = 0;
	while index < current.len() && index < latest.len() {
		match compare_prerelease_identifier(&current[index], &latest[index]) {
			Ordering::Equal => index += 1,
			ordering => return ordering,
		}
	}

	current.len().cmp(&latest.len())
}

fn compare_prerelease_identifier(
	current: &VersionIdentifier,
	latest: &VersionIdentifier,
) -> Ordering {
	match (current, latest) {
		(VersionIdentifier::Number(current), VersionIdentifier::Number(latest)) => {
			current.cmp(latest)
		}
		(VersionIdentifier::Number(_), VersionIdentifier::Text(_)) => Ordering::Less,
		(VersionIdentifier::Text(_), VersionIdentifier::Number(_)) => Ordering::Greater,
		(VersionIdentifier::Text(current), VersionIdentifier::Text(latest)) => current.cmp(latest),
	}
}

fn is_system_install(path: &Path) -> bool {
	#[cfg(unix)]
	{
		matches!(
			path.parent(),
			Some(parent)
				if parent == Path::new("/usr/bin") || parent == Path::new("/usr/local/bin")
		)
	}
	#[cfg(not(unix))]
	{
		let _ = path;
		false
	}
}

#[cfg(unix)]
fn is_aur_install(path: &Path) -> bool {
	path == Path::new("/usr/bin/sticks")
		&& Path::new("/usr/bin/pacman").is_file()
		&& Command::new("/usr/bin/pacman")
			.args(["-Qi", "sticks-aur"])
			.output()
			.map(|output| output.status.success())
			.unwrap_or(false)
}

#[cfg(not(unix))]
fn is_aur_install(_path: &Path) -> bool {
	false
}

pub fn update_project() -> Result<()> {
	println!("🔄 Checking for updates...");

	let current_version = get_current_version();
	let os = get_os();
	if os == Os::Unknown {
		anyhow::bail!("Unsupported operating system. Please update manually.");
	}
	let architecture = get_architecture()?;
	let install_path = get_install_path()?;
	let latest_version = get_latest_version().context("Failed to check for updates")?;

	match compare_versions(&current_version, &latest_version)
		.context("Failed to compare sticks versions")?
	{
		Ordering::Equal => {
			println!(
				"✓ You're already on the latest version (v{})!",
				current_version
			);
			return Ok(());
		}
		Ordering::Greater => {
			println!(
				"✓ You're running a newer version (v{}) than the published release (v{}).",
				current_version, latest_version
			);
			return Ok(());
		}
		Ordering::Less => {}
	}

	println!(
		"📦 Update available: v{} → v{}",
		current_version, latest_version
	);

	if os == Os::Linux && is_system_install(&install_path) {
		if is_aur_install(&install_path) {
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

		if architecture == Architecture::X86_64 && Path::new("/usr/bin/dpkg").is_file() {
			let package = format!(
				"sticks_{}-1_{}.deb",
				latest_version,
				architecture.debian_name()
			);
			anyhow::bail!(
				"Automatic installation of {} is disabled because sticks cannot verify GitHub release artifacts. Install it with your distribution package manager or verify the package before installing it manually.",
				package
			);
		}

		anyhow::bail!(
			"Automatic updates are disabled for this system installation. Use your distribution package manager or install a verified release manually."
		);
	}

	let artifact_name = architecture.binary_name(os)?;
	anyhow::bail!(
		"Automatic update is disabled because sticks cannot verify GitHub release artifacts. Install v{} from a trusted package source and verify {} before replacing {:?}.",
		latest_version,
		artifact_name,
		install_path
	)
}
