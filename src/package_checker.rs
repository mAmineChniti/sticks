use crate::os_detect::SystemPackageManager;
use anyhow::Result;
use std::process::Command;
use strsim::levenshtein;

fn validate_package_name(name: &str) -> Result<()> {
	if name.is_empty()
		|| name.len() > 128
		|| name.starts_with('-')
		|| name.contains(['*', '?', '/', '[', ']', '^'])
		|| name
			.chars()
			.any(|character| character.is_control() || character.is_whitespace())
	{
		anyhow::bail!("Invalid package name: {}", name);
	}
	Ok(())
}

pub fn package_exists(name: &str, pm: SystemPackageManager) -> Result<bool> {
	validate_package_name(name)?;
	match pm {
		SystemPackageManager::Apt => {
			let output = Command::new("apt-cache").arg("policy").arg(name).output()?;
			if !output.status.success() {
				return Ok(false);
			}
			if output.stdout.is_empty() && !output.stderr.is_empty() {
				return Ok(false);
			}
			let text = String::from_utf8_lossy(&output.stdout);
			Ok(text.lines().any(|line| {
				line.trim_start().starts_with("Candidate:") && !line.contains("(none)")
			}) || text.lines().any(|line| line.starts_with("Package:")))
		}
		SystemPackageManager::Dnf => {
			let out = Command::new("dnf").arg("list").arg(name).output();
			if let Ok(o) = out {
				if !o.status.success() {
					return Ok(false);
				}
				// `dnf list` can succeed with no matches; verify output mentions the package.
				let text = String::from_utf8_lossy(&o.stdout).to_lowercase();
				let needle = name.to_lowercase();
				return Ok(text.lines().any(|line| {
					let trimmed = line.trim();
					!trimmed.is_empty()
						&& !trimmed.starts_with('=')
						&& trimmed.split_whitespace().next().is_some_and(|pkg| {
							pkg.split('.')
								.next()
								.unwrap_or(pkg)
								.eq_ignore_ascii_case(&needle)
								|| pkg.eq_ignore_ascii_case(&needle)
						})
				}));
			}
			Ok(false)
		}
		SystemPackageManager::Pacman => {
			let out = Command::new("pacman").arg("-Si").arg(name).output();
			if let Ok(o) = out {
				return Ok(o.status.success());
			}
			Ok(false)
		}
		SystemPackageManager::Apk => {
			let out = Command::new("apk")
				.arg("search")
				.arg("-e")
				.arg(name)
				.output();
			if let Ok(output) = out
				&& output.status.success()
			{
				let text = String::from_utf8_lossy(&output.stdout);
				return Ok(text.lines().any(|line| {
					line.split_whitespace().next().is_some_and(|package| {
						package == name || strip_apk_version(package) == Some(name)
					})
				}));
			}
			Ok(false)
		}
		SystemPackageManager::Zypper => {
			let out = Command::new("zypper")
				.arg("se")
				.arg("-x")
				.arg(name)
				.output();
			if let Ok(o) = out {
				if !o.status.success() {
					return Ok(false);
				}
				// `zypper se` succeeds even with zero matches; check for an exact name hit.
				let text = String::from_utf8_lossy(&o.stdout);
				return Ok(text.lines().any(|line| {
					let fields = line.split('|').map(str::trim).collect::<Vec<_>>();
					fields
						.get(1)
						.is_some_and(|value| value.eq_ignore_ascii_case(name))
				}));
			}
			Ok(false)
		}
		SystemPackageManager::Brew => {
			let out = Command::new("brew").args(["info", name]).output();
			if let Ok(o) = out {
				if !o.status.success() {
					return Ok(false);
				}
				let text = String::from_utf8_lossy(&o.stdout);
				// brew prints "No available formula ..." on stderr but may still
				// succeed; require the formula name to appear in output.
				if text.to_lowercase().contains("no available formula") {
					return Ok(false);
				}
				// Match the name explicitly. A non-empty `brew info` output is
				// not proof of existence: warning and deprecation banners are
				// printed for unknown formulae and would make this always true.
				return Ok(text.lines().any(|line| {
					let lower = line.to_lowercase();
					lower.starts_with(&format!("{}:", name.to_lowercase()))
						|| lower.trim() == name.to_lowercase()
						|| lower.contains(&format!("==> {}", name.to_lowercase()))
				}));
			}
			Ok(false)
		}
		SystemPackageManager::Choco => {
			let out = Command::new("choco")
				.args(["search", "--exact", "--limit-output", name])
				.output();
			if let Ok(o) = out {
				if !o.status.success() {
					return Ok(false);
				}
				let text = String::from_utf8_lossy(&o.stdout);
				return Ok(text.lines().any(|line| {
					let first = line.split('|').next().unwrap_or("").trim();
					first.eq_ignore_ascii_case(name)
				}));
			}
			Ok(false)
		}
		SystemPackageManager::Scoop => {
			let out = Command::new("scoop").args(["search", name]).output();
			if let Ok(o) = out {
				if !o.status.success() {
					return Ok(false);
				}
				let text = String::from_utf8_lossy(&o.stdout);
				return Ok(text.lines().any(|line| {
					line.split_whitespace().next().is_some_and(|pkg| {
						// scoop search prints "Name Version ..." table
						pkg.eq_ignore_ascii_case(name)
					})
				}));
			}
			Ok(false)
		}
		SystemPackageManager::Nix => {
			let out = Command::new("nix-env").args(["-qaP", name]).output();
			if let Ok(o) = out {
				if !o.status.success() {
					return Ok(false);
				}
				let text = String::from_utf8_lossy(&o.stdout);
				return Ok(text.lines().any(|line| {
					line.split_whitespace().any(|token| {
						// nixpkgs attribute paths look like `nixpkgs.curl`
						token
							.rsplit('.')
							.next()
							.unwrap_or(token)
							.eq_ignore_ascii_case(name)
							|| token.eq_ignore_ascii_case(name)
					})
				}));
			}
			Ok(false)
		}
		SystemPackageManager::Unknown => Ok(false),
	}
}

fn strip_apk_version(package: &str) -> Option<&str> {
	// apk versions look like `foo-1.2.3-r0`: strip from the first `-<digit>`.
	let mut search_from = 0;
	while let Some(dash) = package[search_from..].find('-') {
		let index = search_from + dash;
		if package[index + 1..]
			.chars()
			.next()
			.is_some_and(|c| c.is_ascii_digit())
		{
			return Some(&package[..index]);
		}
		search_from = index + 1;
	}
	None
}

fn parse_simple_search_output(cmd: &str, args: &[&str], pm: SystemPackageManager) -> Vec<String> {
	let Ok(output) = Command::new(cmd).args(args).output() else {
		return Vec::new();
	};
	if !output.status.success() {
		return Vec::new();
	}
	let text = String::from_utf8_lossy(&output.stdout);
	text.lines()
		.filter_map(|line| parse_search_line(line, pm))
		.collect()
}

fn parse_search_line(line: &str, pm: SystemPackageManager) -> Option<String> {
	let trimmed = line.trim();
	if trimmed.is_empty() || trimmed.starts_with('=') || trimmed.starts_with("Name") {
		return None;
	}
	if matches!(pm, SystemPackageManager::Zypper) {
		let fields = trimmed.split('|').map(str::trim).collect::<Vec<_>>();
		return fields
			.get(1)
			.filter(|value| !value.is_empty() && **value != "Name")
			.map(|value| value.to_string());
	}
	if matches!(pm, SystemPackageManager::Choco) {
		// `choco search --limit-output` prints `name|version`
		let first = trimmed.split('|').next().unwrap_or("").trim();
		if first.is_empty() || first == "-" {
			return None;
		}
		return Some(first.to_string());
	}
	if matches!(pm, SystemPackageManager::Nix) {
		// `nix-env -qaP` prints `attrpath  name`
		let mut parts = trimmed.split_whitespace();
		if let Some(attr) = parts.next() {
			let name = attr.rsplit('.').next().unwrap_or(attr);
			if !name.is_empty() && name != "-" {
				return Some(name.to_string());
			}
		}
		return None;
	}
	if matches!(pm, SystemPackageManager::Brew | SystemPackageManager::Scoop) {
		let value = trimmed.split_whitespace().next()?;
		if value == "-" || value.starts_with("==>") {
			return None;
		}
		return Some(value.to_string());
	}
	let value = trimmed
		.split_whitespace()
		.next()
		.filter(|value| *value != "-")?;
	if matches!(pm, SystemPackageManager::Apk)
		&& let Some(name) = strip_apk_version(value)
	{
		return Some(name.to_string());
	}
	Some(value.to_string())
}

fn find_candidates(name: &str, pm: SystemPackageManager) -> Vec<String> {
	if validate_package_name(name).is_err() {
		return Vec::new();
	}
	let mut candidates: Vec<String> = Vec::new();

	match pm {
		SystemPackageManager::Apt => {
			candidates = parse_simple_search_output("apt-cache", &["search", name], pm);
		}
		SystemPackageManager::Pacman => {
			let search = Command::new("pacman").arg("-Ss").arg(name).output();
			if let Ok(o) = search
				&& o.status.success()
			{
				let out = String::from_utf8_lossy(&o.stdout).to_string();
				for line in out.lines() {
					if !line.trim().is_empty() {
						// Extract package name after '/' if present
						if let Some(after_slash) = line.split('/').nth(1)
							&& let Some(pkg) = after_slash.split_whitespace().next()
						{
							candidates.push(pkg.to_string());
						}
					}
				}
			}
		}
		SystemPackageManager::Dnf => {
			let search = Command::new("dnf").arg("search").arg(name).output();
			if let Ok(o) = search
				&& o.status.success()
			{
				let out = String::from_utf8_lossy(&o.stdout).to_string();
				for line in out.lines() {
					let trimmed = line.trim();
					// Skip header lines and separators (e.g., "====", "Name", "Summary")
					if trimmed.is_empty()
						|| trimmed.starts_with('=')
						|| trimmed.starts_with("Name")
						|| trimmed.starts_with("Summary")
					{
						continue;
					}
					// Parse "name.arch : description" format, extract only package name without .arch
					if let Some(colon_pos) = trimmed.find(':') {
						let pkg_part = &trimmed[..colon_pos].trim();
						if let Some(dot_pos) = pkg_part.rfind('.') {
							// Remove .arch suffix
							candidates.push(pkg_part[..dot_pos].to_string());
						} else {
							candidates.push(pkg_part.to_string());
						}
					}
				}
			}
		}
		SystemPackageManager::Apk => {
			candidates = parse_simple_search_output("apk", &["search", name], pm);
		}
		SystemPackageManager::Zypper => {
			candidates = parse_simple_search_output("zypper", &["search", name], pm);
		}
		SystemPackageManager::Brew => {
			candidates = parse_simple_search_output("brew", &["search", name], pm);
		}
		SystemPackageManager::Choco => {
			candidates =
				parse_simple_search_output("choco", &["search", "--limit-output", name], pm);
		}
		SystemPackageManager::Scoop => {
			candidates = parse_simple_search_output("scoop", &["search", name], pm);
		}
		SystemPackageManager::Nix => {
			candidates = parse_simple_search_output("nix-env", &["-qaP", name], pm);
		}
		SystemPackageManager::Unknown => {}
	}

	candidates
}

pub fn suggest_similar(name: &str, pm: SystemPackageManager) -> Result<Vec<String>> {
	let mut candidates = find_candidates(name, pm);

	// Deduplicate and score
	candidates.sort();
	candidates.dedup();

	let max_distance = name.chars().count().div_ceil(3).max(2);
	let mut scored: Vec<(usize, String)> = candidates
		.into_iter()
		.map(|candidate| (levenshtein(name, &candidate), candidate))
		.filter(|(distance, _)| *distance <= max_distance)
		.collect();

	scored.sort_by_key(|(distance, _)| *distance);
	Ok(scored
		.into_iter()
		.take(5)
		.map(|(_, candidate)| candidate)
		.collect())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_suggest_similar_fallback() {
		// Test with a package manager that should be available on most systems
		let pm = crate::os_detect::detect_system_package_manager();
		if pm == crate::os_detect::SystemPackageManager::Unknown {
			// Skip test if no package manager is detected
			return;
		}
		let sug = suggest_similar("libcrl", pm).unwrap();
		// Only assert if we got results (some package managers may not return suggestions)
		if !sug.is_empty() {
			// Package names differ per manager: apt has libcurl*/libssl*,
			// brew has curl/openssl, choco has curl/openssl as well.
			assert!(
				sug.iter().any(|s| {
					let s = s.to_lowercase();
					s.contains("curl") || s.contains("ssl")
				}),
				"expected a curl/ssl-related suggestion, got {sug:?}"
			);
		}
	}

	#[test]
	fn test_package_exists_fallback() {
		let pm = crate::os_detect::detect_system_package_manager();
		if pm == crate::os_detect::SystemPackageManager::Unknown {
			// Skip test if no package manager is detected
			return;
		}
		let exists = package_exists("libcurl", pm).unwrap();
		// Only assert if package check succeeded (may vary by system)
		if exists {
			let non = package_exists("fake-nonexistent-package-xyz", pm).unwrap();
			assert!(!non);
		}
	}
}
