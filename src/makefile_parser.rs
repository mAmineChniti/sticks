use anyhow::{Context, Result, bail};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;

/// Represents a parsed Makefile with structured rules
#[derive(Debug, Clone)]
pub struct Makefile {
	pub rules: HashMap<String, Rule>,
	pub content: String,
}

/// Represents a single Makefile rule
#[derive(Debug, Clone)]
pub struct Rule {
	pub target: String,
	pub dependencies: Vec<String>,
	pub commands: Vec<String>,
	pub line_number: usize,
}

impl Makefile {
	/// Parse a Makefile from a file path
	pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
		let content = fs::read_to_string(&path).context("Failed to read Makefile")?;
		Self::parse(&content)
	}

	/// Parse Makefile content from a string
	pub fn parse(content: &str) -> Result<Self> {
		let mut rules = HashMap::new();
		let mut current_rule: Option<Rule> = None;
		for (line_number, line) in content.lines().enumerate() {
			// Skip empty lines and comments
			if line.trim().is_empty() || line.trim_start().starts_with('#') {
				continue;
			}

			// Check if this is a rule definition (target: dependencies)
			if !line.starts_with('\t') && line.contains(':') {
				// Save previous rule if exists
				if let Some(rule) = current_rule.take() {
					rules.insert(rule.target.clone(), rule);
				}

				// Parse new rule
				let (target, deps_str) = line.split_once(':').context("Invalid rule format")?;

				let target = target.trim().to_string();
				let dependencies = deps_str.split_whitespace().map(|s| s.to_string()).collect();

				current_rule = Some(Rule {
					target,
					dependencies,
					commands: Vec::new(),
					line_number,
				});
			} else if let Some(stripped) = line.strip_prefix('\t') {
				// This is a command line
				if let Some(ref mut rule) = current_rule {
					rule.commands.push(stripped.to_string()); // Remove leading tab
				}
			}
		}

		// Save final rule if exists
		if let Some(rule) = current_rule {
			rules.insert(rule.target.clone(), rule);
		}

		Ok(Makefile {
			rules,
			content: content.to_string(),
		})
	}

	/// Get or create the install-deps rule
	pub fn get_or_create_install_deps(&mut self) -> &mut Rule {
		self.rules
			.entry("install-deps".to_string())
			.or_insert_with(|| Rule {
				target: "install-deps".to_string(),
				dependencies: Vec::new(),
				commands: Vec::new(),
				line_number: 0,
			})
	}

	/// Add dependencies to the install-deps rule, deduplicating and sorting
	pub fn add_dependencies(&mut self, deps: &[String]) -> Result<Vec<String>> {
		if deps.is_empty() {
			return Ok(Vec::new());
		}

		// Validate dependency names
		for dep in deps {
			validate_dependency_name(dep)?;
		}

		let install_deps = self.get_or_create_install_deps();

		// Convert commands to a set of dependencies for deduplication
		let mut current_deps: BTreeSet<String> = BTreeSet::new();

		// Extract existing dependencies from the install command
		if let Some(cmd) = install_deps.commands.first() {
			// Parse: sudo <pm> install -y dep1 dep2 or sudo pacman -S --noconfirm dep1 dep2
			// Find the package manager command (install, -S, add, etc.)
			let install_keywords = ["install", "-S", "add"];
			let mut start_idx = None;
			for keyword in &install_keywords {
				if let Some(pos) = cmd.find(keyword) {
					start_idx = Some(pos + keyword.len());
					break;
				}
			}

			if let Some(start) = start_idx {
				let after_cmd = &cmd[start..];
				for part in after_cmd.split_whitespace() {
					// Skip flags like -y, --noconfirm, and sudo
					if !part.starts_with('-') && part != "sudo" && !part.is_empty() {
						current_deps.insert(part.to_string());
					}
				}
			}
		}

		let mut added = Vec::new();
		for dep in deps {
			if current_deps.insert(dep.clone()) {
				added.push(dep.clone());
			}
		}

		if !added.is_empty() {
			// Rebuild the install command
			let sorted_deps: Vec<_> = current_deps.into_iter().collect();
			let install_prefix = crate::os_detect::install_command_prefix();
			let new_cmd = format!("{} {}", install_prefix, sorted_deps.join(" "));
			install_deps.commands = vec![new_cmd];
		}

		// Ensure "all" rule includes install-deps
		ensure_all_rule_includes_deps(self);

		Ok(added)
	}

	/// Remove dependencies from the install-deps rule
	pub fn remove_dependencies(&mut self, deps: &[String]) -> Result<Vec<String>> {
		if deps.is_empty() {
			return Ok(Vec::new());
		}

		let install_deps = match self.rules.get_mut("install-deps") {
			Some(rule) => rule,
			None => bail!("No install-deps rule found in Makefile"),
		};

		let mut current_deps: BTreeSet<String> = BTreeSet::new();
		let deps_to_remove: std::collections::HashSet<_> = deps.iter().cloned().collect();

		// Extract existing dependencies
		if let Some(cmd) = install_deps.commands.first() {
			// Parse: sudo <pm> install -y dep1 dep2 or sudo pacman -S --noconfirm dep1 dep2
			// Find the package manager command (install, -S, add, etc.)
			let install_keywords = ["install", "-S", "add"];
			let mut start_idx = None;
			for keyword in &install_keywords {
				if let Some(pos) = cmd.find(keyword) {
					start_idx = Some(pos + keyword.len());
					break;
				}
			}

			if let Some(start) = start_idx {
				let after_cmd = &cmd[start..];
				for part in after_cmd.split_whitespace() {
					// Skip flags like -y, --noconfirm, and sudo
					if !part.starts_with('-') && part != "sudo" && !part.is_empty() {
						current_deps.insert(part.to_string());
					}
				}
			}
		}

		let before_count = current_deps.len();
		current_deps.retain(|dep| !deps_to_remove.contains(dep));
		let removed_count = before_count - current_deps.len();

		if removed_count == 0 {
			bail!("Dependencies not found in Makefile: {:?}", deps);
		}

		let removed: Vec<_> = deps
			.iter()
			.filter(|d| !current_deps.contains(*d))
			.cloned()
			.collect();

		if current_deps.is_empty() {
			// Remove the entire install-deps rule
			self.rules.remove("install-deps");
			remove_install_deps_from_all(self);
		} else {
			// Update the command with remaining dependencies
			let sorted_deps: Vec<_> = current_deps.into_iter().collect();
			let install_prefix = crate::os_detect::install_command_prefix();
			let new_cmd = format!("{} {}", install_prefix, sorted_deps.join(" "));
			install_deps.commands = vec![new_cmd];
		}

		Ok(removed)
	}

	/// Serialize back to Makefile format
	pub fn to_makefile_string(&self) -> String {
		let mut output = String::new();
		let mut written_targets = std::collections::HashSet::new();

		// First, preserve the original structure by going through original content
		let mut in_rule = false;
		for line in self.content.lines() {
			if line.trim().is_empty() {
				output.push('\n');
				continue;
			}

			if line.trim_start().starts_with('#') {
				output.push_str(line);
				output.push('\n');
				continue;
			}

			if !line.starts_with('\t')
				&& line.contains(':')
				&& let Some(target) = line.split(':').next()
			{
				let target = target.trim();
				if let Some(rule) = self.rules.get(target) {
					written_targets.insert(target.to_string());
					output.push_str(&rule.to_makefile_format());
					in_rule = true;
					continue;
				} else if !self.rules.contains_key(target) {
					// Rule was removed, skip it
					in_rule = true;
					continue;
				}
			}

			// For other lines, skip if it was part of a modified rule
			if !in_rule {
				output.push_str(line);
				output.push('\n');
			} else if line.starts_with('\t') {
				continue; // Skip commands from modified rules
			} else {
				in_rule = false;
				output.push_str(line);
				output.push('\n');
			}
		}

		// Append new rules that weren't in original content
		for (target, rule) in &self.rules {
			if !written_targets.contains(target) {
				output.push('\n');
				output.push_str(&rule.to_makefile_format());
			}
		}

		output
	}

	/// Write to file with atomic semantics (write to temp, then rename)
	pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
		let path = path.as_ref();
		let temp_path = path.with_extension("tmp");

		fs::write(&temp_path, self.to_makefile_string())
			.context("Failed to write temporary Makefile")?;

		fs::rename(&temp_path, path)
			.context("Failed to write Makefile (atomic operation failed)")?;

		Ok(())
	}
}

impl Rule {
	pub fn to_makefile_format(&self) -> String {
		let mut output = format!("{}: {}\n", self.target, self.dependencies.join(" "));
		for cmd in &self.commands {
			output.push('\t');
			output.push_str(cmd);
			output.push('\n');
		}
		output
	}
}

/// Ensure the "all" rule includes install-deps as a dependency
fn ensure_all_rule_includes_deps(makefile: &mut Makefile) {
	if let Some(all_rule) = makefile.rules.get_mut("all")
		&& !all_rule.dependencies.contains(&"install-deps".to_string())
	{
		all_rule.dependencies.push("install-deps".to_string());
	}
}

/// Remove install-deps from the "all" rule
fn remove_install_deps_from_all(makefile: &mut Makefile) {
	if let Some(all_rule) = makefile.rules.get_mut("all") {
		all_rule.dependencies.retain(|dep| dep != "install-deps");
	}
}

/// Validate dependency name (basic security check)
fn validate_dependency_name(name: &str) -> Result<()> {
	if name.is_empty() {
		bail!("Dependency name cannot be empty");
	}

	// Allow alphanumeric, dashes, underscores, dots (common in package names)
	if !name
		.chars()
		.all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
	{
		bail!(
			"Invalid dependency name: {}. Only alphanumeric characters, dashes, underscores, and dots are allowed.",
			name
		);
	}

	if name.len() > 100 {
		bail!("Dependency name too long: {}", name);
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_simple_makefile() {
		let content = "all: clean\n\tbuild\n";
		let makefile = Makefile::parse(content).unwrap();
		assert!(makefile.rules.contains_key("all"));
	}

	#[test]
	fn test_add_dependencies() {
		let content = "all: clean\n\tbuild\n";
		let mut makefile = Makefile::parse(content).unwrap();
		let added = makefile.add_dependencies(&["libcurl".to_string()]).unwrap();
		assert_eq!(added.len(), 1);
		assert!(makefile.rules.contains_key("install-deps"));
	}

	#[test]
	fn test_validate_dependency_name() {
		assert!(validate_dependency_name("libcurl").is_ok());
		assert!(validate_dependency_name("lib-curl_2.0").is_ok());
		assert!(validate_dependency_name("lib.curl").is_ok());
		assert!(validate_dependency_name("").is_err());
		assert!(validate_dependency_name("lib;curl").is_err());
	}
}
