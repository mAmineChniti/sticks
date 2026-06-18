use crate::file_handler::write_atomic;
use crate::languages::Language;
use anyhow::{Context, Result, bail};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::str::FromStr;

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

#[derive(Debug, Clone, Default)]
pub struct CompilerFlags {
	pub flags: Vec<String>,
}

impl CompilerFlags {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add_flag(&mut self, flag: &str) {
		if !flag.is_empty() && !self.flags.iter().any(|existing| existing == flag) {
			self.flags.push(flag.to_string());
		}
	}

	pub fn remove_flag(&mut self, flag: &str) {
		self.flags.retain(|existing| existing != flag);
	}
}

impl std::fmt::Display for CompilerFlags {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.flags.join(" "))
	}
}

#[derive(Debug, Clone, Default)]
pub struct PreprocessorDefs {
	pub defs: Vec<String>,
}

impl PreprocessorDefs {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add_def(&mut self, def: &str) {
		let normalized = def.strip_prefix("-D").unwrap_or(def);
		if !normalized.is_empty() && !self.defs.iter().any(|existing| existing == normalized) {
			self.defs.push(normalized.to_string());
		}
	}

	pub fn remove_def(&mut self, def: &str) {
		let normalized = def.strip_prefix("-D").unwrap_or(def);
		self.defs.retain(|existing| existing != normalized);
	}

	pub fn to_cmake_string(&self) -> String {
		self.defs.join(" ")
	}

	pub fn to_gcc_string(&self) -> String {
		self.defs
			.iter()
			.map(|def| format!("-D{}", def))
			.collect::<Vec<_>>()
			.join(" ")
	}
}

#[derive(Debug, Clone, Default)]
pub struct IncludeDirs {
	pub dirs: Vec<String>,
}

impl IncludeDirs {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add_dir(&mut self, dir: &str) {
		if !dir.is_empty() && !self.dirs.iter().any(|existing| existing == dir) {
			self.dirs.push(dir.to_string());
		}
	}

	pub fn remove_dir(&mut self, dir: &str) {
		self.dirs.retain(|existing| existing != dir);
	}

	pub fn to_cmake_string(&self) -> String {
		self.dirs
			.iter()
			.map(|dir| cmake_path_expression(dir))
			.collect::<Vec<_>>()
			.join(" ")
	}

	pub fn to_gcc_string(&self) -> String {
		self.dirs
			.iter()
			.map(|dir| format!("-I{}", dir))
			.collect::<Vec<_>>()
			.join(" ")
	}
}

#[derive(Debug, Clone, Default)]
pub struct LibraryDirs {
	pub dirs: Vec<String>,
}

impl LibraryDirs {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add_dir(&mut self, dir: &str) {
		if !dir.is_empty() && !self.dirs.iter().any(|existing| existing == dir) {
			self.dirs.push(dir.to_string());
		}
	}

	pub fn remove_dir(&mut self, dir: &str) {
		self.dirs.retain(|existing| existing != dir);
	}

	pub fn to_cmake_string(&self) -> String {
		self.dirs
			.iter()
			.map(|dir| cmake_path_expression(dir))
			.collect::<Vec<_>>()
			.join(" ")
	}

	pub fn to_gcc_string(&self) -> String {
		self.dirs
			.iter()
			.map(|dir| format!("-L{}", dir))
			.collect::<Vec<_>>()
			.join(" ")
	}
}

pub struct BuildConfig {
	pub language: Option<Language>,
	pub cpp_standard: Option<CppStandard>,
	pub compiler_flags: CompilerFlags,
	pub preprocessor_defs: PreprocessorDefs,
	pub include_dirs: IncludeDirs,
	pub library_dirs: LibraryDirs,
}

impl BuildConfig {
	pub fn new() -> Self {
		Self {
			language: None,
			cpp_standard: None,
			compiler_flags: CompilerFlags::new(),
			preprocessor_defs: PreprocessorDefs::new(),
			include_dirs: IncludeDirs::new(),
			library_dirs: LibraryDirs::new(),
		}
	}

	pub fn parse_from_cmake(cmake_path: &Path) -> Result<Self> {
		let content = fs::read_to_string(cmake_path).context("Failed to read CMakeLists.txt")?;
		let mut config = Self::new();
		config.language = Some(detect_cmake_language(&content));

		if config.language == Some(Language::Cpp)
			&& let Some(value) = extract_cmake_standard(&content, "CMAKE_CXX_STANDARD")
			&& let Ok(standard) = CppStandard::from_str(&value)
		{
			config.cpp_standard = Some(standard);
		}

		let flags_variable = match config.language {
			Some(Language::C) => "CMAKE_C_FLAGS",
			_ => "CMAKE_CXX_FLAGS",
		};
		if let Some(flags) = extract_cmake_string_variable(&content, flags_variable) {
			for flag in flags.split_whitespace() {
				if flag.starts_with('-') && !flag.starts_with("-std=") {
					config.compiler_flags.add_flag(flag);
				}
			}
		}

		for value in extract_target_arguments(&content, "target_compile_definitions") {
			let value = value.trim_matches('"');
			if !value.is_empty() {
				config.preprocessor_defs.add_def(value);
			}
		}
		for value in extract_target_arguments(&content, "target_include_directories") {
			let value = normalize_cmake_path(&value);
			if !value.is_empty() {
				config.include_dirs.add_dir(&value);
			}
		}
		for value in extract_call_arguments(&content, "link_directories") {
			let value = normalize_cmake_path(&value);
			if !value.is_empty() {
				config.library_dirs.add_dir(&value);
			}
		}

		Ok(config)
	}

	pub fn parse_from_makefile(makefile_path: &Path) -> Result<Self> {
		let content = fs::read_to_string(makefile_path).context("Failed to read Makefile")?;
		let mut config = Self::new();
		let (language, variable) = detect_makefile_language(&content);
		config.language = Some(language);
		let flags = makefile_assignment(&content, variable).unwrap_or_default();

		for token in flags.split_whitespace() {
			if let Some(value) = token.strip_prefix("-std=c++") {
				if let Ok(standard) = CppStandard::from_str(value) {
					config.cpp_standard = Some(standard);
				}
			} else if let Some(value) = token.strip_prefix("-D") {
				config.preprocessor_defs.add_def(value);
			} else if let Some(value) = token.strip_prefix("-I") {
				config.include_dirs.add_dir(value);
			} else if token.starts_with('-') {
				config.compiler_flags.add_flag(token);
			}
		}

		if let Some(ldflags) = makefile_assignment(&content, "LDFLAGS") {
			for token in ldflags.split_whitespace() {
				if let Some(value) = token.strip_prefix("-L") {
					config.library_dirs.add_dir(value);
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

	pub fn apply_to_cmake(&self, cmake_path: &Path) -> Result<()> {
		let mut content =
			fs::read_to_string(cmake_path).context("Failed to read CMakeLists.txt")?;
		let language = self
			.language
			.unwrap_or_else(|| detect_cmake_language(&content));
		let flags_variable = match language {
			Language::C => "CMAKE_C_FLAGS",
			Language::Cpp => "CMAKE_CXX_FLAGS",
		};

		if language == Language::C && self.cpp_standard.is_some() {
			bail!("C++ standards cannot be applied to a C CMake project");
		}
		if let Some(standard) = self.cpp_standard {
			set_cmake_standard(&mut content, "CMAKE_CXX_STANDARD", standard.to_cmake_flag());
			ensure_cmake_standard_required(&mut content, "CMAKE_CXX_STANDARD");
		}

		let flags = self.compiler_flags.flags.join(" ");
		let flags_value = if flags.is_empty() {
			format!("${{{}}}", flags_variable)
		} else {
			format!("${{{}}} {}", flags_variable, flags)
		};
		set_cmake_string_variable(&mut content, flags_variable, &flags_value);

		replace_target_call(
			&mut content,
			"target_compile_definitions",
			&self.preprocessor_defs.to_cmake_string(),
		)?;
		replace_target_call(
			&mut content,
			"target_include_directories",
			&self.include_dirs.to_cmake_string(),
		)?;
		replace_call(
			&mut content,
			"link_directories",
			&self.library_dirs.to_cmake_string(),
		)?;

		write_atomic(cmake_path, &content).context("Failed to write CMakeLists.txt")
	}

	pub fn apply_to_makefile(&self, makefile_path: &Path) -> Result<()> {
		let mut content = fs::read_to_string(makefile_path).context("Failed to read Makefile")?;
		let language = self
			.language
			.unwrap_or_else(|| detect_makefile_language(&content).0);
		if language == Language::C && self.cpp_standard.is_some() {
			bail!("C++ standards cannot be applied to a C Makefile project");
		}
		let flags_variable = match language {
			Language::C => "CFLAGS",
			Language::Cpp => "CXXFLAGS",
		};

		let mut flags = Vec::new();
		if let Some(standard) = self.cpp_standard {
			flags.push(standard.to_gcc_flag().to_string());
		}
		flags.extend(self.compiler_flags.flags.iter().cloned());
		flags.extend(
			self.preprocessor_defs
				.defs
				.iter()
				.map(|def| format!("-D{}", def)),
		);
		flags.extend(
			self.include_dirs
				.dirs
				.iter()
				.map(|dir| format!("-I{}", dir)),
		);
		flags.dedup();
		set_makefile_assignment(&mut content, flags_variable, &flags.join(" "));

		let mut ldflags = makefile_assignment(&content, "LDFLAGS")
			.unwrap_or_default()
			.split_whitespace()
			.filter(|token| !token.starts_with("-L"))
			.map(str::to_string)
			.collect::<Vec<_>>();
		ldflags.extend(
			self.library_dirs
				.dirs
				.iter()
				.map(|dir| format!("-L{}", dir)),
		);
		set_makefile_assignment(&mut content, "LDFLAGS", &ldflags.join(" "));

		write_atomic(makefile_path, &content).context("Failed to write Makefile")
	}
}

impl Default for BuildConfig {
	fn default() -> Self {
		Self::new()
	}
}

fn detect_cmake_language(content: &str) -> Language {
	let project = Regex::new(r"(?i)project\s*\([^)]*\b(C|CXX)\b").ok();
	if let Some(project) = project
		&& let Some(value) = project.captures(content)
		&& let Some(language) = value.get(1)
	{
		return if language.as_str().eq_ignore_ascii_case("cxx") {
			Language::Cpp
		} else {
			Language::C
		};
	}
	if content.contains("CMAKE_C_STANDARD") {
		Language::C
	} else {
		Language::Cpp
	}
}

fn detect_makefile_language(content: &str) -> (Language, &'static str) {
	if makefile_assignment(content, "CXXFLAGS").is_some()
		|| makefile_assignment(content, "CXX").is_some()
	{
		(Language::Cpp, "CXXFLAGS")
	} else {
		(Language::C, "CFLAGS")
	}
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
	let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
	for line in &mut lines {
		let trimmed = line.trim_start();
		if let Some(after_name) = trimmed.strip_prefix(name) {
			let after_name = after_name.trim_start();
			if after_name.starts_with(":=")
				|| after_name.starts_with("::=")
				|| after_name.starts_with("?=")
				|| after_name.starts_with("+=")
				|| after_name.starts_with('=')
			{
				let operator = if after_name.starts_with("::=") {
					"::="
				} else if after_name.starts_with(":=") {
					":="
				} else if after_name.starts_with("?=") {
					"?="
				} else if after_name.starts_with("+=") {
					"+="
				} else {
					"="
				};
				let indent = &line[..line.len() - trimmed.len()];
				*line = format!("{}{} {} {}", indent, name, operator, value);
				*content = lines.join("\n");
				return;
			}
		}
	}
	lines.push(format!("{} = {}", name, value));
	*content = lines.join("\n");
}

fn extract_cmake_standard(content: &str, variable: &str) -> Option<String> {
	let pattern = format!(r"(?m)set\(\s*{}\s+([0-9]+)", regex::escape(variable));
	Regex::new(&pattern)
		.ok()?
		.captures(content)?
		.get(1)
		.map(|value| value.as_str().to_string())
}

fn set_cmake_standard(content: &mut String, variable: &str, value: &str) {
	let pattern = format!(r"set\(\s*{}\s+[0-9]+[^)]*\)", regex::escape(variable));
	if let Ok(regex) = Regex::new(&pattern)
		&& let Some(start) = regex.find(content)
	{
		let replacement = format!("set({} {})", variable, value);
		content.replace_range(start.start()..start.end(), &replacement);
		return;
	}
	insert_after_project(content, &format!("set({} {})\n", variable, value));
}

fn ensure_cmake_standard_required(content: &mut String, variable: &str) {
	let required = format!("{}_REQUIRED", variable);
	if !content.contains(&required) {
		insert_after_project(content, &format!("set({} ON)\n", required));
	}
}

fn extract_cmake_string_variable(content: &str, variable: &str) -> Option<String> {
	let pattern = format!(r#"set\(\s*{}\s+"([^"]*)""#, regex::escape(variable));
	Regex::new(&pattern)
		.ok()?
		.captures(content)
		.and_then(|captures| captures.get(1))
		.map(|value| value.as_str().to_string())
}

fn is_cmake_set(line: &str, variable: &str) -> bool {
	let Some(start) = line.find("set(") else {
		return false;
	};
	let rest = &line[start + 4..];
	rest.split_whitespace()
		.next()
		.is_some_and(|name| name == variable)
}

fn replace_cmake_string_value(line: &str, variable: &str, value: &str) -> Option<String> {
	let mut cursor = line.find("set(")? + 4;
	while line
		.as_bytes()
		.get(cursor)
		.is_some_and(u8::is_ascii_whitespace)
	{
		cursor += 1;
	}
	let name_end = cursor + variable.len();
	if line.get(cursor..name_end)? != variable {
		return None;
	}
	cursor = name_end;
	while line
		.as_bytes()
		.get(cursor)
		.is_some_and(u8::is_ascii_whitespace)
	{
		cursor += 1;
	}
	if line.as_bytes().get(cursor) != Some(&b'"') {
		return None;
	}
	let value_start = cursor + 1;
	let value_end = value_start + line[value_start..].find('"')?;
	let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
	Some(format!(
		"{}\"{}\"{}",
		&line[..value_start],
		escaped,
		&line[value_end + 1..]
	))
}

fn set_cmake_string_variable(content: &mut String, variable: &str, value: &str) {
	let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
	for line in &mut lines {
		if is_cmake_set(line, variable) {
			*line = replace_cmake_string_value(line, variable, value)
				.unwrap_or_else(|| format!("set({} \"{}\")", variable, value));
			*content = lines.join("\n");
			return;
		}
	}
	let addition = format!("set({} \"{}\")\n", variable, value);
	let target_position = ["add_executable(", "add_library("]
		.iter()
		.filter_map(|needle| content.find(needle))
		.min();
	if let Some(position) = target_position {
		content.insert_str(position, &addition);
	} else {
		insert_after_project(content, &addition);
	}
}

fn extract_target_arguments(content: &str, command: &str) -> Vec<String> {
	extract_call_arguments(content, command)
}

fn extract_call_arguments(content: &str, command: &str) -> Vec<String> {
	let pattern = format!(r"(?m){}[ \t]*\(([^)]*)\)", regex::escape(command));
	let Some(regex) = Regex::new(&pattern).ok() else {
		return Vec::new();
	};
	regex
		.captures_iter(content)
		.filter_map(|captures| captures.get(1))
		.flat_map(|captures| {
			captures
				.as_str()
				.split_whitespace()
				.skip(if command.starts_with("target_") { 1 } else { 0 })
				.filter(|value| *value != "PRIVATE" && *value != "PUBLIC" && *value != "INTERFACE")
				.map(|value| value.trim_matches('"').to_string())
				.collect::<Vec<_>>()
		})
		.collect()
}

fn normalize_cmake_path(value: &str) -> String {
	let value = value.trim().trim_matches('"');
	for prefix in [
		"${CMAKE_CURRENT_SOURCE_DIR}/",
		"${CMAKE_SOURCE_DIR}/",
		"${PROJECT_SOURCE_DIR}/",
	] {
		if let Some(relative) = value.strip_prefix(prefix) {
			return relative.to_string();
		}
	}
	value.to_string()
}

fn cmake_path_expression(value: &str) -> String {
	let value = normalize_cmake_path(value);
	if Path::new(&value).is_absolute() || value.contains("${") {
		if value.contains(' ') {
			format!("\"{}\"", value)
		} else {
			value
		}
	} else if value.contains(' ') {
		format!("\"${{CMAKE_CURRENT_SOURCE_DIR}}/{}\"", value)
	} else {
		format!("${{CMAKE_CURRENT_SOURCE_DIR}}/{}", value)
	}
}

fn insert_after_project(content: &mut String, addition: &str) {
	if let Some(position) = content.find("project(")
		&& let Some(newline) = content[position..].find('\n')
	{
		let insert_at = position + newline + 1;
		content.insert_str(insert_at, addition);
	} else {
		content.push_str(addition);
	}
}

fn replace_target_call(content: &mut String, command: &str, values: &str) -> Result<()> {
	let pattern = format!(
		r"(?m)^[ \t]*{}[ \t]*\([^)]*\)[ \t]*$",
		regex::escape(command)
	);
	let regex = Regex::new(&pattern).map_err(|error| anyhow::anyhow!(error))?;
	let mut seen_targets = Vec::new();
	let mut output = String::new();

	for line in content.lines() {
		if regex.is_match(line) {
			if let Some(arguments) = cmake_call_tokens(line, command)
				&& let Some(target) = arguments.first()
			{
				seen_targets.push(target.clone());
				if !values.is_empty() {
					let scope = arguments
						.get(1)
						.filter(|value| {
							matches!(value.as_str(), "PUBLIC" | "PRIVATE" | "INTERFACE")
						})
						.cloned()
						.unwrap_or_else(|| "PRIVATE".to_string());
					output.push_str(&format!("{}({} {} {})\n", command, target, scope, values));
				}
			}
		} else {
			output.push_str(line);
			output.push('\n');
		}
	}
	*content = output;

	if !values.is_empty() {
		let mut declarations = Vec::new();
		for (index, line) in content.lines().enumerate() {
			for target_command in ["add_executable", "add_library"] {
				if let Some(target) = cmake_call_tokens(line, target_command)
					.and_then(|arguments| arguments.first().cloned())
				{
					declarations.push((index, target));
				}
			}
		}
		let mut missing = declarations
			.into_iter()
			.filter(|(_, target)| !seen_targets.iter().any(|seen| seen == target))
			.collect::<Vec<_>>();
		missing.sort_by_key(|(index, _)| std::cmp::Reverse(*index));
		for (index, target) in missing {
			let mut lines = content.lines().map(str::to_string).collect::<Vec<_>>();
			lines.insert(
				(index + 1).min(lines.len()),
				format!("{}({} PRIVATE {})", command, target, values),
			);
			*content = lines.join("\n");
		}
		if !content.ends_with('\n') {
			content.push('\n');
		}
	}
	Ok(())
}

fn replace_call(content: &mut String, command: &str, values: &str) -> Result<()> {
	let pattern = format!(
		r"(?m)^[ \t]*{}[ \t]*\([^)]*\)[ \t]*$",
		regex::escape(command)
	);
	let regex = Regex::new(&pattern).map_err(|error| anyhow::anyhow!(error))?;
	let mut found = false;
	let mut output = String::new();
	for line in content.lines() {
		if regex.is_match(line) {
			found = true;
			if !values.is_empty() {
				output.push_str(&format!("{}({})\n", command, values));
			}
		} else {
			output.push_str(line);
			output.push('\n');
		}
	}
	*content = output;
	if !found && !values.is_empty() {
		insert_after_project(content, &format!("{}({})\n", command, values));
	}
	Ok(())
}

fn cmake_call_tokens(line: &str, command: &str) -> Option<Vec<String>> {
	let start = line.find(&format!("{}(", command))? + command.len() + 1;
	let end = line[start..].find(')')? + start;
	Some(
		line[start..end]
			.split_whitespace()
			.map(|value| value.trim_matches('"').to_string())
			.collect(),
	)
}
