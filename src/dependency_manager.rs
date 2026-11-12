use crate::file_handler::write_atomic;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

use crate::cmake_dependencies::CMakeDependencyManager;
use crate::makefile_parser::Makefile;
use crate::{
	BuildSystem, PackageManager,
	features::{detect_build_file, detect_build_system, detect_package_manager},
};

/// Universal dependency manager that routes to the appropriate handler
/// based on detected build system and package manager
pub struct DependencyManager;

impl DependencyManager {
	/// Add dependencies to the project
	/// Automatically detects build system and routes accordingly
	pub fn add(dep_names: &[String]) -> Result<Vec<String>> {
		Self::add_with_options(dep_names, false)
	}

	/// `system = true` forces the system-package path (Makefile
	/// `install-deps` / CMake `find_package`) even when a Conan or vcpkg
	/// manifest is present.
	pub fn add_with_options(dep_names: &[String], system: bool) -> Result<Vec<String>> {
		if dep_names.is_empty() {
			return Ok(Vec::new());
		}

		// Try to detect what system to use
		let build_system = detect_build_system().context("Failed to detect build system")?;

		let package_manager = if system {
			None
		} else {
			detect_package_manager().context("Failed to detect package manager")?
		};

		match package_manager {
			Some(pm) => {
				let manifest_path = package_manager_manifest_path(pm)?;
				let manifest_before = read_optional_file(&manifest_path)?;
				let build_path = detect_build_file()?;
				let build_before = match build_path.as_ref() {
					Some(path) => read_optional_file(path)?,
					None => None,
				};
				let result = (|| -> Result<Vec<String>> {
					let manifest_added = Self::add_to_package_manager(pm, dep_names)?;
					let build_dependencies = build_dependency_names(pm, dep_names)?;
					let build_added = match build_system {
						Some(BuildSystem::CMake) => Self::add_to_cmake(&build_dependencies)?,
						Some(BuildSystem::Makefile) => {
							Self::add_linker_dependencies_to_makefile(&build_dependencies)?
						}
						None => Vec::new(),
					};
					let mut all_added = manifest_added;
					for dependency in build_added {
						if !all_added.contains(&dependency) {
							all_added.push(dependency);
						}
					}
					Ok(all_added)
				})();
				if result.is_err() {
					restore_file(&manifest_path, manifest_before.as_deref())?;
					if let Some(build_path) = build_path {
						restore_file(&build_path, build_before.as_deref())?;
					}
				}
				result
			}
			None => {
				// Fall back to build system handler
				match build_system {
					Some(BuildSystem::Makefile) => {
						if !system {
							println!(
								"ℹ Editing system packages (install-deps). Use `sticks add --system` to make this explicit, or a package manager (-p conan|vcpkg) for manifest-based dependencies."
							);
						}
						Self::add_to_makefile(dep_names)
					}
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
		Self::remove_with_options(dep_names, false)
	}

	/// `system = true` mirrors [`Self::add_with_options`].
	pub fn remove_with_options(dep_names: &[String], system: bool) -> Result<Vec<String>> {
		if dep_names.is_empty() {
			return Ok(Vec::new());
		}

		let build_system = detect_build_system().context("Failed to detect build system")?;

		let package_manager = if system {
			None
		} else {
			detect_package_manager().context("Failed to detect package manager")?
		};

		match package_manager {
			Some(pm) => {
				let manifest_path = package_manager_manifest_path(pm)?;
				let manifest_before = read_optional_file(&manifest_path)?;
				let build_path = detect_build_file()?;
				let build_before = match build_path.as_ref() {
					Some(path) => read_optional_file(path)?,
					None => None,
				};
				let result = (|| -> Result<Vec<String>> {
					let build_dependencies = build_dependency_names(pm, dep_names)?;
					let build_removed = match build_system {
						Some(BuildSystem::CMake) => Self::remove_from_cmake(&build_dependencies)?,
						Some(BuildSystem::Makefile) => {
							Self::remove_linker_dependencies_from_makefile(&build_dependencies)?
						}
						None => Vec::new(),
					};
					let manifest_removed = Self::remove_from_package_manager(pm, dep_names)?;
					let mut all_removed = manifest_removed;
					for dependency in build_removed {
						if !all_removed.contains(&dependency) {
							all_removed.push(dependency);
						}
					}
					Ok(all_removed)
				})();
				if result.is_err() {
					restore_file(&manifest_path, manifest_before.as_deref())?;
					if let Some(build_path) = build_path {
						restore_file(&build_path, build_before.as_deref())?;
					}
				}
				result
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
		let makefile_path = build_file_for(BuildSystem::Makefile)?;
		if !makefile_path.is_file() {
			bail!("Makefile not found in the current directory");
		}

		// Validate package names before modifying the Makefile.
		// Skip strict validation when no supported system package manager is
		// detected (e.g. macOS/Windows): `package_exists` would always fail
		// there, so warn instead of blocking the user.
		let pm = crate::os_detect::detect_system_package_manager();
		if pm == crate::os_detect::SystemPackageManager::Unknown {
			println!(
				"⚠ No supported system package manager detected; skipping repository validation for {:?}.",
				dep_names
			);
		} else {
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
		}

		let mut makefile = Makefile::from_file(&makefile_path)?;
		let added = makefile.add_dependencies(dep_names)?;
		let mut content = makefile.to_makefile_string();
		append_makefile_link_flags(&mut content, dep_names);

		write_atomic(&makefile_path, &content).context("Failed to write Makefile")?;

		if !added.is_empty() {
			println!("✓ Added dependencies to Makefile: {:?}", added);
		} else {
			println!("ℹ All specified dependencies already present in Makefile");
		}

		Ok(added)
	}

	/// Remove dependencies from Makefile
	fn remove_from_makefile(dep_names: &[String]) -> Result<Vec<String>> {
		let makefile_path = build_file_for(BuildSystem::Makefile)?;
		if !makefile_path.is_file() {
			bail!("Makefile not found in the current directory");
		}

		let mut makefile = Makefile::from_file(&makefile_path)?;
		let removed = makefile.remove_dependencies(dep_names)?;
		let mut content = makefile.to_makefile_string();
		remove_makefile_link_flags(&mut content, dep_names);

		write_atomic(&makefile_path, &content).context("Failed to write Makefile")?;

		println!("✓ Removed dependencies from Makefile: {:?}", removed);
		Ok(removed)
	}

	fn add_linker_dependencies_to_makefile(dep_names: &[String]) -> Result<Vec<String>> {
		let makefile_path = build_file_for(BuildSystem::Makefile)?;
		let mut content = fs::read_to_string(&makefile_path).context("Failed to read Makefile")?;
		let mut added = Vec::new();
		for dependency in dep_names {
			let flags = makefile_link_flags(dependency);
			if flags.is_empty() {
				continue;
			}
			let existing = makefile_assignment(&content, "LDLIBS").unwrap_or_default();
			if flags
				.iter()
				.any(|flag| !existing.split_whitespace().any(|value| value == flag))
			{
				added.push(dependency.clone());
			}
		}
		append_makefile_link_flags(&mut content, &added);
		if !added.is_empty() {
			write_atomic(&makefile_path, &content).context("Failed to write Makefile")?;
		}
		Ok(added)
	}

	fn remove_linker_dependencies_from_makefile(dep_names: &[String]) -> Result<Vec<String>> {
		let makefile_path = build_file_for(BuildSystem::Makefile)?;
		let mut content = fs::read_to_string(&makefile_path).context("Failed to read Makefile")?;
		let removed: Vec<String> = dep_names
			.iter()
			.filter(|dependency| {
				let flags = makefile_link_flags(dependency);
				!flags.is_empty()
					&& makefile_assignment(&content, "LDLIBS").is_some_and(|value| {
						value
							.split_whitespace()
							.any(|item| flags.iter().any(|flag| item == flag))
					})
			})
			.cloned()
			.collect();
		remove_makefile_link_flags(&mut content, &removed);
		if !removed.is_empty() {
			write_atomic(&makefile_path, &content).context("Failed to write Makefile")?;
		}
		Ok(removed)
	}

	/// Add dependencies to CMakeLists.txt
	fn add_to_cmake(dep_names: &[String]) -> Result<Vec<String>> {
		let cmake_path = build_file_for(BuildSystem::CMake)?;
		if !cmake_path.is_file() {
			bail!("CMakeLists.txt not found in the current directory");
		}

		let mut cmake = CMakeDependencyManager::from_file(&cmake_path)?;
		let added = cmake.add_dependencies(dep_names)?;

		cmake.write_to_file(&cmake_path)?;

		if !added.is_empty() {
			println!("✓ Added dependencies to CMakeLists.txt: {:?}", added);
		} else {
			println!("ℹ All specified dependencies already present in CMakeLists.txt");
		}

		Ok(added)
	}

	/// Remove dependencies from CMakeLists.txt
	fn remove_from_cmake(dep_names: &[String]) -> Result<Vec<String>> {
		let cmake_path = build_file_for(BuildSystem::CMake)?;
		if !cmake_path.is_file() {
			bail!("CMakeLists.txt not found in the current directory");
		}

		let mut cmake = CMakeDependencyManager::from_file(&cmake_path)?;
		let removed = cmake.remove_dependencies(dep_names)?;

		cmake.write_to_file(&cmake_path)?;

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
		let dep_names = conan_specs(dep_names);
		let (conanfile_path, is_python) = if Path::new("conanfile.txt").exists() {
			("conanfile.txt", false)
		} else if Path::new("conanfile.py").exists() {
			("conanfile.py", true)
		} else {
			bail!("No conanfile.txt or conanfile.py found in current directory");
		};

		for dependency in &dep_names {
			validate_conan_dependency(dependency)?;
		}
		if is_python {
			Self::add_to_conan_py(conanfile_path, &dep_names)
		} else {
			Self::add_to_conan_txt(conanfile_path, &dep_names)
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
			} else if in_requires_section {
				let trimmed = line.trim();
				if !trimmed.is_empty() && !trimmed.starts_with('#') {
					existing_deps.insert(trimmed.to_string());
				}
			}
		}

		// Second pass: determine which dependencies to add
		for dep in dep_names {
			if !existing_deps
				.iter()
				.any(|existing| conan_base_name(existing) == conan_base_name(dep))
			{
				added.push(dep.clone());
			}
		}

		// Add new dependencies
		if !added.is_empty() {
			let insert_pos = match lines.iter().position(|line| line.trim() == "[requires]") {
				Some(position) => position,
				None => {
					lines.insert(0, "[requires]".to_string());
					0
				}
			};

			for (offset, dep) in added.iter().enumerate() {
				lines.insert(insert_pos + 1 + offset, dep.clone());
			}

			write_atomic(Path::new(conanfile_path), &lines.join("\n"))
				.context("Failed to write conanfile")?;

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
		let mut requires_end_idx = None;
		let mut requirements_method_idx = None;
		let mut in_requirements = false;

		// Parse existing dependencies from conanfile.py
		for (i, line) in lines.iter().enumerate() {
			let trimmed = line.trim();

			// Check for class-level requires = ... (word-boundary, not requiresFoo)
			if is_requires_assignment(trimmed) && requires_line_idx.is_none() {
				requires_line_idx = Some(i);
				let (value, end) = collect_assignment_value(&lines, i);
				requires_end_idx = Some(end);
				for dep in extract_python_strings(&value) {
					if !dep.is_empty() {
						class_level_deps.insert(dep);
					}
				}
				for dep in extract_python_strings(&value) {
					if !dep.is_empty() {
						class_level_deps.insert(dep);
					}
				}
				// Fallback: single unquoted token (rare, but don't lose it).
				if class_level_deps.is_empty()
					&& let Some(eq_pos) = trimmed.find('=')
				{
					let after_eq = trimmed[eq_pos + 1..].trim().trim_matches(',').trim();
					if !after_eq.is_empty() && !after_eq.starts_with(['"', '\'', '[', '(', '{']) {
						class_level_deps.insert(after_eq.to_string());
					}
				}
			}

			if trimmed.starts_with("def requirements")
				&& trimmed.contains('(')
				&& trimmed.contains(')')
			{
				requirements_method_idx = Some(i);
				in_requirements = true;
			} else if in_requirements && trimmed.starts_with("def ") {
				in_requirements = false;
			}

			if in_requirements && trimmed.contains("self.requires") {
				let (block, _) = collect_call_block(&lines, i);
				if let Some(dep) = parse_self_requires(&block) {
					method_level_deps.insert(dep);
				}
			}
		}

		// Combine all existing deps for deduplication check
		let mut all_existing_deps = class_level_deps.clone();
		all_existing_deps.extend(method_level_deps.clone());

		// Determine which dependencies to add
		for dep in dep_names {
			if !all_existing_deps
				.iter()
				.any(|existing| conan_base_name(existing) == conan_base_name(dep))
			{
				added.push(dep.clone());
			}
		}

		// Add new dependencies to the appropriate location
		if !added.is_empty() {
			if let Some(idx) = requires_line_idx {
				// Prefer class-level requires if it exists (replace whole
				// multiline span when present).
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

					let (original_value, end_idx) = collect_assignment_value(&lines, idx);
					let original_value = if original_value.is_empty() {
						trimmed
							.split_once('=')
							.map(|(_, value)| value)
							.unwrap_or("")
					} else {
						original_value.as_str()
					};
					let new_deps_str = render_python_requires(original_value, &all_deps);
					let new_line = format!("{} {}", before_eq, new_deps_str);
					let end_idx = requires_end_idx.unwrap_or(end_idx).max(idx);
					lines.splice(idx..=end_idx.min(lines.len().saturating_sub(1)), [new_line]);
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

			write_atomic(Path::new(conanfile_path), &lines.join("\n"))
				.context("Failed to write conanfile.py")?;

			println!("✓ Added dependencies to conanfile.py: {:?}", added);
		} else {
			println!("ℹ All specified dependencies already present in conanfile.py");
		}

		Ok(added)
	}

	/// Remove dependencies from Conan conanfile.txt or conanfile.py
	fn remove_from_conan(dep_names: &[String]) -> Result<Vec<String>> {
		let dep_names = conan_specs(dep_names);
		let (conanfile_path, is_python) = if Path::new("conanfile.txt").exists() {
			("conanfile.txt", false)
		} else if Path::new("conanfile.py").exists() {
			("conanfile.py", true)
		} else {
			bail!("No conanfile.txt or conanfile.py found in current directory");
		};

		for dependency in &dep_names {
			validate_conan_dependency(dependency)?;
		}
		if is_python {
			Self::remove_from_conan_py(conanfile_path, &dep_names)
		} else {
			Self::remove_from_conan_txt(conanfile_path, &dep_names)
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
				let trimmed = line.trim();
				let should_remove = !trimmed.is_empty()
					&& !trimmed.starts_with('#')
					&& dep_names
						.iter()
						.any(|dep| conan_base_name(trimmed) == conan_base_name(dep));
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

		write_atomic(Path::new(conanfile_path), &lines.join("\n"))
			.context("Failed to write conanfile")?;

		println!("✓ Removed dependencies from conanfile: {:?}", removed);
		Ok(removed)
	}

	fn remove_from_conan_py(conanfile_path: &str, dep_names: &[String]) -> Result<Vec<String>> {
		let content = fs::read_to_string(conanfile_path).context("Failed to read conanfile.py")?;
		let lines = content.lines().collect::<Vec<_>>();
		let mut class_line = None;
		let mut method_lines = Vec::new();
		let mut in_requirements = false;
		let mut existing = std::collections::HashSet::new();

		for (index, line) in lines.iter().enumerate() {
			let trimmed = line.trim();
			if trimmed.starts_with("def requirements") && trimmed.contains('(') {
				in_requirements = true;
				continue;
			}
			if in_requirements && trimmed.starts_with("def ") {
				in_requirements = false;
			}
			if !in_requirements && is_requires_assignment(trimmed) {
				let owned: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
				let (value, end) = collect_assignment_value(&owned, index);
				let deps = parse_python_requirements(&value);
				existing.extend(deps.iter().cloned());
				if class_line.is_none() {
					class_line = Some((index, end, deps));
				}
			}
			if in_requirements && trimmed.contains("self.requires") {
				let owned: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
				let (block, _) = collect_call_block(&owned, index);
				if let Some(dep) = parse_self_requires(&block) {
					existing.insert(dep.clone());
					method_lines.push((index, dep));
				}
			}
		}

		let removed = dep_names
			.iter()
			.filter(|dependency| {
				existing
					.iter()
					.any(|existing| conan_base_name(existing) == conan_base_name(dependency))
			})
			.cloned()
			.collect::<Vec<_>>();
		if removed.is_empty() {
			bail!("Dependencies not found in conanfile.py: {:?}", dep_names);
		}

		let mut output = Vec::new();
		// Set while skipping the continuation lines of a `requires = (...)`
		// block that has already been rewritten as a single line.
		let mut skip_through = 0usize;
		for (index, line) in lines.iter().enumerate() {
			if index <= skip_through {
				continue;
			}
			if let Some((class_index, class_end, class_deps)) = class_line.as_ref()
				&& *class_index == index
			{
				let remaining = class_deps
					.iter()
					.filter(|dependency| {
						!removed
							.iter()
							.any(|removed| conan_base_name(removed) == conan_base_name(dependency))
					})
					.cloned()
					.collect::<Vec<_>>();
				if remaining.is_empty() {
					// Drop the whole assignment block.
					skip_through = *class_end;
					continue;
				}
				// `is_requires_assignment` guarantees an `=` on this line.
				let equals = line.find('=').ok_or_else(|| {
					anyhow::anyhow!("Malformed requires assignment in conanfile.py")
				})?;
				let original_value = &line[equals + 1..];
				output.push(format!(
					"{} {}",
					&line[..=equals],
					render_python_requires(original_value, &remaining)
				));
				// The rewritten value is a single line; drop the old
				// continuation lines so the file stays valid Python.
				skip_through = *class_end;
				continue;
			}
			if method_lines.iter().any(|(method_index, dependency)| {
				*method_index == index
					&& removed
						.iter()
						.any(|removed| conan_base_name(removed) == conan_base_name(dependency))
			}) {
				continue;
			}
			output.push((*line).to_string());
		}

		let mut rendered = output.join("\n");
		if !rendered.ends_with('\n') {
			rendered.push('\n');
		}
		write_atomic(Path::new(conanfile_path), &rendered)
			.context("Failed to write conanfile.py")?;
		println!("Removed dependencies from conanfile.py: {:?}", removed);
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
		if !json.is_object() {
			bail!("vcpkg.json root must be an object");
		}

		if json.get("dependencies").is_none() {
			json["dependencies"] = serde_json::json!([]);
		} else if !json["dependencies"].is_array() {
			bail!("vcpkg.json dependencies must be an array");
		}
		let dependencies = json["dependencies"]
			.as_array_mut()
			.ok_or_else(|| anyhow::anyhow!("Failed to get dependencies array as mutable"))?;

		let mut added = Vec::new();
		for dep in dep_names {
			let dep_str = dep.as_str();

			let (package_name, version) = parse_vcpkg_dependency(dep_str)?;
			let existing = dependencies.iter().find(|entry| {
				entry
					.as_str()
					.is_some_and(|name| normalize_vcpkg_name(name) == package_name)
					|| entry.as_object().is_some_and(|object| {
						object
							.get("name")
							.and_then(|name| name.as_str())
							.is_some_and(|name| normalize_vcpkg_name(name) == package_name)
					})
			});
			if let Some(existing) = existing {
				if let Some(requested_version) = version.as_deref() {
					let existing_version = existing
						.as_object()
						.and_then(|object| object.get("version>="))
						.and_then(|version| version.as_str());
					if existing_version != Some(requested_version) {
						bail!(
							"vcpkg dependency '{}' is already present with a different version constraint",
							package_name
						);
					}
				}
				continue;
			}

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

		if !added.is_empty() {
			write_atomic(Path::new(vcpkg_path), &serde_json::to_string_pretty(&json)?)
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
		if !json.is_object() {
			bail!("vcpkg.json root must be an object");
		}

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
			let should_remove = dep_names
				.iter()
				.any(|dependency| vcpkg_dependency_matches(d, dependency));
			if should_remove {
				removed.push(d.to_string());
			}
			!should_remove
		});

		if removed.is_empty() {
			bail!("Dependencies not found in vcpkg.json: {:?}", dep_names);
		}

		write_atomic(Path::new(vcpkg_path), &serde_json::to_string_pretty(&json)?)
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
		let makefile_path = match build_file_for(BuildSystem::Makefile) {
			Ok(path) if path.is_file() => path,
			_ => {
				println!("No Makefile found.");
				return Ok(());
			}
		};

		let makefile = Makefile::from_file(&makefile_path)?;

		if let Some(install_deps) = makefile.rules.get("install-deps") {
			if let Some(cmd) = install_deps.commands.first() {
				println!("📦 Makefile Dependencies:");
				// Extract and print dependencies - handle various package managers
				let parts: Vec<&str> = cmd.split_whitespace().collect();
				let start = parts
					.iter()
					.position(|part| matches!(*part, "install" | "-S" | "add"))
					.map(|index| index + 1)
					.unwrap_or(0);
				for dep in parts.iter().skip(start) {
					if !dep.starts_with('-')
						&& dep.chars().all(|character| {
							character.is_ascii_alphanumeric()
								|| matches!(character, '_' | '-' | '.')
						}) {
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
		let cmake_path = match build_file_for(BuildSystem::CMake) {
			Ok(path) if path.is_file() => path,
			_ => {
				println!("No CMakeLists.txt found.");
				return Ok(());
			}
		};

		let cmake = CMakeDependencyManager::from_file(&cmake_path)?;

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
			} else if in_requires_section {
				let trimmed = line.trim();
				if !trimmed.is_empty() && !trimmed.starts_with('#') {
					println!("  • {}", trimmed);
					found_any = true;
				}
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
		let mut in_requirements = false;
		let lines: Vec<&str> = content.lines().collect();
		let owned: Vec<String> = lines.iter().map(|line| line.to_string()).collect();

		for (index, line) in lines.iter().enumerate() {
			let trimmed = line.trim();

			// Check for class-level requires = ... (multiline-aware)
			if !in_requirements && is_requires_assignment(trimmed) {
				let (value, _) = collect_assignment_value(&owned, index);
				for dep in extract_python_strings(&value) {
					if !dep.is_empty() {
						println!("  • {}", dep);
						found_any = true;
					}
				}
			}

			if trimmed.starts_with("def requirements") && trimmed.contains('(') {
				in_requirements = true;
			} else if in_requirements && trimmed.starts_with("def ") {
				in_requirements = false;
			}

			// Parse self.requires(...) calls within requirements method
			if in_requirements && trimmed.contains("self.requires") {
				let (block, _) = collect_call_block(&owned, index);
				if let Some(dep) = parse_self_requires(&block) {
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

		let empty_vec = Vec::new();
		let dependencies = match json.get("dependencies") {
			Some(value) => value
				.as_array()
				.ok_or_else(|| anyhow::anyhow!("vcpkg.json dependencies must be an array"))?,
			None => &empty_vec,
		};

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

/// Normalize `name@version` shorthand into Conan ref syntax
/// (`name/version`). Specs already containing `/` — i.e. full refs like
/// `name/version@user/channel` — pass through unchanged.
fn conan_specs(dep_names: &[String]) -> Vec<String> {
	dep_names
		.iter()
		.map(|dep| {
			if dep.contains('/') {
				return dep.clone();
			}
			match dep.split_once('@') {
				Some((name, version)) if !name.is_empty() && !version.is_empty() => {
					format!("{name}/{version}")
				}
				_ => dep.clone(),
			}
		})
		.collect()
}

fn conan_base_name(value: &str) -> &str {
	value.split('/').next().unwrap_or(value)
}

fn render_python_requires(original_value: &str, dependencies: &[String]) -> String {
	let value = original_value.trim();
	let rendered = dependencies
		.iter()
		.map(|dependency| format!("\"{dependency}\""))
		.collect::<Vec<_>>()
		.join(", ");
	if value.starts_with('[') {
		format!("[{rendered}]")
	} else if value.starts_with('(') {
		// Preserve tuple syntax, including trailing comma for singletons.
		if dependencies.len() == 1 {
			format!("({rendered},)")
		} else {
			format!("({rendered})")
		}
	} else {
		format!("\"{}\"", dependencies.join("\", \""))
	}
}

/// Extracts Python string literals from `value`, handling single/double/
/// triple quotes, escapes, trailing commas, tuples/lists, and `#` comments.
fn is_requires_assignment(trimmed: &str) -> bool {
	let Some(rest) = trimmed.strip_prefix("requires") else {
		return false;
	};
	match rest.chars().next() {
		Some('=') => true,
		Some(character) if character.is_whitespace() => rest.contains('='),
		_ => false,
	}
}

/// Collects the right-hand side of a `requires = ...` assignment starting at
/// `start`, joining continued lines until brackets/parens balance. Returns
/// the joined value and the last line index of the block.
fn collect_assignment_value(lines: &[String], start: usize) -> (String, usize) {
	let line = &lines[start];
	let eq_pos = match line.find('=') {
		Some(pos) => pos,
		None => return (String::new(), start),
	};
	let mut value = line[eq_pos + 1..].to_string();
	let mut end = start;
	let mut depth_square = 0i32;
	let mut depth_paren = 0i32;
	let mut in_single = false;
	let mut in_double = false;
	let scan = |text: &str,
	            depth_square: &mut i32,
	            depth_paren: &mut i32,
	            in_single: &mut bool,
	            in_double: &mut bool| {
		let bytes = text.as_bytes();
		let mut i = 0;
		while i < bytes.len() {
			let byte = bytes[i];
			if *in_single {
				if byte == b'\'' {
					*in_single = false;
				} else if byte == b'\\' {
					i += 1;
				}
			} else if *in_double {
				if byte == b'"' {
					*in_double = false;
				} else if byte == b'\\' {
					i += 1;
				}
			} else {
				match byte {
					b'\'' => *in_single = true,
					b'"' => *in_double = true,
					b'#' => break,
					b'[' => *depth_square += 1,
					b']' => *depth_square -= 1,
					b'(' => *depth_paren += 1,
					b')' => *depth_paren -= 1,
					_ => {}
				}
			}
			i += 1;
		}
	};
	scan(
		&value,
		&mut depth_square,
		&mut depth_paren,
		&mut in_single,
		&mut in_double,
	);
	while (depth_square > 0 || depth_paren > 0 || in_single || in_double) && end + 1 < lines.len() {
		end += 1;
		value.push('\n');
		value.push_str(&lines[end]);
		scan(
			&lines[end],
			&mut depth_square,
			&mut depth_paren,
			&mut in_single,
			&mut in_double,
		);
	}
	(value, end)
}

/// Collects a `self.requires(...)` call starting at `start`, joining lines
/// until parens balance. Returns the joined block and end line index.
fn collect_call_block(lines: &[String], start: usize) -> (String, usize) {
	let mut block = lines[start].clone();
	let mut end = start;
	let mut depth = 0i32;
	let mut in_single = false;
	let mut in_double = false;
	let scan = |text: &str, depth: &mut i32, in_single: &mut bool, in_double: &mut bool| {
		let bytes = text.as_bytes();
		let mut i = 0;
		while i < bytes.len() {
			let byte = bytes[i];
			if *in_single {
				if byte == b'\'' {
					*in_single = false;
				} else if byte == b'\\' {
					i += 1;
				}
			} else if *in_double {
				if byte == b'"' {
					*in_double = false;
				} else if byte == b'\\' {
					i += 1;
				}
			} else {
				match byte {
					b'\'' => *in_single = true,
					b'"' => *in_double = true,
					b'#' => break,
					b'(' => *depth += 1,
					b')' => *depth -= 1,
					_ => {}
				}
			}
			i += 1;
		}
	};
	scan(&block, &mut depth, &mut in_single, &mut in_double);
	while depth > 0 && end + 1 < lines.len() {
		end += 1;
		block.push('\n');
		block.push_str(&lines[end]);
		scan(&lines[end], &mut depth, &mut in_single, &mut in_double);
	}
	(block, end)
}

/// True when `chars` at `index` begins `run` repetitions of `quote`.
fn starts_with_run(chars: &[char], index: usize, quote: char, run: usize) -> bool {
	index + run <= chars.len() && chars[index..index + run].iter().all(|c| *c == quote)
}

fn extract_python_strings(value: &str) -> Vec<String> {
	let mut out = Vec::new();
	// Operate on chars so multi-byte UTF-8 can never be split at a non-boundary.
	let chars: Vec<char> = value.chars().collect();
	let mut index = 0;
	while index < chars.len() {
		// Skip comments outside strings.
		if chars[index] == '#' {
			while index < chars.len() && chars[index] != '\n' {
				index += 1;
			}
			continue;
		}
		// Triple quotes first.
		let is_triple = (chars[index] == '"' || chars[index] == '\'')
			&& starts_with_run(&chars, index, chars[index], 3);
		if is_triple {
			let quote = chars[index];
			let mut end = index + 3;
			while end < chars.len() {
				if chars[end] == '\\' {
					end += 2;
					continue;
				}
				if starts_with_run(&chars, end, quote, 3) {
					break;
				}
				end += 1;
			}
			let literal: String = chars[index + 3..end.min(chars.len())].iter().collect();
			if !literal.trim().is_empty() {
				out.push(literal);
			}
			index = (end + 3).min(chars.len());
			continue;
		}
		if chars[index] == '"' || chars[index] == '\'' {
			let quote = chars[index];
			let mut end = index + 1;
			let mut literal = String::new();
			while end < chars.len() {
				if chars[end] == '\\' && end + 1 < chars.len() {
					match chars[end + 1] {
						'n' => literal.push('\n'),
						't' => literal.push('\t'),
						other => literal.push(other),
					}
					end += 2;
					continue;
				}
				if chars[end] == quote {
					break;
				}
				literal.push(chars[end]);
				end += 1;
			}
			if !literal.trim().is_empty() {
				out.push(literal);
			}
			index = end + 1;
			continue;
		}
		index += 1;
	}
	out
}

fn parse_python_requirements(value: &str) -> Vec<String> {
	let strings = extract_python_strings(value);
	if !strings.is_empty() {
		return strings;
	}
	// Fallback for unquoted edge cases: split on commas.
	value
		.trim()
		.trim_start_matches('[')
		.trim_end_matches(']')
		.trim_matches(|character| character == '"' || character == '\'')
		.split(',')
		.map(|item| {
			item.trim()
				.trim_matches(|character| character == '"' || character == '\'')
				.to_string()
		})
		.filter(|item| !item.is_empty())
		.collect()
}

fn parse_self_requires(line: &str) -> Option<String> {
	let start = line.find("self.requires(")? + "self.requires(".len();
	let rest = &line[start..];
	// Extract the first string literal (handles extra args like
	// `self.requires("foo", override=True)` and multiline calls when the
	// caller passes a joined block).
	extract_python_strings(rest).into_iter().next()
}

fn build_dependency_names(
	package_manager: PackageManager,
	dependencies: &[String],
) -> Result<Vec<String>> {
	let mut names = Vec::new();
	for dependency in dependencies {
		let name = match package_manager {
			PackageManager::Conan => dependency
				.split('/')
				.next()
				.unwrap_or(dependency)
				.split('@')
				.next()
				.unwrap_or(dependency)
				.to_string(),
			PackageManager::Vcpkg => {
				let (name, _) = parse_vcpkg_dependency(dependency)?;
				name.split('[').next().unwrap_or(&name).to_string()
			}
		};
		let name = name.as_str();
		let name = name.split(':').next().unwrap_or(name);
		if !name.is_empty() && !names.iter().any(|existing| existing == name) {
			names.push(name.to_string());
		}
	}
	Ok(names)
}

fn makefile_assignment(content: &str, name: &str) -> Option<String> {
	for line in content.lines() {
		let trimmed = line.trim_start();
		let Some(after_name) = trimmed.strip_prefix(name) else {
			continue;
		};
		let after_name = after_name.trim_start();
		if let Some(value) = after_name
			.strip_prefix(":=")
			.or_else(|| after_name.strip_prefix("::="))
			.or_else(|| after_name.strip_prefix("?="))
			.or_else(|| after_name.strip_prefix("+="))
			.or_else(|| after_name.strip_prefix('='))
		{
			return Some(value.trim().to_string());
		}
	}
	None
}

fn set_makefile_assignment(content: &mut String, name: &str, value: &str) {
	let mut lines = content.lines().map(str::to_string).collect::<Vec<_>>();
	for line in &mut lines {
		let trimmed = line.trim_start();
		let Some(after_name) = trimmed.strip_prefix(name) else {
			continue;
		};
		let after_name = after_name.trim_start();
		if after_name.starts_with(":=")
			|| after_name.starts_with("::=")
			|| after_name.starts_with("?=")
			|| after_name.starts_with("+=")
			|| after_name.starts_with('=')
		{
			let indent = &line[..line.len() - trimmed.len()];
			*line = format!("{}{} = {}", indent, name, value);
			*content = lines.join("\n");
			return;
		}
	}
	lines.push(format!("{} = {}", name, value));
	*content = lines.join("\n");
}
fn makefile_link_flags(dependency: &str) -> Vec<String> {
	let mut name = dependency.trim();
	for separator in ['/', ':', '>', '@'] {
		if let Some((base, _)) = name.split_once(separator) {
			name = base;
		}
	}
	if let Some((base, _)) = name.split_once('=') {
		name = base;
	}
	let name = name.split('[').next().unwrap_or(name);
	let name = if name.len() > 3 && name.to_ascii_lowercase().starts_with("lib") {
		&name[3..]
	} else {
		name
	};
	let lower_name = name.to_ascii_lowercase();
	let flags = match lower_name.as_str() {
		"openssl" => vec!["-lssl", "-lcrypto"],
		"curl" | "libcurl" => vec!["-lcurl"],
		"ssl" => vec!["-lssl"],
		"crypto" => vec!["-lcrypto"],
		"zlib" | "z" => vec!["-lz"],
		"sqlite" | "sqlite3" => vec!["-lsqlite3"],
		"pthread" | "threads" => vec!["-lpthread"],
		"fmt" => vec!["-lfmt"],
		"yaml" | "yaml-cpp" => vec!["-lyaml-cpp"],
		"m" | "libm" => vec!["-lm"],
		"dl" => vec!["-ldl"],
		"rt" => vec!["-lrt"],
		"png" | "libpng" => vec!["-lpng"],
		"jpeg" | "libjpeg" => vec!["-ljpeg"],
		"xml2" | "libxml2" => vec!["-lxml2"],
		"json-c" | "json" => vec!["-ljson-c"],
		"ssh2" | "libssh2" => vec!["-lssh2"],
		"pq" | "libpq" | "postgres" | "postgresql" => vec!["-lpq"],
		"bz2" | "bzip2" => vec!["-lbz2"],
		"lzma" => vec!["-llzma"],
		"archive" | "libarchive" => vec!["-larchive"],
		"gtest" | "googletest" => vec!["-lgtest", "-lgtest_main", "-lpthread"],
		"pthread-stubs" => vec!["-lpthread"],
		_ => return Vec::new(),
	};
	flags.into_iter().map(str::to_string).collect()
}
fn append_makefile_link_flags(content: &mut String, dependencies: &[String]) {
	let mut flags = makefile_assignment(content, "LDLIBS")
		.unwrap_or_default()
		.split_whitespace()
		.map(str::to_string)
		.collect::<Vec<_>>();
	for dependency in dependencies {
		let mapped = makefile_link_flags(dependency);
		if mapped.is_empty() {
			println!(
				"⚠ No automatic LDLIBS mapping for '{}'; install-deps was updated but no linker flag was added. Add it to LDLIBS manually if needed.",
				dependency
			);
			continue;
		}
		for flag in mapped {
			if !flags.contains(&flag) {
				flags.push(flag);
			}
		}
	}
	set_makefile_assignment(content, "LDLIBS", &flags.join(" "));
}

fn remove_makefile_link_flags(content: &mut String, dependencies: &[String]) {
	let requested = dependencies
		.iter()
		.flat_map(|dependency| makefile_link_flags(dependency))
		.collect::<std::collections::HashSet<_>>();
	let flags = makefile_assignment(content, "LDLIBS")
		.unwrap_or_default()
		.split_whitespace()
		.filter(|flag| !requested.contains(*flag))
		.collect::<Vec<_>>()
		.join(" ");
	set_makefile_assignment(content, "LDLIBS", &flags);
}

fn package_manager_manifest_path(package_manager: PackageManager) -> Result<PathBuf> {
	match package_manager {
		PackageManager::Conan => {
			for path in ["conanfile.txt", "conanfile.py"] {
				let path = PathBuf::from(path);
				if path.is_file() {
					return Ok(path);
				}
			}
			bail!("No conanfile.txt or conanfile.py found in current directory")
		}
		PackageManager::Vcpkg => Ok(PathBuf::from("vcpkg.json")),
	}
}

fn read_optional_file(path: &Path) -> Result<Option<String>> {
	match fs::read_to_string(path) {
		Ok(content) => Ok(Some(content)),
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
		Err(error) => Err(error).with_context(|| format!("Failed to read {}", path.display())),
	}
}

fn restore_file(path: &Path, original: Option<&str>) -> Result<()> {
	if let Some(content) = original {
		write_atomic(path, content)
			.with_context(|| format!("Failed to restore {}", path.display()))?;
	} else if path.exists() {
		fs::remove_file(path)
			.with_context(|| format!("Failed to remove partially written {}", path.display()))?;
	}
	Ok(())
}

fn build_file_for(build_system: BuildSystem) -> Result<PathBuf> {
	let path = detect_build_file()?.ok_or_else(|| anyhow::anyhow!("No build system detected"))?;
	let is_cmake = path
		.file_name()
		.is_some_and(|name| name == "CMakeLists.txt");
	if (build_system == BuildSystem::CMake) == is_cmake {
		Ok(path)
	} else {
		bail!("{} not found in the current directory", build_system)
	}
}

fn validate_conan_dependency(value: &str) -> Result<()> {
	if value.is_empty()
		|| value.starts_with('-')
		|| value.len() > 128
		|| value.chars().any(|character| {
			character.is_control()
				|| character.is_whitespace()
				|| matches!(character, '"' | '\'' | ';' | '$' | '`' | '\\' | '#')
		}) {
		bail!("Invalid Conan dependency: {}", value);
	}
	Ok(())
}

fn vcpkg_dependency_matches(value: &serde_json::Value, requested: &str) -> bool {
	let Ok((name, version)) = parse_vcpkg_dependency(requested) else {
		return value.as_str() == Some(requested);
	};
	if value.as_str() == Some(name.as_str()) {
		return true;
	}
	let Some(object) = value.as_object() else {
		return false;
	};
	if object.get("name").and_then(|value| value.as_str()) != Some(name.as_str()) {
		return false;
	}
	version.is_none_or(|version| {
		object.get("version>=").and_then(|value| value.as_str()) == Some(version.as_str())
	})
}

fn normalize_vcpkg_name(value: &str) -> String {
	let mut base = value;
	let mut suffix = String::new();
	if let Some((name, rest)) = value.split_once(':') {
		base = name;
		suffix.push(':');
		suffix.push_str(rest);
	}
	if let Some((name, rest)) = base.split_once('[') {
		base = name;
		suffix = format!("[{}]{}", rest, suffix);
	}
	let mut normalized = base.to_ascii_lowercase();
	if normalized == "libcurl" {
		normalized = "curl".to_string();
	}
	format!("{}{}", normalized, suffix)
}

fn parse_vcpkg_dependency(value: &str) -> Result<(String, Option<String>)> {
	if value.is_empty()
		|| value.chars().any(|character| {
			character.is_whitespace()
				|| matches!(
					character,
					'"' | '\'' | '\\' | ';' | '$' | '`' | '#' | '\n' | '\r'
				)
		}) {
		bail!("Invalid vcpkg dependency: {}", value);
	}

	// `name@version` is the sticks shorthand; it maps to vcpkg's
	// `version>=` constraint, the least surprising interpretation.
	if let Some((name, version)) = value.split_once('@') {
		if name.is_empty() || version.is_empty() {
			bail!("Invalid vcpkg dependency: {}", value);
		}
		return Ok((normalize_vcpkg_name(name), Some(version.to_string())));
	}
	if let Some((name, version)) = value.split_once(">=") {
		if name.is_empty() || version.is_empty() {
			bail!("Invalid vcpkg dependency: {}", value);
		}
		return Ok((normalize_vcpkg_name(name), Some(version.to_string())));
	}
	if value.contains(['>', '<', '=']) {
		bail!("Unsupported vcpkg version constraint: {}", value);
	}

	Ok((normalize_vcpkg_name(value), None))
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
