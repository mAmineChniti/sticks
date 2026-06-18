use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

use crate::cmake_dependencies::CMakeDependencyManager;
use crate::makefile_parser::Makefile;
use crate::{
	BuildSystem, PackageManager,
	features::{detect_build_system, detect_package_manager},
};

/// Universal dependency manager that routes to the appropriate handler
/// based on detected build system and package manager
pub struct DependencyManager;

impl DependencyManager {
	/// Add dependencies to the project
	/// Automatically detects build system and routes accordingly
	pub fn add(dep_names: &[String]) -> Result<Vec<String>> {
		if dep_names.is_empty() {
			return Ok(Vec::new());
		}

		// Try to detect what system to use
		let build_system = detect_build_system().context("Failed to detect build system")?;

		let package_manager =
			detect_package_manager().context("Failed to detect package manager")?;

		match package_manager {
			Some(pm) => {
				// Route to package manager handler if available
				Self::add_to_package_manager(pm, dep_names)
			}
			None => {
				// Fall back to build system handler
				match build_system {
					Some(BuildSystem::Makefile) => Self::add_to_makefile(dep_names),
					Some(BuildSystem::CMake) => Self::add_to_cmake(dep_names),
					None => bail!(
						"No build system or package manager detected. Please initialize your project first."
					),
				}
			}
		}
	}

	/// Remove dependencies from the project
	pub fn remove(dep_names: &[String]) -> Result<Vec<String>> {
		if dep_names.is_empty() {
			return Ok(Vec::new());
		}

		let build_system = detect_build_system().context("Failed to detect build system")?;

		let package_manager =
			detect_package_manager().context("Failed to detect package manager")?;

		match package_manager {
			Some(pm) => {
				// Route to package manager handler if available
				Self::remove_from_package_manager(pm, dep_names)
			}
			None => {
				// Fall back to build system handler
				match build_system {
					Some(BuildSystem::Makefile) => Self::remove_from_makefile(dep_names),
					Some(BuildSystem::CMake) => Self::remove_from_cmake(dep_names),
					None => bail!("No build system or package manager detected."),
				}
			}
		}
	}

	/// Add dependencies to Makefile
	fn add_to_makefile(dep_names: &[String]) -> Result<Vec<String>> {
		if !Path::new("Makefile").exists() {
			bail!("Makefile not found in the current directory");
		}

		// Validate package names before modifying the Makefile
		let pm = crate::os_detect::detect_system_package_manager();
		for dep in dep_names {
			if !crate::package_checker::package_exists(dep, pm)? {
				let suggestions =
					crate::package_checker::suggest_similar(dep, pm).unwrap_or_default();
				if let Some(first) = suggestions.first() {
					bail!("Package '{}' not found. Did you mean '{}' ?", dep, first);
				} else {
					bail!("Package '{}' not found in repositories.", dep);
				}
			}
		}

		let mut makefile = Makefile::from_file("Makefile")?;
		let added = makefile.add_dependencies(dep_names)?;

		makefile.write_to_file("Makefile")?;

		if !added.is_empty() {
			println!("✓ Added dependencies to Makefile: {:?}", added);
		} else {
			println!("ℹ All specified dependencies already present in Makefile");
		}

		Ok(added)
	}

	/// Remove dependencies from Makefile
	fn remove_from_makefile(dep_names: &[String]) -> Result<Vec<String>> {
		if !Path::new("Makefile").exists() {
			bail!("Makefile not found in the current directory");
		}

		let mut makefile = Makefile::from_file("Makefile")?;
		let removed = makefile.remove_dependencies(dep_names)?;

		makefile.write_to_file("Makefile")?;

		println!("✓ Removed dependencies from Makefile: {:?}", removed);
		Ok(removed)
	}

	/// Add dependencies to CMakeLists.txt
	fn add_to_cmake(dep_names: &[String]) -> Result<Vec<String>> {
		if !Path::new("CMakeLists.txt").exists() {
			bail!("CMakeLists.txt not found in the current directory");
		}

		let mut cmake = CMakeDependencyManager::from_file("CMakeLists.txt")?;
		let added = cmake.add_dependencies(dep_names)?;

		cmake.write_to_file("CMakeLists.txt")?;

		if !added.is_empty() {
			println!("✓ Added dependencies to CMakeLists.txt: {:?}", added);
		} else {
			println!("ℹ All specified dependencies already present in CMakeLists.txt");
		}

		Ok(added)
	}

	/// Remove dependencies from CMakeLists.txt
	fn remove_from_cmake(dep_names: &[String]) -> Result<Vec<String>> {
		if !Path::new("CMakeLists.txt").exists() {
			bail!("CMakeLists.txt not found in the current directory");
		}

		let mut cmake = CMakeDependencyManager::from_file("CMakeLists.txt")?;
		let removed = cmake.remove_dependencies(dep_names)?;

		cmake.write_to_file("CMakeLists.txt")?;

		println!("✓ Removed dependencies from CMakeLists.txt: {:?}", removed);
		Ok(removed)
	}

	/// Add dependencies to Conan (conanfile.txt/py)
	fn add_to_package_manager(pm: PackageManager, dep_names: &[String]) -> Result<Vec<String>> {
		match pm {
			PackageManager::Conan => Self::add_to_conan(dep_names),
			PackageManager::Vcpkg => Self::add_to_vcpkg(dep_names),
		}
	}

	/// Remove dependencies from package manager
	fn remove_from_package_manager(
		pm: PackageManager,
		dep_names: &[String],
	) -> Result<Vec<String>> {
		match pm {
			PackageManager::Conan => Self::remove_from_conan(dep_names),
			PackageManager::Vcpkg => Self::remove_from_vcpkg(dep_names),
		}
	}

	/// Add dependencies to Conan conanfile.txt or conanfile.py
	fn add_to_conan(dep_names: &[String]) -> Result<Vec<String>> {
		let (conanfile_path, is_python) = if Path::new("conanfile.txt").exists() {
			("conanfile.txt", false)
		} else if Path::new("conanfile.py").exists() {
			("conanfile.py", true)
		} else {
			bail!("No conanfile.txt or conanfile.py found in current directory");
		};

		if is_python {
			Self::add_to_conan_py(conanfile_path, dep_names)
		} else {
			Self::add_to_conan_txt(conanfile_path, dep_names)
		}
	}

	/// Add dependencies to conanfile.txt (INI-style)
	fn add_to_conan_txt(conanfile_path: &str, dep_names: &[String]) -> Result<Vec<String>> {
		let content = fs::read_to_string(conanfile_path).context("Failed to read conanfile")?;

		let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
		let mut added = Vec::new();
		let mut in_requires_section = false;
		let mut existing_deps = std::collections::HashSet::new();

		// First pass: collect all existing dependencies from [requires] section
		for line in lines.iter() {
			if line.trim() == "[requires]" {
				in_requires_section = true;
			} else if line.trim().starts_with('[') && in_requires_section {
				in_requires_section = false;
			} else if in_requires_section && !line.trim().is_empty() {
				existing_deps.insert(line.trim().to_string());
			}
		}

		// Second pass: determine which dependencies to add
		for dep in dep_names {
			if !existing_deps.contains(dep) {
				added.push(dep.clone());
			}
		}

		// Add new dependencies
		if !added.is_empty() {
			let insert_pos = lines
				.iter()
				.position(|l| l.trim() == "[requires]")
				.ok_or_else(|| anyhow::anyhow!("No [requires] section found in conanfile"))?;

			for dep in &added {
				lines.insert(insert_pos + 1, dep.clone());
			}

			fs::write(conanfile_path, lines.join("\n")).context("Failed to write conanfile")?;

			println!("✓ Added dependencies to conanfile: {:?}", added);
		} else {
			println!("ℹ All specified dependencies already present in conanfile");
		}

		Ok(added)
	}

	/// Add dependencies to conanfile.py (Python-style)
	fn add_to_conan_py(conanfile_path: &str, dep_names: &[String]) -> Result<Vec<String>> {
		let content = fs::read_to_string(conanfile_path).context("Failed to read conanfile.py")?;

		let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
		let mut added = Vec::new();
		let mut class_level_deps = std::collections::HashSet::new();
		let mut method_level_deps = std::collections::HashSet::new();
		let mut requires_line_idx = None;
		let mut requirements_method_idx = None;

		// Parse existing dependencies from conanfile.py
		for (i, line) in lines.iter().enumerate() {
			let trimmed = line.trim();

			// Check for class-level requires = "dep1, dep2"
			if trimmed.starts_with("requires") && trimmed.contains('=') {
				requires_line_idx = Some(i);
				if let Some(eq_pos) = trimmed.find('=') {
					let after_eq = &trimmed[eq_pos + 1..].trim();
					let deps_str = after_eq
						.trim_start_matches('"')
						.trim_start_matches('\'')
						.trim_start_matches('[')
						.trim_end_matches('"')
						.trim_end_matches('\'')
						.trim_end_matches(']');

					for dep in deps_str.split(',') {
						let dep = dep
							.trim()
							.trim_start_matches('"')
							.trim_start_matches('\'')
							.trim_end_matches('"')
							.trim_end_matches('\'');
						if !dep.is_empty() {
							class_level_deps.insert(dep.to_string());
						}
					}
				}
			}

			// Check for def requirements(self): method
			if trimmed.starts_with("def requirements(self)") {
				requirements_method_idx = Some(i);
			}

			// Parse self.requires("dep") calls within requirements method
			if requirements_method_idx.is_some()
				&& trimmed.contains("self.requires")
				&& let Some(start) = trimmed.find('(')
				&& let Some(end) = trimmed.rfind(')')
			{
				let args = &trimmed[start + 1..end].trim();
				let dep = args
					.trim_start_matches('"')
					.trim_start_matches('\'')
					.trim_end_matches('"')
					.trim_end_matches('\'');
				if !dep.is_empty() {
					method_level_deps.insert(dep.to_string());
				}
			}
		}

		// Combine all existing deps for deduplication check
		let mut all_existing_deps = class_level_deps.clone();
		all_existing_deps.extend(method_level_deps.clone());

		// Determine which dependencies to add
		for dep in dep_names {
			if !all_existing_deps.contains(dep) {
				added.push(dep.clone());
			}
		}

		// Add new dependencies to the appropriate location
		if !added.is_empty() {
			if let Some(idx) = requires_line_idx {
				// Prefer class-level requires if it exists
				let line = &lines[idx];
				let trimmed = line.trim();
				if let Some(eq_pos) = trimmed.find('=') {
					// Preserve original indentation by using the original line
					let original_eq_pos = line.find('=').unwrap_or(eq_pos);
					let before_eq = &line[..=original_eq_pos];

					// Build new requires string with both class-level and new deps
					let mut all_deps: Vec<String> = class_level_deps
						.iter()
						.cloned()
						.chain(added.iter().cloned())
						.collect();
					all_deps.sort();
					all_deps.dedup();

					let new_deps_str = format!("\"{}\"", all_deps.join("\", \""));
					let new_line = format!("{} {}", before_eq, new_deps_str);
					lines[idx] = new_line;
				}
			} else if let Some(idx) = requirements_method_idx {
				// Add self.requires calls to requirements method
				let insert_pos = idx + 1;
				for dep in &added {
					lines.insert(insert_pos, format!("        self.requires(\"{}\")", dep));
				}
			} else {
				// Add new requires line after class definition
				let insert_pos = lines
					.iter()
					.position(|l| l.trim().starts_with("class "))
					.ok_or_else(|| anyhow::anyhow!("No class definition found in conanfile.py"))?;

				let deps_str = dep_names
					.iter()
					.map(|d| format!("\"{}\"", d))
					.collect::<Vec<_>>()
					.join(", ");
				lines.insert(insert_pos + 1, format!("    requires = {}", deps_str));
			}

			fs::write(conanfile_path, lines.join("\n")).context("Failed to write conanfile.py")?;

			println!("✓ Added dependencies to conanfile.py: {:?}", added);
		} else {
			println!("ℹ All specified dependencies already present in conanfile.py");
		}

		Ok(added)
	}

	/// Remove dependencies from Conan conanfile.txt or conanfile.py
	fn remove_from_conan(dep_names: &[String]) -> Result<Vec<String>> {
		let (conanfile_path, is_python) = if Path::new("conanfile.txt").exists() {
			("conanfile.txt", false)
		} else if Path::new("conanfile.py").exists() {
			("conanfile.py", true)
		} else {
			bail!("No conanfile.txt or conanfile.py found in current directory");
		};

		if is_python {
			Self::remove_from_conan_py(conanfile_path, dep_names)
		} else {
			Self::remove_from_conan_txt(conanfile_path, dep_names)
		}
	}

	/// Remove dependencies from conanfile.txt (INI-style)
	fn remove_from_conan_txt(conanfile_path: &str, dep_names: &[String]) -> Result<Vec<String>> {
		let content = fs::read_to_string(conanfile_path).context("Failed to read conanfile")?;

		let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
		let mut removed = Vec::new();
		let mut in_requires_section = false;

		lines.retain(|line| {
			if line.trim() == "[requires]" {
				in_requires_section = true;
				true
			} else if line.trim().starts_with('[') && in_requires_section {
				in_requires_section = false;
				true
			} else if in_requires_section {
				let should_remove = dep_names.iter().any(|dep| line.contains(dep));
				if should_remove {
					removed.push(line.trim().to_string());
				}
				!should_remove
			} else {
				true
			}
		});

		if removed.is_empty() {
			bail!("Dependencies not found in conanfile: {:?}", dep_names);
		}

		fs::write(conanfile_path, lines.join("\n")).context("Failed to write conanfile")?;

		println!("✓ Removed dependencies from conanfile: {:?}", removed);
		Ok(removed)
	}

	/// Remove dependencies from conanfile.py (Python-style)
	fn remove_from_conan_py(conanfile_path: &str, dep_names: &[String]) -> Result<Vec<String>> {
		let content = fs::read_to_string(conanfile_path).context("Failed to read conanfile.py")?;

		let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
		let mut removed = Vec::new();
		let mut class_level_deps = std::collections::HashSet::new();
		let mut method_level_deps = std::collections::HashSet::new();
		let mut requires_line_idx = None;
		let mut requirements_method_idx = None;
		let mut self_requires_lines = Vec::new();

		// Parse existing dependencies from conanfile.py
		for (i, line) in lines.iter().enumerate() {
			let trimmed = line.trim();

			// Check for class-level requires = "dep1, dep2"
			if trimmed.starts_with("requires") && trimmed.contains('=') {
				requires_line_idx = Some(i);
				if let Some(eq_pos) = trimmed.find('=') {
					let after_eq = &trimmed[eq_pos + 1..].trim();
					let deps_str = after_eq
						.trim_start_matches('"')
						.trim_start_matches('\'')
						.trim_start_matches('[')
						.trim_end_matches('"')
						.trim_end_matches('\'')
						.trim_end_matches(']');

					for dep in deps_str.split(',') {
						let dep = dep
							.trim()
							.trim_start_matches('"')
							.trim_start_matches('\'')
							.trim_end_matches('"')
							.trim_end_matches('\'');
						if !dep.is_empty() {
							class_level_deps.insert(dep.to_string());
						}
					}
				}
			}

			// Check for def requirements(self): method
			if trimmed.starts_with("def requirements(self)") {
				requirements_method_idx = Some(i);
			}

			// Parse self.requires("dep") calls within requirements method
			if requirements_method_idx.is_some() && trimmed.contains("self.requires") {
				self_requires_lines.push(i);
				if let Some(start) = trimmed.find('(')
					&& let Some(end) = trimmed.rfind(')')
				{
					let args = &trimmed[start + 1..end].trim();
					let dep = args
						.trim_start_matches('"')
						.trim_start_matches('\'')
						.trim_end_matches('"')
						.trim_end_matches('\'');
					if !dep.is_empty() {
						method_level_deps.insert(dep.to_string());
					}
				}
			}
		}

		// Combine all existing deps for removal check
		let mut all_existing_deps = class_level_deps.clone();
		all_existing_deps.extend(method_level_deps.clone());

		// Determine which dependencies to remove
		for dep in dep_names {
			if all_existing_deps.contains(dep) {
				removed.push(dep.clone());
			}
		}

		if removed.is_empty() {
			bail!("Dependencies not found in conanfile.py: {:?}", dep_names);
		}

		// Remove dependencies from their respective locations
		if let Some(idx) = requires_line_idx {
			// Update existing requires line (class-level)
			let line = &lines[idx];
			let trimmed = line.trim();
			if let Some(eq_pos) = trimmed.find('=') {
				// Preserve original indentation by using the original line
				let original_eq_pos = line.find('=').unwrap_or(eq_pos);
				let before_eq = &line[..=original_eq_pos];

				// Build new requires string without removed deps
				let mut remaining_deps: Vec<String> = class_level_deps
					.iter()
					.filter(|d| !removed.contains(d))
					.cloned()
					.collect();
				remaining_deps.sort();

				if remaining_deps.is_empty() {
					// Remove the entire requires line if no deps left
					lines.remove(idx);
				} else {
					let new_deps_str = format!("\"{}\"", remaining_deps.join("\", \""));
					let new_line = format!("{} {}", before_eq, new_deps_str);
					lines[idx] = new_line;
				}
			}
		}

		// Remove self.requires lines for removed dependencies (method-level)
		let mut lines_to_remove = Vec::new();
		for line_idx in self_requires_lines {
			let trimmed = lines[line_idx].trim();
			if let Some(start) = trimmed.find('(')
				&& let Some(end) = trimmed.rfind(')')
			{
				let args = &trimmed[start + 1..end].trim();
				let dep = args
					.trim_start_matches('"')
					.trim_start_matches('\'')
					.trim_end_matches('"')
					.trim_end_matches('\'');
				if removed.contains(&dep.to_string()) {
					lines_to_remove.push(line_idx);
				}
			}
		}
		// Remove lines in reverse order to preserve indices
		for line_idx in lines_to_remove.into_iter().rev() {
			lines.remove(line_idx);
		}

		fs::write(conanfile_path, lines.join("\n")).context("Failed to write conanfile.py")?;

		println!("✓ Removed dependencies from conanfile.py: {:?}", removed);
		Ok(removed)
	}

	/// Add dependencies to vcpkg.json
	fn add_to_vcpkg(dep_names: &[String]) -> Result<Vec<String>> {
		let vcpkg_path = "vcpkg.json";
		if !Path::new(vcpkg_path).exists() {
			bail!("No vcpkg.json found in current directory");
		}

		let content = fs::read_to_string(vcpkg_path).context("Failed to read vcpkg.json")?;

		let mut json: serde_json::Value =
			serde_json::from_str(&content).context("Failed to parse vcpkg.json")?;

		// Ensure dependencies array exists, create if missing
		if !json["dependencies"].is_array() {
			json["dependencies"] = serde_json::json!([]);
		}
		let dependencies = json["dependencies"]
			.as_array_mut()
			.ok_or_else(|| anyhow::anyhow!("Failed to get dependencies array as mutable"))?;

		let mut added = Vec::new();
		for dep in dep_names {
			let dep_str = dep.as_str();

			// Parse version syntax: "package:version" or "package>=version"
			let (package_name, version) =
				if dep.contains(':') || dep.contains('>') || dep.contains('=') {
					let parts: Vec<&str> = dep.splitn(2, ':').collect();
					(parts[0], Some(parts[1].to_string()))
				} else {
					(dep_str, None)
				};

			if !dependencies.iter().any(|d| {
				if let Some(s) = d.as_str() {
					s == package_name
				} else if let Some(obj) = d.as_object() {
					obj.get("name").and_then(|n| n.as_str()) == Some(package_name)
				} else {
					false
				}
			}) {
				// Add with version if specified
				let dep_json = if let Some(ver) = version {
					serde_json::json!({
						"name": package_name,
						"version>=": ver
					})
				} else {
					serde_json::json!(package_name)
				};

				dependencies.push(dep_json);
				added.push(dep.clone());
			}
		}

		if !added.is_empty() {
			fs::write(vcpkg_path, serde_json::to_string_pretty(&json)?)
				.context("Failed to write vcpkg.json")?;

			println!("✓ Added dependencies to vcpkg.json: {:?}", added);
		} else {
			println!("ℹ All specified dependencies already present in vcpkg.json");
		}

		Ok(added)
	}

	/// Remove dependencies from vcpkg.json
	fn remove_from_vcpkg(dep_names: &[String]) -> Result<Vec<String>> {
		let vcpkg_path = "vcpkg.json";
		if !Path::new(vcpkg_path).exists() {
			bail!("No vcpkg.json found in current directory");
		}

		let content = fs::read_to_string(vcpkg_path).context("Failed to read vcpkg.json")?;

		let mut json: serde_json::Value =
			serde_json::from_str(&content).context("Failed to parse vcpkg.json")?;

		// Ensure dependencies array exists, treat missing as empty for remove
		if !json["dependencies"].is_array() {
			// No dependencies to remove
			bail!("Dependencies not found in vcpkg.json: {:?}", dep_names);
		}
		let dependencies = json["dependencies"]
			.as_array_mut()
			.ok_or_else(|| anyhow::anyhow!("Failed to get dependencies array as mutable"))?;

		let mut removed = Vec::new();

		dependencies.retain(|d| {
			let should_remove = dep_names.iter().any(|dep| {
				if let Some(s) = d.as_str() {
					s == dep
				} else if let Some(obj) = d.as_object() {
					obj.get("name").and_then(|n| n.as_str()) == Some(dep)
				} else {
					false
				}
			});

			if should_remove {
				removed.push(d.to_string());
			}
			!should_remove
		});

		if removed.is_empty() {
			bail!("Dependencies not found in vcpkg.json: {:?}", dep_names);
		}

		fs::write(vcpkg_path, serde_json::to_string_pretty(&json)?)
			.context("Failed to write vcpkg.json")?;

		println!("✓ Removed dependencies from vcpkg.json: {:?}", removed);
		Ok(removed)
	}

	/// List current dependencies
	pub fn list() -> Result<()> {
		let build_system = detect_build_system().context("Failed to detect build system")?;
		let package_manager =
			detect_package_manager().context("Failed to detect package manager")?;

		if let Some(pm) = package_manager {
			match pm {
				PackageManager::Conan => Self::list_conan(),
				PackageManager::Vcpkg => Self::list_vcpkg(),
			}
		} else {
			match build_system {
				Some(BuildSystem::Makefile) => Self::list_makefile(),
				Some(BuildSystem::CMake) => Self::list_cmake(),
				None => {
					println!("No build system or package manager detected.");
					Ok(())
				}
			}
		}
	}

	fn list_makefile() -> Result<()> {
		if !Path::new("Makefile").exists() {
			println!("No Makefile found.");
			return Ok(());
		}

		let makefile = Makefile::from_file("Makefile")?;

		if let Some(install_deps) = makefile.rules.get("install-deps") {
			if let Some(cmd) = install_deps.commands.first() {
				println!("📦 Makefile Dependencies:");
				// Extract and print dependencies - handle various package managers
				let parts: Vec<&str> = cmd.split_whitespace().collect();
				for dep in parts.iter() {
					// Skip common package manager command parts and flags
					if *dep != "-y"
						&& *dep != "install"
						&& *dep != "sudo" && *dep != "apt"
						&& *dep != "apt-get"
						&& *dep != "dnf" && *dep != "yum"
						&& *dep != "pacman"
						&& *dep != "apk" && *dep != "zypper"
						&& *dep != "dnf5" && *dep != "-S"
						&& *dep != "--noconfirm"
						&& *dep != "add"
					{
						println!("  • {}", dep);
					}
				}
			} else {
				println!("No dependencies found in Makefile.");
			}
		} else {
			println!("No install-deps rule found in Makefile.");
		}

		Ok(())
	}

	fn list_cmake() -> Result<()> {
		if !Path::new("CMakeLists.txt").exists() {
			println!("No CMakeLists.txt found.");
			return Ok(());
		}

		let cmake = CMakeDependencyManager::from_file("CMakeLists.txt")?;

		println!("📦 CMake Dependencies:");
		let mut found_any = false;

		for line in cmake.content.lines() {
			if line.contains("find_package(")
				&& let Some(start) = line.find("find_package(")
				&& let Some(end) = line[start..].find(")")
			{
				let pkg_content = &line[start + 13..start + end];
				// Take only the first token (package name), skip flags like REQUIRED
				if let Some(pkg_name) = pkg_content.split_whitespace().next() {
					println!("  • {}", pkg_name);
					found_any = true;
				}
			}
		}

		if !found_any {
			println!("No dependencies found in CMakeLists.txt.");
		}

		Ok(())
	}

	fn list_conan() -> Result<()> {
		let (conanfile_path, is_python) = if Path::new("conanfile.txt").exists() {
			("conanfile.txt", false)
		} else if Path::new("conanfile.py").exists() {
			("conanfile.py", true)
		} else {
			println!("No conanfile.txt or conanfile.py found.");
			return Ok(());
		};

		if is_python {
			Self::list_conan_py(conanfile_path)
		} else {
			Self::list_conan_txt(conanfile_path)
		}
	}

	/// List dependencies from conanfile.txt (INI-style)
	fn list_conan_txt(conanfile_path: &str) -> Result<()> {
		let content = fs::read_to_string(conanfile_path).context("Failed to read conanfile")?;

		println!("📦 Conan Dependencies:");
		let mut in_requires_section = false;
		let mut found_any = false;

		for line in content.lines() {
			if line.trim() == "[requires]" {
				in_requires_section = true;
			} else if line.trim().starts_with('[') && in_requires_section {
				in_requires_section = false;
			} else if in_requires_section && !line.trim().is_empty() {
				println!("  • {}", line.trim());
				found_any = true;
			}
		}

		if !found_any {
			println!("No dependencies found in conanfile.");
		}

		Ok(())
	}

	/// List dependencies from conanfile.py (Python-style)
	fn list_conan_py(conanfile_path: &str) -> Result<()> {
		let content = fs::read_to_string(conanfile_path).context("Failed to read conanfile.py")?;

		println!("📦 Conan Dependencies:");
		let mut found_any = false;
		let mut requirements_method_idx = None;

		for line in content.lines() {
			let trimmed = line.trim();

			// Check for class-level requires = "dep1, dep2"
			if trimmed.starts_with("requires")
				&& trimmed.contains('=')
				&& let Some(eq_pos) = trimmed.find('=')
			{
				let after_eq = &trimmed[eq_pos + 1..].trim();
				let deps_str = after_eq
					.trim_start_matches('"')
					.trim_start_matches('\'')
					.trim_start_matches('[')
					.trim_end_matches('"')
					.trim_end_matches('\'')
					.trim_end_matches(']');

				for dep in deps_str.split(',') {
					let dep = dep
						.trim()
						.trim_start_matches('"')
						.trim_start_matches('\'')
						.trim_end_matches('"')
						.trim_end_matches('\'');
					if !dep.is_empty() {
						println!("  • {}", dep);
						found_any = true;
					}
				}
			}

			// Check for def requirements(self): method
			if trimmed.starts_with("def requirements(self)") {
				requirements_method_idx = Some(true);
			}

			// Parse self.requires("dep") calls within requirements method
			if requirements_method_idx.is_some()
				&& trimmed.contains("self.requires")
				&& let Some(start) = trimmed.find('(')
				&& let Some(end) = trimmed.rfind(')')
			{
				let args = &trimmed[start + 1..end].trim();
				let dep = args
					.trim_start_matches('"')
					.trim_start_matches('\'')
					.trim_end_matches('"')
					.trim_end_matches('\'');
				if !dep.is_empty() {
					println!("  • {}", dep);
					found_any = true;
				}
			}
		}

		if !found_any {
			println!("No dependencies found in conanfile.py.");
		}

		Ok(())
	}

	fn list_vcpkg() -> Result<()> {
		let vcpkg_path = "vcpkg.json";
		if !Path::new(vcpkg_path).exists() {
			println!("No vcpkg.json found.");
			return Ok(());
		}

		let content = fs::read_to_string(vcpkg_path).context("Failed to read vcpkg.json")?;
		let json: serde_json::Value =
			serde_json::from_str(&content).context("Failed to parse vcpkg.json")?;

		// Treat missing dependencies as empty array
		let empty_vec = vec![];
		let dependencies = json["dependencies"].as_array().unwrap_or(&empty_vec);

		println!("📦 Vcpkg Dependencies:");
		if dependencies.is_empty() {
			println!("No dependencies found in vcpkg.json.");
		} else {
			for dep in dependencies {
				if let Some(s) = dep.as_str() {
					println!("  • {}", s);
				} else if let Some(obj) = dep.as_object()
					&& let Some(name) = obj.get("name").and_then(|n| n.as_str())
				{
					println!("  • {}", name);
				}
			}
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_dependency_manager_creation() {
		let _dm = DependencyManager;
		// Just ensure it compiles and creates
	}
}
