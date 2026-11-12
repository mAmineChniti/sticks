use anyhow::{Context, Result};
use serde_json::Value;
use std::cmp::Ordering;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::constants::github;
#[cfg(unix)]
use crate::constants::install_paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Os {
	Linux,
	Macos,
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
			(Os::Macos, Self::X86_64 | Self::Aarch64) => {
				Ok(format!("sticks-macos-{}", self.release_name()))
			}
			(Os::Macos, Self::ArmV7) => {
				anyhow::bail!("No macOS release artifact is available for armv7")
			}
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
		// Homebrew locations (macOS Apple Silicon + Intel, Linuxbrew)
		candidates.push(PathBuf::from("/opt/homebrew/bin/sticks"));
		candidates.push(PathBuf::from("/usr/local/bin/sticks"));
		candidates.push(PathBuf::from("/home/linuxbrew/.linuxbrew/bin/sticks"));
		if let Some(home) = dirs::home_dir() {
			candidates.push(home.join(install_paths::CARGO_BIN_SUFFIX));
			candidates.push(home.join(".local/bin/sticks"));
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
	} else if cfg!(target_os = "macos") {
		Os::Macos
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
		.map_err(|error| {
			// A missing `curl` is by far the most common cause, and the raw
			// "No such file or directory" tells the user nothing.
			if error.kind() == std::io::ErrorKind::NotFound {
				anyhow::anyhow!(
					"curl is required to check for updates. Install it, or update through your package manager."
				)
			} else {
				anyhow::Error::new(error).context("Failed to run curl")
			}
		})?;

	if !output.status.success() {
		// `--fail` is deliberately omitted so the response body survives to
		// explain *why* the request failed (rate limit, auth, 404, ...).
		let detail = String::from_utf8_lossy(&output.stderr);
		let detail = detail.trim();
		anyhow::bail!(
			"Failed to fetch the latest release information{}{}",
			if detail.is_empty() { "" } else { ": " },
			detail
		);
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
		matches!(path.parent(), Some(parent) if {
			parent == Path::new("/usr/bin")
				|| parent == Path::new("/usr/local/bin")
				|| parent == Path::new("/opt/homebrew/bin")
				|| parent == Path::new("/home/linuxbrew/.linuxbrew/bin")
		})
	}
	#[cfg(not(unix))]
	{
		let _ = path;
		false
	}
}

fn is_homebrew_install(path: &Path) -> bool {
	path.starts_with("/opt/homebrew/Cellar/sticks")
		|| path.starts_with("/usr/local/Cellar/sticks")
		|| path.starts_with("/home/linuxbrew/.linuxbrew/Cellar/sticks")
		|| Command::new("brew")
			.args(["list", "sticks"])
			.output()
			.map(|o| o.status.success())
			.unwrap_or(false)
}

fn is_cargo_install(path: &Path) -> bool {
	path.ends_with(".cargo/bin/sticks") || path.ends_with(".cargo/bin/sticks.exe")
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

	// Never steer a user onto a prerelease. `/releases/latest` normally
	// excludes them, but if a prerelease were ever marked latest, comparing
	// `0.4.1` against `0.4.2-rc.1` would advertise an "update" that is a
	// downgrade in stability terms.
	if let Ok(latest) = parse_version(&latest_version)
		&& latest.prerelease.is_some()
	{
		println!(
			"ℹ️  The latest published release is a prerelease (v{}). Staying on v{}.",
			latest_version, current_version
		);
		return Ok(());
	}

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

	if is_homebrew_install(&install_path) {
		println!();
		println!("ℹ️  Homebrew installation detected.");
		println!("📦 Update with:");
		println!();
		println!("  brew upgrade sticks");
		return Ok(());
	}

	if is_cargo_install(&install_path) {
		println!();
		println!("ℹ️  Cargo installation detected.");
		println!("📦 Update with:");
		println!();
		println!("  cargo install sticks --force");
		return Ok(());
	}

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
				"Automatic installation of {} is disabled: package installation requires root and dpkg. Download it from the release page, verify it against SHA256SUMS, then install with dpkg.",
				package
			);
		}

		anyhow::bail!(
			"Automatic updates are disabled for this system installation. Use your distribution package manager, or install a release binary manually after verifying it against SHA256SUMS."
		);
	}

	let artifact_name = architecture.binary_name(os)?;
	update_binary(&install_path, &artifact_name, &latest_version)
}

/// Tag-pinned download base, e.g.
/// `https://github.com/mAmineChniti/sticks/releases/download/v0.4.2`.
fn release_download_base(version: &str) -> String {
	format!(
		"https://github.com/{}/{}/releases/download/v{}",
		github::REPO_OWNER,
		github::REPO_NAME,
		version
	)
}

fn download_file(url: &str, dest: &Path) -> Result<()> {
	let status = Command::new("curl")
		.args([
			"--silent",
			"--show-error",
			"--location",
			"--proto",
			"=https",
			"--tlsv1.2",
			"--max-time",
			"300",
			"--fail",
			"--output",
		])
		.arg(dest)
		.arg(url)
		.status()
		.map_err(|error| {
			if error.kind() == std::io::ErrorKind::NotFound {
				anyhow::anyhow!(
					"curl is required to download updates. Install it, or update through your package manager."
				)
			} else {
				anyhow::Error::new(error).context("Failed to run curl")
			}
		})?;
	if !status.success() {
		anyhow::bail!("Failed to download {} (curl exited with {})", url, status);
	}
	Ok(())
}

fn sha256_hex(path: &Path) -> Result<String> {
	use sha2::{Digest, Sha256};
	use std::io;
	let mut file =
		fs::File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
	let mut hasher = Sha256::new();
	io::copy(&mut file, &mut hasher).context("Failed to hash downloaded file")?;
	Ok(format!("{:x}", hasher.finalize()))
}

/// Parse `sha256sum` output into `(hash, filename)` pairs. Filenames are
/// normalized by stripping a leading `./`.
fn parse_sha256sums(content: &str) -> Vec<(String, String)> {
	content
		.lines()
		.filter_map(|line| {
			let mut parts = line.split_whitespace();
			let hash = parts.next()?;
			let name = parts.next()?.trim_start_matches("./");
			if hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()) {
				Some((hash.to_ascii_lowercase(), name.to_string()))
			} else {
				None
			}
		})
		.collect()
}

fn find_checksum<'a>(entries: &'a [(String, String)], artifact: &str) -> Option<&'a str> {
	entries
		.iter()
		.find(|(_, name)| name == artifact)
		.map(|(hash, _)| hash.as_str())
}

/// Verify `artifact_path` against the checksum recorded for `artifact_name`
/// in `sums_content`. This is the security boundary of `sticks update`:
/// the installed binary is only replaced after this returns `Ok`.
fn verify_artifact(sums_content: &str, artifact_path: &Path, artifact_name: &str) -> Result<()> {
	let entries = parse_sha256sums(sums_content);
	let expected = find_checksum(&entries, artifact_name).ok_or_else(|| {
		anyhow::anyhow!(
			"SHA256SUMS does not list {} — refusing to install an unverified artifact",
			artifact_name
		)
	})?;
	let actual = sha256_hex(artifact_path)?;
	if actual != expected {
		anyhow::bail!(
			"Checksum mismatch for {}: expected {}, got {}. The download may be corrupted or tampered with; the installed binary was NOT replaced.",
			artifact_name,
			expected,
			actual
		);
	}
	Ok(())
}

/// Download `artifact_name` for `version`, verify it against the published
/// SHA256SUMS, then atomically replace `install_path`. Any failure leaves
/// the installed binary untouched.
fn update_binary(install_path: &Path, artifact_name: &str, version: &str) -> Result<()> {
	let base = release_download_base(version);
	let work = env::temp_dir().join(format!(
		"sticks-update-{}-{}",
		std::process::id(),
		SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map(|d| d.as_nanos())
			.unwrap_or_default()
	));
	fs::create_dir_all(&work).context("Failed to create temporary download directory")?;

	let result = (|| -> Result<()> {
		let sums_path = work.join("SHA256SUMS");
		download_file(&format!("{base}/SHA256SUMS"), &sums_path)?;
		let sums = fs::read_to_string(&sums_path).context("Failed to read SHA256SUMS")?;

		let artifact_tmp = work.join(artifact_name);
		download_file(&format!("{base}/{artifact_name}"), &artifact_tmp)?;

		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			fs::set_permissions(&artifact_tmp, fs::Permissions::from_mode(0o755))
				.context("Failed to mark downloaded binary executable")?;
		}

		verify_artifact(&sums, &artifact_tmp, artifact_name)?;

		// Verified: back up the current binary, then replace it.
		crate::file_handler::backup_existing(install_path)?;
		#[cfg(target_os = "windows")]
		if install_path.exists() {
			fs::remove_file(install_path)
				.with_context(|| format!("Failed to replace {}", install_path.display()))?;
		}
		fs::rename(&artifact_tmp, install_path).with_context(|| {
			format!(
				"Failed to install {} to {}",
				artifact_name,
				install_path.display()
			)
		})?;
		println!("✓ Updated sticks to v{}", version);
		println!(
			"  Previous binary backed up to {}",
			crate::file_handler::backup_path(install_path).display()
		);
		Ok(())
	})();

	let _ = fs::remove_dir_all(&work);
	result
}

#[cfg(test)]
mod tests {
	use super::*;

	fn order(current: &str, latest: &str) -> Ordering {
		compare_versions(current, latest).expect("versions must parse")
	}

	#[test]
	fn numeric_core_compares_numerically_not_lexically() {
		// The bug this guards: "0.10.0" < "0.9.0" under string comparison.
		assert_eq!(order("0.9.0", "0.10.0"), Ordering::Less);
		assert_eq!(order("0.10.0", "0.9.0"), Ordering::Greater);
		assert_eq!(order("0.4.1", "0.4.10"), Ordering::Less);
		assert_eq!(order("1.9.0", "1.10.0"), Ordering::Less);
	}

	#[test]
	fn equal_versions_compare_equal() {
		assert_eq!(order("0.4.1", "0.4.1"), Ordering::Equal);
		assert_eq!(order("1.0.0", "1.0.0+build.5"), Ordering::Equal);
	}

	#[test]
	fn prerelease_sorts_below_its_release() {
		assert_eq!(order("1.0.0-rc.1", "1.0.0"), Ordering::Less);
		assert_eq!(order("1.0.0", "1.0.0-rc.1"), Ordering::Greater);
	}

	#[test]
	fn prerelease_identifiers_order_correctly() {
		assert_eq!(order("1.0.0-rc.1", "1.0.0-rc.2"), Ordering::Less);
		assert_eq!(order("1.0.0-rc.2", "1.0.0-rc.10"), Ordering::Less);
		// A numeric identifier ranks below an alphanumeric one.
		assert_eq!(order("1.0.0-1", "1.0.0-alpha"), Ordering::Less);
		// More fields wins when the shared prefix is equal.
		assert_eq!(order("1.0.0-rc", "1.0.0-rc.1"), Ordering::Less);
	}

	#[test]
	fn rejects_malformed_versions() {
		for bad in [
			"",
			"not-a-version",
			"1.2.3.4",          // more than three core components
			"1..2",             // empty component
			"01.2.3",           // leading zero
			"1.2.3-",           // empty prerelease
			"1.2.3-01",         // leading zero in numeric identifier
			"1.2.3-alpha_beta", // `_` is not a valid identifier character
		] {
			assert!(
				parse_version(bad).is_err(),
				"expected `{bad}` to be rejected"
			);
		}
	}

	#[test]
	fn accepts_short_and_padded_version_forms() {
		// Tags like `v1` are accepted and compare as the padded equivalent.
		assert_eq!(order("1", "1.0.0"), Ordering::Equal);
		assert_eq!(order("1.2", "1.2.0"), Ordering::Equal);
		assert_eq!(order("v1.2.3", "1.2.3"), Ordering::Equal);
	}

	#[test]
	fn accepts_well_formed_versions() {
		for good in [
			"0.0.4",
			"1.2.3",
			"0.4.1-rc.1",
			"1.0.0-alpha.1",
			"1.0.0-0.3.7",
			"1.0.0+build.1",
			"10.20.30",
		] {
			assert!(parse_version(good).is_ok(), "expected `{good}` to parse");
		}
	}

	#[test]
	fn prerelease_is_detected() {
		assert!(parse_version("1.0.0-rc.1").unwrap().prerelease.is_some());
		assert!(parse_version("1.0.0").unwrap().prerelease.is_none());
	}

	#[test]
	fn validate_install_path_rejects_unrelated_files() {
		let dir = tempfile::tempdir().unwrap();
		let decoy = dir.path().join("not-sticks");
		std::fs::write(&decoy, "#!/bin/sh\n").unwrap();
		assert!(validate_install_path(&decoy).is_err());
	}

	#[test]
	fn parse_sha256sums_normalizes_filenames() {
		let content = "abc123  ./sticks-linux-x86_64\ndef456  sticks-macos-aarch64\n";
		let entries = parse_sha256sums(content);
		// Short hashes are rejected.
		assert!(entries.is_empty());
		let hash = "a".repeat(64);
		let content = format!("{hash}  ./sticks-linux-x86_64\n");
		let entries = parse_sha256sums(&content);
		assert_eq!(entries.len(), 1);
		assert_eq!(entries[0].1, "sticks-linux-x86_64");
	}

	#[test]
	fn verify_artifact_accepts_matching_checksum() {
		let dir = tempfile::tempdir().unwrap();
		let artifact = dir.path().join("sticks-linux-x86_64");
		std::fs::write(&artifact, "real binary bytes").unwrap();
		let sum = sha256_hex(&artifact).unwrap();
		let sums = format!("{sum}  ./sticks-linux-x86_64\n");
		assert!(verify_artifact(&sums, &artifact, "sticks-linux-x86_64").is_ok());
	}

	#[test]
	fn verify_artifact_rejects_tampered_binary() {
		let dir = tempfile::tempdir().unwrap();
		let artifact = dir.path().join("sticks-linux-x86_64");
		std::fs::write(&artifact, "MALICIOUS binary").unwrap();
		// Checksum belongs to a different (honest) payload.
		let honest = dir.path().join("honest");
		std::fs::write(&honest, "real binary bytes").unwrap();
		let sum = sha256_hex(&honest).unwrap();
		let sums = format!("{sum}  ./sticks-linux-x86_64\n");
		let error = verify_artifact(&sums, &artifact, "sticks-linux-x86_64").unwrap_err();
		assert!(error.to_string().contains("Checksum mismatch"));
	}

	#[test]
	fn verify_artifact_rejects_tampered_checksum_file() {
		let dir = tempfile::tempdir().unwrap();
		let artifact = dir.path().join("sticks-linux-x86_64");
		std::fs::write(&artifact, "real binary bytes").unwrap();
		// Attacker-controlled checksum that does not match the artifact.
		let sums = format!("{}  ./sticks-linux-x86_64\n", "b".repeat(64));
		assert!(verify_artifact(&sums, &artifact, "sticks-linux-x86_64").is_err());
	}

	#[test]
	fn verify_artifact_rejects_missing_entry() {
		let dir = tempfile::tempdir().unwrap();
		let artifact = dir.path().join("sticks-linux-x86_64");
		std::fs::write(&artifact, "real binary bytes").unwrap();
		let sums = format!("{}  ./sticks-windows-x86_64.exe\n", "a".repeat(64));
		let error = verify_artifact(&sums, &artifact, "sticks-linux-x86_64").unwrap_err();
		assert!(error.to_string().contains("does not list"));
	}
}
