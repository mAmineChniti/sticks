use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

/// Manages CMake dependencies using find_package() and target_link_libraries()
pub struct CMakeDependencyManager {
	pub content: String,
}

impl CMakeDependencyManager {
	/// Parse CMakeLists.txt from file
	pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
		let content = fs::read_to_string(&path).context("Failed to read CMakeLists.txt")?;
		Ok(CMakeDependencyManager { content })
	}

	/// Parse CMakeLists.txt from string
	pub fn parse(content: &str) -> Self {
		CMakeDependencyManager {
			content: content.to_string(),
		}
	}

	/// Add dependencies to CMakeLists.txt
	/// Adds find_package() directives and target_link_libraries()
	/// Supports component-based dependencies (e.g., "Boost:filesystem")
	pub fn add_dependencies(&mut self, deps: &[String]) -> Result<Vec<String>> {
		if deps.is_empty() {
			return Ok(Vec::new());
		}

		let mut added = Vec::new();

		for dep in deps {
			// Parse component syntax: "package:component1,component2"
			let (package_name, components) = if dep.contains(':') {
				let parts: Vec<&str> = dep.splitn(2, ':').collect();
				validate_cmake_package_name(parts[0])?;
				let pkg = format_cmake_package_name(parts[0]);
				let comps: Vec<String> =
					parts[1].split(',').map(|c| c.trim().to_string()).collect();
				(pkg, comps)
			} else {
				validate_cmake_package_name(dep)?;
				(format_cmake_package_name(dep), Vec::new())
			};

			// Check if already present
			let find_pattern = if components.is_empty() {
				format!("find_package({})", package_name)
			} else {
				format!("find_package({} COMPONENTS", package_name)
			};

			if self.content.contains(&find_pattern)
				|| self
					.content
					.contains(&format!("find_package({} ", package_name))
			{
				continue; // Already present
			}

			added.push(dep.clone());

			// Add find_package directive after project() line
			if let Some(project_pos) = self.content.find("project(")
				&& let Some(newline_pos) = self.content[project_pos..].find('\n')
			{
				let insert_pos = project_pos + newline_pos + 1;
				let find_cmd = if components.is_empty() {
					format!("find_package({} REQUIRED)\n", package_name)
				} else {
					format!(
						"find_package({} REQUIRED COMPONENTS {})\n",
						package_name,
						components.join(" ")
					)
				};
				self.content.insert_str(insert_pos, &find_cmd);
			}

			// Add target_link_libraries
			// For components, link to specific components
			let link_names = if components.is_empty() {
				vec![package_name.clone()]
			} else {
				components
					.iter()
					.map(|c| format!("{}::{}", package_name, c))
					.collect()
			};

			for link_name in link_names {
				self.add_to_target_link_libraries(&link_name)?;
			}
		}

		Ok(added)
	}

	/// Remove dependencies from CMakeLists.txt
	pub fn remove_dependencies(&mut self, deps: &[String]) -> Result<Vec<String>> {
		if deps.is_empty() {
			return Ok(Vec::new());
		}

		let mut removed = Vec::new();

		for dep in deps {
			let package_name = format_cmake_package_name(dep);
			let find_pattern = format!("find_package({}", package_name);

			if !self.content.contains(&find_pattern) {
				bail!("Package {} not found in CMakeLists.txt", dep);
			}

			removed.push(dep.clone());

			// Remove find_package line
			if let Some(start) = self.content.find(&find_pattern)
				&& let Some(end) = self.content[start..].find('\n')
			{
				self.content.drain(start..start + end + 1);
			}

			// Remove from target_link_libraries
			self.remove_from_target_link_libraries(&package_name)?;
		}

		Ok(removed)
	}

	/// Add package to target_link_libraries()
	fn add_to_target_link_libraries(&mut self, package_name: &str) -> Result<()> {
		let target_link_pattern = "target_link_libraries(";

		if let Some(pos) = self.content.find(target_link_pattern) {
			// Find the end of the target_link_libraries call
			let rest = &self.content[pos..];
			if let Some(end) = rest.find(')') {
				let insert_pos = pos + end;
				let link_entry = format!(" {}", package_name);
				if !rest[..end].contains(package_name) {
					self.content.insert_str(insert_pos, &link_entry);
				}
			}
		} else {
			// Synthesize a new target_link_libraries entry if none exists
			// Find the first target (add_executable or add_library)
			if let Some(target_pos) = self
				.content
				.find("add_executable(")
				.or_else(|| self.content.find("add_library("))
			{
				let rest = &self.content[target_pos..];
				if let Some(end) = rest.find(')') {
					let insert_pos = target_pos + end;
					// Extract target name
					let target_name = rest[..end].split_whitespace().nth(1).unwrap_or("main");
					let new_entry =
						format!("\ntarget_link_libraries({} {})", target_name, package_name);
					self.content.insert_str(insert_pos, &new_entry);
				}
			}
		}

		Ok(())
	}

	/// Remove package from target_link_libraries()
	fn remove_from_target_link_libraries(&mut self, package_name: &str) -> Result<()> {
		// Simple removal: find and remove the package reference
		let pattern = format!(" {}", package_name);
		if let Some(pos) = self.content.find(&pattern) {
			self.content.drain(pos..pos + pattern.len());
		}

		// Also try without leading space
		if let Some(pos) = self.content.find(package_name)
			&& pos > 0
			&& self.content.chars().nth(pos - 1) == Some(' ')
		{
			self.content.drain(pos - 1..pos + package_name.len());
		}

		Ok(())
	}

	/// Write to file with atomic semantics
	pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
		let path = path.as_ref();
		let temp_path = path.with_extension("tmp");

		fs::write(&temp_path, &self.content).context("Failed to write temporary CMakeLists.txt")?;

		fs::rename(&temp_path, path)
			.context("Failed to write CMakeLists.txt (atomic operation failed)")?;

		Ok(())
	}
}

/// Format dependency name to CMake package name (e.g., "libcurl" -> "CURL")
fn format_cmake_package_name(dep: &str) -> String {
	let normalized = if dep.to_lowercase().starts_with("lib") {
		&dep[3..]
	} else {
		dep
	};
	normalized.to_uppercase().replace("_", "")
}

/// Validate CMake package name
fn validate_cmake_package_name(name: &str) -> Result<()> {
	if name.is_empty() {
		bail!("Package name cannot be empty");
	}

	if name.len() > 100 {
		bail!("Package name too long: {}", name);
	}

	// CMake package names typically follow similar rules
	if !name
		.chars()
		.all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
	{
		bail!(
			"Invalid package name: {}. Only alphanumeric characters, dashes, underscores, and dots are allowed.",
			name
		);
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_cmake() {
		let content = "cmake_minimum_required(VERSION 3.15)\nproject(myapp)\n";
		let cm = CMakeDependencyManager::parse(content);
		assert!(!cm.content.is_empty());
	}

	#[test]
	fn test_format_package_name() {
		assert_eq!(format_cmake_package_name("libcurl"), "CURL");
		assert_eq!(format_cmake_package_name("openssl"), "OPENSSL");
	}
}
