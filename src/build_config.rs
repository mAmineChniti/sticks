use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::str::FromStr;

/// Represents C++ standard versions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CppStandard {
	Cpp11,
	Cpp14,
	Cpp17,
	Cpp20,
	Cpp23,
}

impl FromStr for CppStandard {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self> {
		match s.to_lowercase().as_str() {
			"c++11" | "cpp11" | "11" => Ok(CppStandard::Cpp11),
			"c++14" | "cpp14" | "14" => Ok(CppStandard::Cpp14),
			"c++17" | "cpp17" | "17" => Ok(CppStandard::Cpp17),
			"c++20" | "cpp20" | "20" => Ok(CppStandard::Cpp20),
			"c++23" | "cpp23" | "23" => Ok(CppStandard::Cpp23),
			_ => bail!(
				"Invalid C++ standard: {}. Valid options: 11, 14, 17, 20, 23",
				s
			),
		}
	}
}

impl CppStandard {
	pub fn to_cmake_flag(&self) -> &'static str {
		match self {
			CppStandard::Cpp11 => "11",
			CppStandard::Cpp14 => "14",
			CppStandard::Cpp17 => "17",
			CppStandard::Cpp20 => "20",
			CppStandard::Cpp23 => "23",
		}
	}

	pub fn to_gcc_flag(&self) -> &'static str {
		match self {
			CppStandard::Cpp11 => "-std=c++11",
			CppStandard::Cpp14 => "-std=c++14",
			CppStandard::Cpp17 => "-std=c++17",
			CppStandard::Cpp20 => "-std=c++20",
			CppStandard::Cpp23 => "-std=c++23",
		}
	}

	pub fn as_str(&self) -> &'static str {
		match self {
			CppStandard::Cpp11 => "C++11",
			CppStandard::Cpp14 => "C++14",
			CppStandard::Cpp17 => "C++17",
			CppStandard::Cpp20 => "C++20",
			CppStandard::Cpp23 => "C++23",
		}
	}
}

/// Represents compiler flags
#[derive(Debug, Clone, Default)]
pub struct CompilerFlags {
	pub flags: Vec<String>,
}

impl CompilerFlags {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add_flag(&mut self, flag: &str) {
		if !self.flags.contains(&flag.to_string()) {
			self.flags.push(flag.to_string());
		}
	}

	pub fn remove_flag(&mut self, flag: &str) {
		self.flags.retain(|f| f != flag);
	}
}

impl std::fmt::Display for CompilerFlags {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.flags.join(" "))
	}
}

/// Represents preprocessor definitions
#[derive(Debug, Clone, Default)]
pub struct PreprocessorDefs {
	pub defs: Vec<String>,
}

impl PreprocessorDefs {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add_def(&mut self, def: &str) {
		if !self.defs.contains(&def.to_string()) {
			self.defs.push(def.to_string());
		}
	}

	pub fn remove_def(&mut self, def: &str) {
		self.defs.retain(|d| d != def);
	}

	pub fn to_cmake_string(&self) -> String {
		self.defs
			.iter()
			.map(|d| format!("{}{}", if d.contains('=') { "" } else { "-D" }, d))
			.collect::<Vec<_>>()
			.join(" ")
	}

	pub fn to_gcc_string(&self) -> String {
		self.defs
			.iter()
			.map(|d| format!("-D{}", d))
			.collect::<Vec<_>>()
			.join(" ")
	}
}

/// Represents include directories
#[derive(Debug, Clone, Default)]
pub struct IncludeDirs {
	pub dirs: Vec<String>,
}

impl IncludeDirs {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add_dir(&mut self, dir: &str) {
		if !self.dirs.contains(&dir.to_string()) {
			self.dirs.push(dir.to_string());
		}
	}

	pub fn remove_dir(&mut self, dir: &str) {
		self.dirs.retain(|d| d != dir);
	}

	pub fn to_cmake_string(&self) -> String {
		self.dirs
			.iter()
			.map(|d| {
				let is_absolute = Path::new(d).is_absolute();
				format!(
					"{}{}",
					if is_absolute {
						""
					} else {
						"${CMAKE_CURRENT_SOURCE_DIR}/"
					},
					d
				)
			})
			.collect::<Vec<_>>()
			.join(" ")
	}

	pub fn to_gcc_string(&self) -> String {
		self.dirs
			.iter()
			.map(|d| format!("-I{}", d))
			.collect::<Vec<_>>()
			.join(" ")
	}
}

/// Represents library directories
#[derive(Debug, Clone, Default)]
pub struct LibraryDirs {
	pub dirs: Vec<String>,
}

impl LibraryDirs {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add_dir(&mut self, dir: &str) {
		if !self.dirs.contains(&dir.to_string()) {
			self.dirs.push(dir.to_string());
		}
	}

	pub fn remove_dir(&mut self, dir: &str) {
		self.dirs.retain(|d| d != dir);
	}

	pub fn to_cmake_string(&self) -> String {
		self.dirs
			.iter()
			.map(|d| {
				let is_absolute = Path::new(d).is_absolute();
				format!(
					"{}{}",
					if is_absolute {
						""
					} else {
						"${CMAKE_CURRENT_SOURCE_DIR}/"
					},
					d
				)
			})
			.collect::<Vec<_>>()
			.join(" ")
	}

	pub fn to_gcc_string(&self) -> String {
		self.dirs
			.iter()
			.map(|d| format!("-L{}", d))
			.collect::<Vec<_>>()
			.join(" ")
	}
}

/// Build configuration manager
pub struct BuildConfig {
	pub cpp_standard: Option<CppStandard>,
	pub compiler_flags: CompilerFlags,
	pub preprocessor_defs: PreprocessorDefs,
	pub include_dirs: IncludeDirs,
	pub library_dirs: LibraryDirs,
}

impl BuildConfig {
	pub fn new() -> Self {
		BuildConfig {
			cpp_standard: None,
			compiler_flags: CompilerFlags::new(),
			preprocessor_defs: PreprocessorDefs::new(),
			include_dirs: IncludeDirs::new(),
			library_dirs: LibraryDirs::new(),
		}
	}

	/// Parse existing configuration from CMakeLists.txt
	pub fn parse_from_cmake(cmake_path: &Path) -> Result<Self> {
		let content = fs::read_to_string(cmake_path).context("Failed to read CMakeLists.txt")?;
		let mut config = Self::new();

		// Pre-compile regex patterns
		let std_regex = regex::Regex::new(r"CMAKE_CXX_STANDARD\s+(\d+)")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let flags_regex = regex::Regex::new(r#"CMAKE_CXX_FLAGS\s+"([^"]*)""#)
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let defs_regex = regex::Regex::new(r"target_compile_definitions\([^)]+PRIVATE\s+([^)]+)\)")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let include_regex =
			regex::Regex::new(r"target_include_directories\([^)]+PRIVATE\s+([^)]+)\)")
				.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let lib_regex = regex::Regex::new(r"link_directories\(([^)]+)\)")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;

		// Parse C++ standard
		if let Some(caps) = std_regex.captures(&content)
			&& let Some(std_str) = caps.get(1)
			&& let Ok(std) = CppStandard::from_str(std_str.as_str())
		{
			config.cpp_standard = Some(std);
		}

		// Parse compiler flags from CMAKE_CXX_FLAGS
		if let Some(caps) = flags_regex.captures(&content)
			&& let Some(flags_str) = caps.get(1)
		{
			for flag in flags_str.as_str().split_whitespace() {
				if !flag.is_empty() {
					config.compiler_flags.add_flag(flag);
				}
			}
		}

		// Parse preprocessor definitions from target_compile_definitions
		if let Some(caps) = defs_regex.captures(&content)
			&& let Some(defs_str) = caps.get(1)
		{
			for def in defs_str.as_str().split_whitespace() {
				let def = def.strip_prefix("-D").unwrap_or(def);
				if !def.is_empty() {
					config.preprocessor_defs.add_def(def);
				}
			}
		}

		// Parse include directories from target_include_directories
		if let Some(caps) = include_regex.captures(&content)
			&& let Some(dirs_str) = caps.get(1)
		{
			for dir in dirs_str.as_str().split_whitespace() {
				if !dir.is_empty() {
					config.include_dirs.add_dir(dir);
				}
			}
		}

		// Parse library directories from link_directories
		if let Some(caps) = lib_regex.captures(&content)
			&& let Some(dirs_str) = caps.get(1)
		{
			for dir in dirs_str.as_str().split_whitespace() {
				if !dir.is_empty() {
					config.library_dirs.add_dir(dir);
				}
			}
		}

		Ok(config)
	}

	/// Parse existing configuration from Makefile
	pub fn parse_from_makefile(makefile_path: &Path) -> Result<Self> {
		let content = fs::read_to_string(makefile_path).context("Failed to read Makefile")?;
		let mut config = Self::new();

		// Pre-compile regex patterns
		let std_regex = regex::Regex::new(r"-std=([cC]\+\+)?(\d+)")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let cxxflags_regex = regex::Regex::new(r"CXXFLAGS\s*=[^\n]*")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let d_regex =
			regex::Regex::new(r"-D([^\s]+)").map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let i_regex =
			regex::Regex::new(r"-I([^\s]+)").map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let ldflags_regex = regex::Regex::new(r"LDFLAGS\s*=[^\n]*")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;

		// Parse C++ standard from CXXFLAGS
		if let Some(caps) = std_regex.captures(&content)
			&& let Some(std_str) = caps.get(2)
			&& let Ok(std) = CppStandard::from_str(std_str.as_str())
		{
			config.cpp_standard = Some(std);
		}

		// Parse compiler flags from CXXFLAGS
		if let Some(caps) = cxxflags_regex.captures(&content)
			&& let Some(flags_line) = caps.get(0)
		{
			for flag in flags_line.as_str().split_whitespace() {
				if flag.starts_with('-') && !flag.contains("std=") {
					config.compiler_flags.add_flag(flag);
				}
			}
		}

		// Parse preprocessor definitions from CXXFLAGS (-D flags)
		for caps in d_regex.captures_iter(&content) {
			if let Some(def) = caps.get(1) {
				config.preprocessor_defs.add_def(def.as_str());
			}
		}

		// Parse include directories from CXXFLAGS (-I flags)
		for caps in i_regex.captures_iter(&content) {
			if let Some(dir) = caps.get(1) {
				config.include_dirs.add_dir(dir.as_str());
			}
		}

		// Parse library directories from LDFLAGS
		if let Some(caps) = ldflags_regex.captures(&content)
			&& let Some(ldflags_line) = caps.get(0)
		{
			for flag in ldflags_line.as_str().split_whitespace() {
				if flag.starts_with("-L") {
					let dir = flag.strip_prefix("-L").unwrap_or(flag);
					config.library_dirs.add_dir(dir);
				}
			}
		}

		Ok(config)
	}

	pub fn set_cpp_standard(&mut self, standard: CppStandard) {
		self.cpp_standard = Some(standard);
	}

	pub fn add_compiler_flag(&mut self, flag: &str) {
		self.compiler_flags.add_flag(flag);
	}

	pub fn remove_compiler_flag(&mut self, flag: &str) {
		self.compiler_flags.remove_flag(flag);
	}

	pub fn add_preprocessor_def(&mut self, def: &str) {
		self.preprocessor_defs.add_def(def);
	}

	pub fn remove_preprocessor_def(&mut self, def: &str) {
		self.preprocessor_defs.remove_def(def);
	}

	pub fn add_include_dir(&mut self, dir: &str) {
		self.include_dirs.add_dir(dir);
	}

	pub fn remove_include_dir(&mut self, dir: &str) {
		self.include_dirs.remove_dir(dir);
	}

	pub fn add_library_dir(&mut self, dir: &str) {
		self.library_dirs.add_dir(dir);
	}

	pub fn remove_library_dir(&mut self, dir: &str) {
		self.library_dirs.remove_dir(dir);
	}

	/// Apply configuration to CMakeLists.txt
	pub fn apply_to_cmake(&self, cmake_path: &Path) -> Result<()> {
		let mut content =
			fs::read_to_string(cmake_path).context("Failed to read CMakeLists.txt")?;

		// Pre-compile regex patterns
		let std_regex = regex::Regex::new(r"CMAKE_CXX_STANDARD\s+\d+")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let flags_regex = regex::Regex::new(r#"CMAKE_CXX_FLAGS\s+"([^"]*)""#)
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let defs_regex = regex::Regex::new(r"target_compile_definitions\(([^)]+)\)")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let defs_replace_regex = regex::Regex::new(r"target_compile_definitions\([^)]+\)")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let include_regex = regex::Regex::new(r"target_include_directories\(([^)]+)\)")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let include_replace_regex = regex::Regex::new(r"target_include_directories\([^)]+\)")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let lib_regex = regex::Regex::new(r"link_directories\(([^)]+)\)")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let lib_replace_regex = regex::Regex::new(r"link_directories\([^)]+\)")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;

		// Set C++ standard
		if let Some(std) = self.cpp_standard {
			let cmake_std = format!("CMAKE_CXX_STANDARD {}", std.to_cmake_flag());
			if content.contains("CMAKE_CXX_STANDARD") {
				// Replace existing
				content = std_regex.replace(&content, &cmake_std).to_string();
			} else {
				// Add after project()
				if let Some(project_pos) = content.find("project(")
					&& let Some(newline_pos) = content[project_pos..].find('\n')
				{
					let insert_pos = project_pos + newline_pos + 1;
					content.insert_str(insert_pos, &format!("set({} REQUIRED)\n", cmake_std));
				}
			}
		}

		// Add compiler flags (merge with existing)
		if !self.compiler_flags.flags.is_empty() {
			let flags_str = format!("{}", self.compiler_flags);
			if content.contains("CMAKE_CXX_FLAGS") {
				// Merge with existing flags
				if let Some(caps) = flags_regex.captures(&content)
					&& let Some(existing_flags) = caps.get(1)
				{
					let existing: Vec<String> = existing_flags
						.as_str()
						.split_whitespace()
						.map(|s| s.to_string())
						.collect();
					let new_flags: Vec<String> = self
						.compiler_flags
						.flags
						.iter()
						.filter(|f| {
							**f != "-std=c++11"
								&& **f != "-std=c++14" && **f != "-std=c++17"
								&& **f != "-std=c++20" && **f != "-std=c++23"
						})
						.cloned()
						.collect();
					let merged: Vec<String> = existing
						.iter()
						.chain(new_flags.iter())
						.filter(|f| !f.is_empty())
						.cloned()
						.collect();
					let merged_str = merged.join(" ");
					content = flags_regex
						.replace(&content, &format!("CMAKE_CXX_FLAGS \"{}\"", merged_str))
						.to_string();
				}
			} else {
				if let Some(project_pos) = content.find("project(")
					&& let Some(newline_pos) = content[project_pos..].find('\n')
				{
					let insert_pos = project_pos + newline_pos + 1;
					content.insert_str(
						insert_pos,
						&format!("set(CMAKE_CXX_FLAGS \"{}\")\n", flags_str),
					);
				}
			}
		}

		// Add preprocessor definitions
		if !self.preprocessor_defs.defs.is_empty() {
			let defs_str = self.preprocessor_defs.to_cmake_string();
			if let Some(caps) = defs_regex.captures(&content)
				&& let Some(existing) = caps.get(1)
			{
				// Append to existing target_compile_definitions
				let new_content = format!(
					"target_compile_definitions({} {})",
					existing.as_str(),
					defs_str
				);
				content = defs_replace_regex
					.replace(&content, &new_content)
					.to_string();
			} else if let Some(target_link_pos) = content.find("target_link_libraries(")
				&& let Some(end) = content[target_link_pos..].find(')')
			{
				let insert_pos = target_link_pos + end;
				// Find the first target name (add_executable or add_library)
				let target_name = content
					.lines()
					.find(|l| l.contains("add_executable(") || l.contains("add_library("))
					.and_then(|l| {
						let start = l.find('(').unwrap_or(0);
						let rest = &l[start + 1..];
						rest.split_whitespace().next()
					})
					.unwrap_or("main");
				content.insert_str(
					insert_pos,
					&format!(
						"\ntarget_compile_definitions({} PRIVATE {})",
						target_name, defs_str
					),
				);
			}
		}

		// Add include directories
		if !self.include_dirs.dirs.is_empty() {
			let include_str = self.include_dirs.to_cmake_string();
			if let Some(caps) = include_regex.captures(&content)
				&& let Some(existing) = caps.get(1)
			{
				// Append to existing target_include_directories
				let new_content = format!(
					"target_include_directories({} {})",
					existing.as_str(),
					include_str
				);
				content = include_replace_regex
					.replace(&content, &new_content)
					.to_string();
			} else if let Some(target_link_pos) = content.find("target_link_libraries(")
				&& let Some(end) = content[target_link_pos..].find(')')
			{
				let insert_pos = target_link_pos + end;
				// Find the first target name (add_executable or add_library)
				let target_name = content
					.lines()
					.find(|l| l.contains("add_executable(") || l.contains("add_library("))
					.and_then(|l| {
						let start = l.find('(').unwrap_or(0);
						let rest = &l[start + 1..];
						rest.split_whitespace().next()
					})
					.unwrap_or("main");
				content.insert_str(
					insert_pos,
					&format!(
						"\ntarget_include_directories({} PRIVATE {})",
						target_name, include_str
					),
				);
			}
		}

		// Add library directories
		if !self.library_dirs.dirs.is_empty() {
			let lib_str = self.library_dirs.to_cmake_string();
			if content.contains("link_directories") {
				if let Some(caps) = lib_regex.captures(&content)
					&& let Some(existing) = caps.get(1)
				{
					let new_content =
						format!("link_directories({} {})", existing.as_str(), lib_str);
					content = lib_replace_regex
						.replace(&content, &new_content)
						.to_string();
				}
			} else {
				content.insert_str(0, &format!("link_directories({})\n", lib_str));
			}
		}

		fs::write(cmake_path, content).context("Failed to write CMakeLists.txt")?;
		Ok(())
	}

	/// Apply configuration to Makefile
	pub fn apply_to_makefile(&self, makefile_path: &Path) -> Result<()> {
		let mut content = fs::read_to_string(makefile_path).context("Failed to read Makefile")?;

		// Pre-compile regex patterns
		let cxxflags_regex = regex::Regex::new(r"CXXFLAGS\s*=\s*([^\n]*)")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let cxxflags_replace_regex = regex::Regex::new(r"CXXFLAGS\s*=\s*[^\n]*")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let ldflags_regex = regex::Regex::new(r"LDFLAGS\s*=\s*([^\n]*)")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;
		let ldflags_replace_regex = regex::Regex::new(r"LDFLAGS\s*=\s*[^\n]*")
			.map_err(|e| anyhow::anyhow!("Regex error: {}", e))?;

		// Collect all CXXFLAGS modifications
		let mut cxxflags_needs_update = false;
		let mut final_cxxflags: Vec<String> = Vec::new();

		if content.contains("CXXFLAGS")
			&& let Some(caps) = cxxflags_regex.captures(&content)
			&& let Some(existing_flags) = caps.get(1)
		{
			final_cxxflags = existing_flags
				.as_str()
				.split_whitespace()
				.map(|s| s.to_string())
				.collect();
		}

		// Set C++ standard (remove existing -std flags first)
		if let Some(std) = self.cpp_standard {
			let gcc_flag = std.to_gcc_flag();
			final_cxxflags.retain(|f| !f.starts_with("-std="));
			final_cxxflags.push(gcc_flag.to_string());
			cxxflags_needs_update = true;
		}

		// Add compiler flags (filter out -std flags as they're handled separately)
		if !self.compiler_flags.flags.is_empty() {
			let new_flags: Vec<String> = self
				.compiler_flags
				.flags
				.iter()
				.filter(|f| !f.contains("-std="))
				.cloned()
				.collect();
			for flag in new_flags {
				if !final_cxxflags.contains(&flag) {
					final_cxxflags.push(flag);
				}
			}
			cxxflags_needs_update = true;
		}

		// Add preprocessor definitions
		if !self.preprocessor_defs.defs.is_empty() {
			let defs_str = self.preprocessor_defs.to_gcc_string();
			let new_defs: Vec<String> =
				defs_str.split_whitespace().map(|s| s.to_string()).collect();
			for def in new_defs {
				if !final_cxxflags.contains(&def) {
					final_cxxflags.push(def);
				}
			}
			cxxflags_needs_update = true;
		}

		// Add include directories
		if !self.include_dirs.dirs.is_empty() {
			let include_str = self.include_dirs.to_gcc_string();
			let new_includes: Vec<String> = include_str
				.split_whitespace()
				.map(|s| s.to_string())
				.collect();
			for inc in new_includes {
				if !final_cxxflags.contains(&inc) {
					final_cxxflags.push(inc);
				}
			}
			cxxflags_needs_update = true;
		}

		// Write CXXFLAGS if modified or if it doesn't exist
		if cxxflags_needs_update {
			let new_line = format!("CXXFLAGS = {}", final_cxxflags.join(" "));
			if content.contains("CXXFLAGS") {
				content = cxxflags_replace_regex
					.replace(&content, &new_line)
					.to_string();
			} else {
				content.insert_str(0, &format!("{}\n", new_line));
			}
		} else if !content.contains("CXXFLAGS") && !final_cxxflags.is_empty() {
			// If CXXFLAGS doesn't exist but we have flags to add
			content.insert_str(0, &format!("CXXFLAGS = {}\n", final_cxxflags.join(" ")));
		}

		// Add library directories (merge with existing)
		if !self.library_dirs.dirs.is_empty() {
			if content.contains("LDFLAGS") {
				if let Some(caps) = ldflags_regex.captures(&content)
					&& let Some(existing_flags) = caps.get(1)
				{
					let existing: Vec<String> = existing_flags
						.as_str()
						.split_whitespace()
						.map(|s| s.to_string())
						.collect();

					let lib_str = self.library_dirs.to_gcc_string();
					let new_libs: Vec<String> =
						lib_str.split_whitespace().map(|s| s.to_string()).collect();

					let merged: Vec<String> = existing
						.iter()
						.chain(new_libs.iter())
						.filter(|f| !f.is_empty())
						.cloned()
						.collect();

					let new_line = format!("LDFLAGS = {}", merged.join(" "));
					content = ldflags_replace_regex
						.replace(&content, &new_line)
						.to_string();
				}
			} else {
				let lib_str = self.library_dirs.to_gcc_string();
				content.insert_str(0, &format!("LDFLAGS = {}\n", lib_str));
			}
		}

		fs::write(makefile_path, content).context("Failed to write Makefile")?;
		Ok(())
	}
}

impl Default for BuildConfig {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_cpp_standard_from_str() {
		assert_eq!(CppStandard::from_str("17").unwrap(), CppStandard::Cpp17);
		assert_eq!(CppStandard::from_str("c++20").unwrap(), CppStandard::Cpp20);
		assert!(CppStandard::from_str("invalid").is_err());
	}

	#[test]
	fn test_cpp_standard_flags() {
		assert_eq!(CppStandard::Cpp17.to_cmake_flag(), "17");
		assert_eq!(CppStandard::Cpp20.to_gcc_flag(), "-std=c++20");
	}

	#[test]
	fn test_compiler_flags() {
		let mut flags = CompilerFlags::new();
		flags.add_flag("-O3");
		flags.add_flag("-Wall");
		assert_eq!(flags.flags.len(), 2);
		flags.remove_flag("-O3");
		assert_eq!(flags.flags.len(), 1);
	}

	#[test]
	fn test_preprocessor_defs() {
		let mut defs = PreprocessorDefs::new();
		defs.add_def("DEBUG");
		defs.add_def("VERSION=1.0");
		assert_eq!(defs.defs.len(), 2);
		defs.remove_def("DEBUG");
		assert_eq!(defs.defs.len(), 1);
	}
}
