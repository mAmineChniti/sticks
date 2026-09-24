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
			let output = Command::new("apt-cache")
				.arg("policy")
				.arg(name)
				.output()
				.or_else(|_| Command::new("apt-get").arg("show").arg(name).output())?;
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
				return Ok(o.status.success());
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
						package == name
							|| package.strip_prefix(name).is_some_and(|suffix| {
								suffix.starts_with('-')
									&& suffix[1..]
										.chars()
										.all(|character| character.is_ascii_digit())
							})
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
				return Ok(o.status.success());
			}
			Ok(false)
		}
		SystemPackageManager::Unknown => Ok(false),
	}
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
	let value = trimmed
		.split_whitespace()
		.next()
		.filter(|value| *value != "-")?;
	if matches!(pm, SystemPackageManager::Apk)
		&& let Some((name, version)) = value.rsplit_once('-')
		&& version.chars().all(|character| character.is_ascii_digit())
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
			assert!(
				sug.iter()
					.any(|s| s.contains("libcurl") || s.contains("libssl"))
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
