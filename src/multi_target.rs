use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::str::FromStr;

/// Represents build target type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetType {
	Executable,
	StaticLibrary,
	SharedLibrary,
}

impl FromStr for TargetType {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self> {
		match s.to_lowercase().as_str() {
			"exe" | "executable" | "app" => Ok(TargetType::Executable),
			"static" | "static-lib" | "a" => Ok(TargetType::StaticLibrary),
			"shared" | "shared-lib" | "so" | "dll" | "dylib" => Ok(TargetType::SharedLibrary),
			_ => bail!(
				"Invalid target type: {}. Valid options: exe, static, shared",
				s
			),
		}
	}
}

impl TargetType {
	pub fn as_str(&self) -> &'static str {
		match self {
			TargetType::Executable => "executable",
			TargetType::StaticLibrary => "static library",
			TargetType::SharedLibrary => "shared library",
		}
	}

	pub fn cmake_target_type(&self) -> &'static str {
		match self {
			TargetType::Executable => "add_executable",
			TargetType::StaticLibrary => "add_library",
			TargetType::SharedLibrary => "add_library",
		}
	}

	pub fn file_extension(&self) -> &'static str {
		match self {
			TargetType::Executable => "",
			TargetType::StaticLibrary => ".a",
			TargetType::SharedLibrary => ".so",
		}
	}
}

/// Represents a build target
#[derive(Debug, Clone)]
pub struct BuildTarget {
	pub name: String,
	pub target_type: TargetType,
	pub sources: Vec<String>,
	pub dependencies: Vec<String>,
}

impl BuildTarget {
	pub fn new(name: String, target_type: TargetType) -> Self {
		BuildTarget {
			name,
			target_type,
			sources: Vec::new(),
			dependencies: Vec::new(),
		}
	}

	pub fn add_source(&mut self, source: String) {
		if !self.sources.contains(&source) {
			self.sources.push(source);
		}
	}

	pub fn add_dependency(&mut self, dep: String) {
		if !self.dependencies.contains(&dep) {
			self.dependencies.push(dep);
		}
	}
}

/// Multi-target project manager
pub struct MultiTargetManager {
	pub targets: Vec<BuildTarget>,
}

impl MultiTargetManager {
	pub fn new() -> Self {
		MultiTargetManager {
			targets: Vec::new(),
		}
	}

	/// Parse existing targets from CMakeLists.txt
	pub fn parse_from_cmake(cmake_path: &Path) -> Result<Self> {
		let content = fs::read_to_string(cmake_path).context("Failed to read CMakeLists.txt")?;
		let mut manager = Self::new();

		// Pre-compile regex patterns
		let exec_regex = regex::Regex::new(r"add_executable\((\S+)\s+(.*)\)")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let lib_regex = regex::Regex::new(r"add_library\((\S+)\s+(.*)\)")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let shared_lib_regex = regex::Regex::new(r"add_library\((\S+)\s+SHARED\s+(.*)\)")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let link_regex = regex::Regex::new(r"target_link_libraries\((\S+)\s+(.*)\)")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;

		// Parse add_executable and add_library
		for line in content.lines() {
			let line = line.trim();

			// Parse add_executable
			if let Some(caps) = exec_regex.captures(line)
				&& let Some(name) = caps.get(1)
			{
				let sources_str = caps.get(2).map(|s| s.as_str()).unwrap_or("");
				let sources: Vec<String> = sources_str
					.split_whitespace()
					.map(|s| s.to_string())
					.collect();

				let mut target =
					BuildTarget::new(name.as_str().to_string(), TargetType::Executable);
				for src in sources {
					target.add_source(src);
				}
				manager.add_target(target);
			}

			// Parse add_library (static) - check line doesn't contain SHARED
			if let Some(caps) = lib_regex.captures(line)
				&& !line.contains("SHARED")
				&& let Some(name) = caps.get(1)
			{
				let sources_str = caps.get(2).map(|s| s.as_str()).unwrap_or("");
				let sources: Vec<String> = sources_str
					.split_whitespace()
					.map(|s| s.to_string())
					.collect();

				let mut target =
					BuildTarget::new(name.as_str().to_string(), TargetType::StaticLibrary);
				for src in sources {
					target.add_source(src);
				}
				manager.add_target(target);
			}

			// Parse add_library SHARED
			if let Some(caps) = shared_lib_regex.captures(line)
				&& let Some(name) = caps.get(1)
			{
				let sources_str = caps.get(2).map(|s| s.as_str()).unwrap_or("");
				let sources: Vec<String> = sources_str
					.split_whitespace()
					.map(|s| s.to_string())
					.collect();

				let mut target =
					BuildTarget::new(name.as_str().to_string(), TargetType::SharedLibrary);
				for src in sources {
					target.add_source(src);
				}
				manager.add_target(target);
			}
		}

		// Parse target_link_libraries for dependencies
		for line in content.lines() {
			if let Some(caps) = link_regex.captures(line)
				&& let Some(name) = caps.get(1)
			{
				let deps_str = caps.get(2).map(|s| s.as_str()).unwrap_or("");
				if let Some(target) = manager.targets.iter_mut().find(|t| t.name == name.as_str()) {
					for dep in deps_str.split_whitespace() {
						target.add_dependency(dep.to_string());
					}
				}
			}
		}

		Ok(manager)
	}

	/// Parse existing targets from Makefile
	pub fn parse_from_makefile(makefile_path: &Path) -> Result<Self> {
		let content = fs::read_to_string(makefile_path).context("Failed to read Makefile")?;
		let mut manager = Self::new();

		// Pre-compile regex patterns
		let target_regex = regex::Regex::new(r"^([^\s:]+):\s*(.*)$")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let lib_regex =
			regex::Regex::new(r"-l([^\s]+)").map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;

		// Parse targets (lines ending with :)
		for line in content.lines() {
			let line = line.trim();

			// Skip comments and special targets
			if line.starts_with('#') || line.starts_with('.') || line == "all:" || line == "clean:"
			{
				continue;
			}

			// Parse target: sources pattern
			if let Some(caps) = target_regex.captures(line)
				&& let Some(name) = caps.get(1)
			{
				let name_str = name.as_str();

				// Determine target type based on name pattern
				let target_type = if name_str.starts_with("lib") && name_str.ends_with(".a") {
					TargetType::StaticLibrary
				} else if name_str.starts_with("lib") && name_str.ends_with(".so") {
					TargetType::SharedLibrary
				} else {
					TargetType::Executable
				};

				let sources_str = caps.get(2).map(|s| s.as_str()).unwrap_or("");
				let sources: Vec<String> = sources_str
					.split_whitespace()
					.map(|s| s.to_string())
					.collect();

				let mut target = BuildTarget::new(name_str.to_string(), target_type);
				for src in sources {
					target.add_source(src);
				}
				manager.add_target(target);
			}
		}

		// Parse dependencies from link flags (-l flags in build rules)
		for line in content.lines() {
			// This is a simplified approach - in a real implementation, you'd need to
			// match dependencies to specific targets
			for caps in lib_regex.captures_iter(line) {
				if let Some(dep) = caps.get(1) {
					// Add to first target as a fallback
					if let Some(target) = manager.targets.first_mut() {
						target.add_dependency(format!("-l{}", dep.as_str()));
					}
				}
			}
		}

		Ok(manager)
	}

	pub fn add_target(&mut self, target: BuildTarget) {
		// Check for duplicate target name
		if self.get_target(&target.name).is_some() {
			return; // Skip adding duplicate
		}
		self.targets.push(target);
	}

	pub fn remove_target(&mut self, name: &str) -> Result<()> {
		let before = self.targets.len();
		self.targets.retain(|t| t.name != name);
		if self.targets.len() == before {
			bail!("Target '{}' not found", name);
		}
		Ok(())
	}

	pub fn get_target(&self, name: &str) -> Option<&BuildTarget> {
		self.targets.iter().find(|t| t.name == name)
	}

	/// Add target to CMakeLists.txt
	pub fn add_to_cmake(&self, cmake_path: &Path) -> Result<()> {
		let mut content =
			fs::read_to_string(cmake_path).context("Failed to read CMakeLists.txt")?;

		for target in &self.targets {
			let cmake_cmd = target.target_type.cmake_target_type();
			let sources_str = target.sources.join(" ");

			let target_def = if target.target_type == TargetType::SharedLibrary {
				format!("{}({} SHARED {})\n", cmake_cmd, target.name, sources_str)
			} else {
				format!("{}({} {})\n", cmake_cmd, target.name, sources_str)
			};

			if !content.contains(&format!("{}({}", cmake_cmd, target.name)) {
				content.push_str(&target_def);
			}

			// Add target_link_libraries if dependencies exist
			if !target.dependencies.is_empty() {
				let link_str = target.dependencies.join(" ");
				let link_def = format!("target_link_libraries({} {})\n", target.name, link_str);
				if !content.contains(&format!("target_link_libraries({}", target.name)) {
					content.push_str(&link_def);
				}
			}
		}

		fs::write(cmake_path, content).context("Failed to write CMakeLists.txt")?;
		Ok(())
	}

	/// Add target to Makefile
	pub fn add_to_makefile(&self, makefile_path: &Path) -> Result<()> {
		let mut content = fs::read_to_string(makefile_path).context("Failed to read Makefile")?;

		for target in &self.targets {
			let sources_str = target.sources.join(" ");
			let deps_str = target.dependencies.join(" ");

			let target_def = match target.target_type {
				TargetType::Executable => format!(
					"{}: {}\n\t$(CXX) $(CXXFLAGS) {} -o {} {}\n",
					target.name, sources_str, sources_str, target.name, deps_str
				),
				TargetType::StaticLibrary => format!(
					"lib{}.a: {}\n\tar rcs lib{}.a {}\n",
					target.name, sources_str, target.name, sources_str
				),
				TargetType::SharedLibrary => format!(
					"lib{}.so: {}\n\t$(CXX) $(CXXFLAGS) -shared -fPIC {} -o lib{}.so {}\n",
					target.name, sources_str, sources_str, target.name, deps_str
				),
			};

			let rule_identifier = match target.target_type {
				TargetType::Executable => target.name.clone(),
				TargetType::StaticLibrary => format!("lib{}.a", target.name),
				TargetType::SharedLibrary => format!("lib{}.so", target.name),
			};

			if !content.contains(&format!("{}:", rule_identifier)) {
				content.push_str(&target_def);
			}
		}

		fs::write(makefile_path, content).context("Failed to write Makefile")?;
		Ok(())
	}
}

impl Default for MultiTargetManager {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_target_type_from_str() {
		assert_eq!(TargetType::from_str("exe").unwrap(), TargetType::Executable);
		assert_eq!(
			TargetType::from_str("static").unwrap(),
			TargetType::StaticLibrary
		);
		assert_eq!(
			TargetType::from_str("shared").unwrap(),
			TargetType::SharedLibrary
		);
		assert!(TargetType::from_str("invalid").is_err());
	}

	#[test]
	fn test_build_target() {
		let mut target = BuildTarget::new("mylib".to_string(), TargetType::StaticLibrary);
		target.add_source("src/lib.cpp".to_string());
		assert_eq!(target.sources.len(), 1);
	}

	#[test]
	fn test_multi_target_manager() {
		let mut manager = MultiTargetManager::new();
		let target = BuildTarget::new("myapp".to_string(), TargetType::Executable);
		manager.add_target(target);
		assert_eq!(manager.targets.len(), 1);
	}
}
