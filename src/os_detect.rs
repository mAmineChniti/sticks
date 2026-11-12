use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemPackageManager {
	Apt,
	Dnf,
	Pacman,
	Apk,
	Zypper,
	Brew,
	Choco,
	Scoop,
	Nix,
	Unknown,
}

/// Detects the system package manager by probing common commands.
/// Probe order is Linux-first (apt > dnf > pacman > apk > zypper) then
/// cross-platform (brew, choco, scoop, nix). On systems with several
/// managers installed the first match wins; use `detect_all_system_package_managers`
/// if you need the full list.
pub fn detect_system_package_manager() -> SystemPackageManager {
	detect_all_system_package_managers()
		.into_iter()
		.next()
		.unwrap_or(SystemPackageManager::Unknown)
}

/// Returns every supported package manager found on `PATH`, in probe order.
pub fn detect_all_system_package_managers() -> Vec<SystemPackageManager> {
	let mut found = Vec::new();
	if command_exists("apt") || command_exists("apt-get") {
		found.push(SystemPackageManager::Apt);
	}
	if command_exists("dnf") {
		found.push(SystemPackageManager::Dnf);
	}
	if command_exists("pacman") {
		found.push(SystemPackageManager::Pacman);
	}
	if command_exists("apk") {
		found.push(SystemPackageManager::Apk);
	}
	if command_exists("zypper") {
		found.push(SystemPackageManager::Zypper);
	}
	if command_exists("brew") {
		found.push(SystemPackageManager::Brew);
	}
	if command_exists("choco") {
		found.push(SystemPackageManager::Choco);
	}
	if command_exists("scoop") {
		found.push(SystemPackageManager::Scoop);
	}
	if command_exists("nix-env") || command_exists("nix") {
		found.push(SystemPackageManager::Nix);
	}
	found
}

fn command_exists(cmd: &str) -> bool {
	// `--version` works for most managers; choco/scoop print version with
	// no args too, but tolerate either. Avoid PATH lookup cost duplication
	// by trying `--version` first then bare invocation metadata.
	for args in [&["--version"][..], &[][..]] {
		let result = Command::new(cmd).args(args).output();
		if let Ok(output) = result
			&& output.status.success()
		{
			return true;
		}
	}
	// Fall back to `which`/`where` style lookup when version probes fail
	// (e.g. minimal containers where `--version` needs a display).
	#[cfg(unix)]
	{
		if Command::new("sh")
			.args(["-c", &format!("command -v {}", cmd)])
			.output()
			.map(|o| o.status.success() && !o.stdout.is_empty())
			.unwrap_or(false)
		{
			return true;
		}
	}
	#[cfg(windows)]
	{
		if Command::new("where")
			.arg(cmd)
			.output()
			.map(|o| o.status.success())
			.unwrap_or(false)
		{
			return true;
		}
	}
	false
}

impl SystemPackageManager {
	/// Human-readable name used in messages.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Apt => "apt",
			Self::Dnf => "dnf",
			Self::Pacman => "pacman",
			Self::Apk => "apk",
			Self::Zypper => "zypper",
			Self::Brew => "brew",
			Self::Choco => "choco",
			Self::Scoop => "scoop",
			Self::Nix => "nix",
			Self::Unknown => "unknown",
		}
	}
}

/// Returns the preferred install command prefix for the system package manager
/// e.g. "sudo apt install -y"
/// Returns `None` when no supported package manager is detected so callers
/// don't silently emit a wrong install command (previously `Unknown` fell
/// back to apt, which is misleading on e.g. macOS/Windows).
pub fn install_command_prefix_opt() -> Option<String> {
	match detect_system_package_manager() {
		SystemPackageManager::Apt => Some("sudo apt install -y".to_string()),
		SystemPackageManager::Dnf => Some("sudo dnf install -y".to_string()),
		SystemPackageManager::Pacman => Some("sudo pacman -S --noconfirm".to_string()),
		SystemPackageManager::Apk => Some("sudo apk add".to_string()),
		SystemPackageManager::Zypper => Some("sudo zypper install -y".to_string()),
		SystemPackageManager::Brew => Some("brew install".to_string()),
		SystemPackageManager::Choco => Some("choco install -y".to_string()),
		SystemPackageManager::Scoop => Some("scoop install".to_string()),
		SystemPackageManager::Nix => Some("nix-env -iA nixpkgs.".to_string()),
		SystemPackageManager::Unknown => None,
	}
}

/// Builds a complete install command for `packages`.
///
/// Most managers take a space before the package list, but `nix-env -iA`
/// expects the attribute appended directly (`nix-env -iA nixpkgs.curl`), so a
/// naive `"<prefix> <package>"` join produces a command that does not run.
pub fn install_command_for(packages: &[String]) -> Option<String> {
	let prefix = install_command_prefix_opt()?;
	Some(if prefix.ends_with('.') {
		format!("{}{}", prefix, packages.join(" "))
	} else {
		format!("{} {}", prefix, packages.join(" "))
	})
}

/// Returns the preferred install command prefix for the system package manager
/// e.g. "sudo apt install -y"
pub fn install_command_prefix() -> String {
	install_command_prefix_opt().unwrap_or_else(|| "sudo apt install -y".to_string())
}
