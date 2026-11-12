use crate::cmake_parser::{self, CmakeCall};
use crate::file_handler::write_atomic;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

pub struct CMakeDependencyManager {
	pub content: String,
}

impl CMakeDependencyManager {
	pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
		let content = fs::read_to_string(path).context("Failed to read CMakeLists.txt")?;
		Ok(Self { content })
	}

	pub fn parse(content: &str) -> Self {
		Self {
			content: content.to_string(),
		}
	}

	pub fn add_dependencies(&mut self, deps: &[String]) -> Result<Vec<String>> {
		let mut added = Vec::new();
		for dependency in deps {
			let parsed = DependencySpec::parse(dependency)?;
			let mut changed = self.ensure_find_package(
				&parsed.package,
				parsed.version.as_deref(),
				&parsed.components,
			)?;
			for link_name in parsed.link_names() {
				changed |= self.add_to_target_link_libraries(&link_name)?;
			}
			if changed {
				added.push(dependency.clone());
			}
		}
		Ok(added)
	}

	pub fn remove_dependencies(&mut self, deps: &[String]) -> Result<Vec<String>> {
		let mut removed = Vec::new();
		for dependency in deps {
			let parsed = DependencySpec::parse(dependency)?;
			if !self.remove_find_package(&parsed.package, &parsed.components)? {
				bail!("Package {} not found in CMakeLists.txt", dependency);
			}
			self.remove_from_target_link_libraries(&parsed.package, &parsed.components)?;
			removed.push(dependency.clone());
		}
		Ok(removed)
	}

	fn ensure_find_package(
		&mut self,
		package: &str,
		version: Option<&str>,
		components: &[String],
	) -> Result<bool> {
		let Some(call) = self.find_package_call(package) else {
			if self.project_line().is_none() {
				bail!("CMakeLists.txt does not contain a project() declaration");
			}
			let version_arg = version.map_or_else(String::new, |v| format!(" {}", v));
			let command = if components.is_empty() {
				format!("find_package({}{} REQUIRED)\n", package, version_arg)
			} else {
				format!(
					"find_package({}{} REQUIRED COMPONENTS {})\n",
					package,
					version_arg,
					components.join(" ")
				)
			};
			self.insert_after_project(&command);
			return Ok(true);
		};

		// Merge a requested version into an existing find_package() call.
		// In `find_package(<name> [<version>] ...)`, a version argument is
		// recognized by starting with a digit.
		if let Some(version) = version {
			let args = call.args.clone();
			let existing_version = args
				.get(1)
				.filter(|arg| arg.chars().next().is_some_and(|c| c.is_ascii_digit()))
				.cloned();
			if existing_version.as_deref() != Some(version) {
				let mut replacement = vec![args[0].clone(), version.to_string()];
				replacement.extend(
					args.iter()
						.skip(1)
						.filter(|arg| Some(arg.as_str()) != existing_version.as_deref())
						.cloned(),
				);
				self.replace_call(
					&call,
					&format!("find_package({})", cmake_parser::render_args(&replacement)),
				);
				// Re-resolve the call for the component merge below.
				return self.ensure_find_package(package, None, components);
			}
		}

		if components.is_empty() {
			return Ok(false);
		}

		let arguments = call.args.clone();
		let existing_components = find_component_clause(&arguments, "COMPONENTS")
			.map_or_else(Vec::new, |(start, end)| arguments[start..end].to_vec());
		let mut merged = existing_components.clone();
		for component in components {
			if !merged.contains(component) {
				merged.push(component.clone());
			}
		}
		if merged == existing_components {
			return Ok(false);
		}
		let replacement_arguments = replace_component_clause(&arguments, "COMPONENTS", &merged);
		let replacement = format!(
			"find_package({})",
			cmake_parser::render_args(&replacement_arguments)
		);
		self.replace_call(&call, &replacement);
		Ok(true)
	}

	fn remove_find_package(&mut self, package: &str, components: &[String]) -> Result<bool> {
		let Some(call) = self.find_package_call(package) else {
			return Ok(false);
		};
		if components.is_empty() {
			self.remove_call(&call);
			return Ok(true);
		}

		let arguments = call.args.clone();
		let Some((start, end)) = find_component_clause(&arguments, "COMPONENTS") else {
			bail!("Components are not declared for package {}", package);
		};
		let remaining = arguments[start..end]
			.iter()
			.filter(|value| !components.contains(value))
			.cloned()
			.collect::<Vec<_>>();
		if remaining.len() == end - start {
			bail!(
				"Components {:?} are not declared for package {}",
				components,
				package
			);
		}
		let replacement_arguments = replace_component_clause(&arguments, "COMPONENTS", &remaining);
		let replacement = format!(
			"find_package({})",
			cmake_parser::render_args(&replacement_arguments)
		);
		self.replace_call(&call, &replacement);
		Ok(true)
	}

	fn add_to_target_link_libraries(&mut self, link_name: &str) -> Result<bool> {
		if let Some(call) = self.target_link_call() {
			let mut arguments = call.args.clone();
			if arguments
				.iter()
				.skip(1)
				.any(|value| cmake_link_names_equal(value, link_name))
			{
				return Ok(false);
			}
			arguments.push(link_name.to_string());
			let replacement = format!(
				"target_link_libraries({})",
				cmake_parser::render_args(&arguments)
			);
			self.replace_call(&call, &replacement);
			return Ok(true);
		}

		let Some(target) = self.first_target_name() else {
			return Ok(false);
		};
		let command = format!("\ntarget_link_libraries({} {})\n", target, link_name);
		self.insert_after_target(&target, &command);
		Ok(true)
	}

	fn remove_from_target_link_libraries(
		&mut self,
		package: &str,
		components: &[String],
	) -> Result<()> {
		let calls = cmake_parser::find_calls(&self.content, "target_link_libraries");
		for call in calls.into_iter().rev() {
			let arguments = call.args.clone();
			let filtered = arguments
				.iter()
				.enumerate()
				.filter_map(|(index, value)| {
					if index == 0 {
						return Some(value.clone());
					}
					let remove = if components.is_empty() {
						value == package
							|| value.starts_with(&format!("{}::", package))
							|| cmake_link_names_equal(value, package)
					} else {
						components.iter().any(|component| {
							value == &format!("{}::{}", package, component)
								|| cmake_link_names_equal(
									value,
									&format!("{}::{}", package, component),
								)
						})
					};
					(!remove).then(|| value.to_string())
				})
				.collect::<Vec<_>>();
			if filtered.len() == arguments.len() {
				continue;
			}
			let has_remaining_link = filtered
				.iter()
				.skip(1)
				.any(|value| !matches!(value.as_str(), "PUBLIC" | "PRIVATE" | "INTERFACE"));
			if has_remaining_link {
				self.replace_call(
					&call,
					&format!(
						"target_link_libraries({})",
						cmake_parser::render_args(&filtered)
					),
				);
			} else {
				self.remove_call(&call);
			}
			// Refresh calls after mutation: re-parse on next iteration would be
			// needed for multiple matches, so break and loop via fresh scan.
			// Since we iterate rev over a snapshot, byte offsets may shift;
			// re-resolve by restarting the scan if more matches remain.
			break;
		}
		// Handle any remaining matches (offsets shifted after first edit).
		if cmake_parser::find_calls(&self.content, "target_link_libraries")
			.iter()
			.any(|call| {
				call.args.iter().skip(1).any(|value| {
					if components.is_empty() {
						value == package
							|| value.starts_with(&format!("{}::", package))
							|| cmake_link_names_equal(value, package)
					} else {
						components
							.iter()
							.any(|component| value == &format!("{}::{}", package, component))
					}
				})
			}) {
			self.remove_from_target_link_libraries(package, components)?;
		}
		Ok(())
	}

	fn find_package_call(&self, package: &str) -> Option<CmakeCall> {
		cmake_parser::find_calls(&self.content, "find_package")
			.into_iter()
			.find(|call| {
				call.args
					.first()
					.is_some_and(|name| canonical_cmake_package_name(name) == *package)
			})
	}

	#[allow(dead_code)]
	fn find_package_line(&self, package: &str) -> Option<usize> {
		self.find_package_call(package).map(|call| call.start_line)
	}

	fn target_link_call(&self) -> Option<CmakeCall> {
		let primary_target = self.first_target_name()?;
		cmake_parser::find_calls(&self.content, "target_link_libraries")
			.into_iter()
			.find(|call| {
				call.args
					.first()
					.is_some_and(|target| target == &primary_target)
			})
	}

	#[allow(dead_code)]
	fn target_link_line(&self) -> Option<usize> {
		self.target_link_call().map(|call| call.start_line)
	}

	fn first_target_name(&self) -> Option<String> {
		let mut fallback = None;
		for command in ["add_executable", "add_library"] {
			for call in cmake_parser::find_calls(&self.content, command) {
				let Some(name) = call.args.first().cloned() else {
					continue;
				};
				if name == "ALIAS" || name.contains("::") {
					continue;
				}
				if fallback.is_none() {
					fallback = Some(name.clone());
				}
				if !name.to_ascii_lowercase().contains("test") {
					return Some(name);
				}
			}
		}
		fallback
	}

	fn project_line(&self) -> Option<usize> {
		cmake_parser::find_calls(&self.content, "project")
			.into_iter()
			.next()
			.map(|call| call.start_line)
	}

	fn insert_after_project(&mut self, text: &str) {
		// Insert after the *end* of a possibly-multiline project() call.
		let index = cmake_parser::find_calls(&self.content, "project")
			.into_iter()
			.next()
			.map(|call| call.end_line + 1)
			.or(self.project_line().map(|index| index + 1))
			.unwrap_or(0);
		self.insert_at(index, text);
	}

	fn insert_after_target(&mut self, target_name: &str, text: &str) {
		for command in ["add_executable", "add_library"] {
			if let Some(call) = cmake_parser::find_calls(&self.content, command)
				.into_iter()
				.find(|call| call.args.first().is_some_and(|name| name == target_name))
			{
				self.insert_at(call.end_line + 1, text);
				return;
			}
		}
		let index = self
			.content
			.lines()
			.enumerate()
			.find_map(|(index, line)| {
				(is_cmake_target_line(line, "add_executable", target_name)
					|| is_cmake_target_line(line, "add_library", target_name))
				.then_some(index + 1)
			})
			.or_else(|| {
				cmake_parser::find_calls(&self.content, "add_executable")
					.into_iter()
					.next()
					.map(|call| call.end_line + 1)
			})
			.or_else(|| {
				cmake_parser::find_calls(&self.content, "add_library")
					.into_iter()
					.next()
					.map(|call| call.end_line + 1)
			})
			.unwrap_or_else(|| self.content.lines().count());
		self.insert_at(index, text);
	}

	fn replace_call(&mut self, call: &CmakeCall, text: &str) {
		cmake_parser::replace_span(&mut self.content, call.start_byte, call.end_byte, text);
	}

	fn remove_call(&mut self, call: &CmakeCall) {
		let mut updated = self.content.clone();
		// Remove the span plus one trailing newline when present to avoid
		// leaving a blank line behind.
		let mut end = call.end_byte;
		if updated.as_bytes().get(end) == Some(&b'\n') {
			end += 1;
		} else if updated.as_bytes().get(end) == Some(&b'\r')
			&& updated.as_bytes().get(end + 1) == Some(&b'\n')
		{
			end += 2;
		}
		updated.replace_range(call.start_byte..end, "");
		self.content = updated;
	}

	fn insert_at(&mut self, index: usize, text: &str) {
		let mut lines = self.content.lines().map(str::to_string).collect::<Vec<_>>();
		let index = index.min(lines.len());
		lines.insert(index, text.trim_end_matches('\n').to_string());
		self.content = lines.join("\n");
		if !self.content.ends_with('\n') {
			self.content.push('\n');
		}
	}

	#[allow(dead_code)]
	fn replace_line(&mut self, index: usize, text: &str) {
		let mut lines = self.content.lines().map(str::to_string).collect::<Vec<_>>();
		if index < lines.len() {
			lines[index] = text.to_string();
		}
		self.content = lines.join("\n");
		if !self.content.ends_with('\n') {
			self.content.push('\n');
		}
	}

	#[allow(dead_code)]
	fn remove_line(&mut self, index: usize) {
		let mut lines = self.content.lines().map(str::to_string).collect::<Vec<_>>();
		if index < lines.len() {
			lines.remove(index);
		}
		self.content = lines.join("\n");
		if !self.content.ends_with('\n') {
			self.content.push('\n');
		}
	}

	pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
		write_atomic(path.as_ref(), &self.content).context("Failed to write CMakeLists.txt")
	}
}

struct DependencySpec {
	package: String,
	version: Option<String>,
	components: Vec<String>,
}

impl DependencySpec {
	fn parse(value: &str) -> Result<Self> {
		// `name@version` is sticks' shorthand; it maps to CMake's
		// `find_package(<name> <version> ...)`. The version suffix is
		// stripped before `:` component parsing.
		let (value, version) = match value.rsplit_once('@') {
			Some((name, version))
				if !name.is_empty()
					&& !version.is_empty()
					&& version.chars().all(|c| {
						c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-' | '_')
					}) =>
			{
				(name, Some(version.to_string()))
			}
			_ => (value, None),
		};
		let (package, components) = if let Some((package, component_list)) = value.split_once(':') {
			validate_cmake_identifier(package, "package")?;
			let components = component_list
				.split(',')
				.map(str::trim)
				.map(|component| {
					validate_cmake_identifier(component, "component")?;
					Ok(component.to_string())
				})
				.collect::<Result<Vec<_>>>()?;
			(package, components)
		} else {
			validate_cmake_identifier(value, "package")?;
			(value, Vec::new())
		};
		let package = canonical_cmake_package_name(package);
		if package.is_empty() {
			bail!("Package name cannot normalize to an empty CMake name");
		}
		Ok(Self {
			package,
			version,
			components,
		})
	}

	fn link_names(&self) -> Vec<String> {
		if !self.components.is_empty() {
			return self
				.components
				.iter()
				.map(|component| format!("{}::{}", self.package, component))
				.collect();
		}
		if self.package == "CURL" {
			return vec!["CURL::libcurl".to_string()];
		}
		if self.package == "ZLIB" {
			return vec!["ZLIB::ZLIB".to_string()];
		}
		if self.package == "Boost" {
			return vec!["Boost::headers".to_string()];
		}
		if self.package == "Threads" {
			return vec!["Threads::Threads".to_string()];
		}
		if self.package == "OpenSSL" {
			return vec!["OpenSSL::SSL".to_string(), "OpenSSL::Crypto".to_string()];
		}
		if self.package.eq_ignore_ascii_case("spdlog") {
			return vec!["spdlog::spdlog".to_string()];
		}
		if self.package.eq_ignore_ascii_case("fmt") {
			return vec!["fmt::fmt".to_string()];
		}
		if self.package.eq_ignore_ascii_case("nlohmann_json") {
			return vec!["nlohmann_json::nlohmann_json".to_string()];
		}
		if self.package.eq_ignore_ascii_case("sqlite3") {
			return vec!["SQLite::SQLite3".to_string()];
		}
		if self.package.eq_ignore_ascii_case("libpng") || self.package.eq_ignore_ascii_case("png") {
			return vec!["PNG::PNG".to_string()];
		}
		if self.package.eq_ignore_ascii_case("libjpeg") || self.package.eq_ignore_ascii_case("jpeg")
		{
			return vec!["JPEG::JPEG".to_string()];
		}
		if self.package.eq_ignore_ascii_case("libxml2") || self.package.eq_ignore_ascii_case("xml2")
		{
			return vec!["LibXml2::LibXml2".to_string()];
		}
		if self.package.eq_ignore_ascii_case("gtest")
			|| self.package.eq_ignore_ascii_case("googletest")
		{
			return vec!["GTest::gtest".to_string(), "GTest::gtest_main".to_string()];
		}
		if self.package.eq_ignore_ascii_case("catch2") {
			return vec!["Catch2::Catch2WithMain".to_string()];
		}
		if self.package.eq_ignore_ascii_case("doctest") {
			return vec!["doctest::doctest".to_string()];
		}
		if self.package.eq_ignore_ascii_case("yaml-cpp")
			|| self.package.eq_ignore_ascii_case("yaml")
		{
			return vec!["yaml-cpp".to_string()];
		}
		if self.package.eq_ignore_ascii_case("libarchive")
			|| self.package.eq_ignore_ascii_case("archive")
		{
			return vec!["LibArchive::LibArchive".to_string()];
		}
		if self.package.eq_ignore_ascii_case("libssh2") || self.package.eq_ignore_ascii_case("ssh2")
		{
			return vec!["Libssh2::libssh2".to_string()];
		}
		if self.package.eq_ignore_ascii_case("bzip2") || self.package.eq_ignore_ascii_case("bz2") {
			return vec!["BZip2::BZip2".to_string()];
		}
		vec![self.package.clone()]
	}
}

#[allow(dead_code)]
fn cmake_call_arguments(line: &str, command: &str) -> Result<Vec<String>> {
	cmake_parser::call_arguments(line, command)
}

fn canonical_cmake_package_name(value: &str) -> String {
	let formatted = format_cmake_package_name(value);
	match formatted.as_str() {
		"OPENSSL" => "OpenSSL".to_string(),
		"THREADS" => "Threads".to_string(),
		_ => formatted,
	}
}

fn format_cmake_package_name(dep: &str) -> String {
	if dep.eq_ignore_ascii_case("boost") {
		return "Boost".to_string();
	}
	if dep.eq_ignore_ascii_case("curl") || dep.eq_ignore_ascii_case("libcurl") {
		return "CURL".to_string();
	}
	if dep.eq_ignore_ascii_case("zlib") || dep.eq_ignore_ascii_case("libz") {
		return "ZLIB".to_string();
	}
	if dep.eq_ignore_ascii_case("openssl")
		|| dep.eq_ignore_ascii_case("libssl")
		|| dep.eq_ignore_ascii_case("libcrypto")
	{
		return "OPENSSL".to_string();
	}
	if dep.eq_ignore_ascii_case("threads") {
		return "Threads".to_string();
	}
	if dep.eq_ignore_ascii_case("spdlog") {
		return "spdlog".to_string();
	}
	if dep.eq_ignore_ascii_case("fmt") {
		return "fmt".to_string();
	}
	if dep.eq_ignore_ascii_case("nlohmann_json")
		|| dep.eq_ignore_ascii_case("json")
		|| dep.eq_ignore_ascii_case("nlohmann-json")
	{
		return "nlohmann_json".to_string();
	}
	if dep.eq_ignore_ascii_case("sqlite3") || dep.eq_ignore_ascii_case("sqlite") {
		return "SQLite3".to_string();
	}
	if dep.eq_ignore_ascii_case("png") || dep.eq_ignore_ascii_case("libpng") {
		return "PNG".to_string();
	}
	if dep.eq_ignore_ascii_case("jpeg") || dep.eq_ignore_ascii_case("libjpeg") {
		return "JPEG".to_string();
	}
	if dep.eq_ignore_ascii_case("libxml2") || dep.eq_ignore_ascii_case("xml2") {
		return "LibXml2".to_string();
	}
	if dep.eq_ignore_ascii_case("gtest") || dep.eq_ignore_ascii_case("googletest") {
		return "GTest".to_string();
	}
	if dep.eq_ignore_ascii_case("catch2") {
		return "Catch2".to_string();
	}
	if dep.eq_ignore_ascii_case("doctest") {
		return "doctest".to_string();
	}
	if dep.eq_ignore_ascii_case("yaml-cpp") || dep.eq_ignore_ascii_case("yaml") {
		return "yaml-cpp".to_string();
	}
	if dep.eq_ignore_ascii_case("libarchive") || dep.eq_ignore_ascii_case("archive") {
		return "LibArchive".to_string();
	}
	if dep.eq_ignore_ascii_case("libssh2") || dep.eq_ignore_ascii_case("ssh2") {
		return "Libssh2".to_string();
	}
	if dep.eq_ignore_ascii_case("bzip2") || dep.eq_ignore_ascii_case("bz2") {
		return "BZip2".to_string();
	}
	if dep.eq_ignore_ascii_case("libpq") || dep.eq_ignore_ascii_case("postgres") {
		return "PostgreSQL".to_string();
	}
	if dep.chars().any(|character| character.is_ascii_uppercase()) {
		return dep.to_string();
	}
	let normalized = if dep.len() > 3 && dep.to_ascii_lowercase().starts_with("lib") {
		&dep[3..]
	} else {
		dep
	};
	normalized.to_ascii_lowercase()
}

fn cmake_link_names_equal(left: &str, right: &str) -> bool {
	let normalize = |value: &str| {
		value
			.split_once("::")
			.map(|(package, component)| {
				format!("{}::{}", canonical_cmake_package_name(package), component)
			})
			.unwrap_or_else(|| canonical_cmake_package_name(value))
	};
	normalize(left) == normalize(right)
}

fn cmake_target_name(line: &str, command: &str) -> Option<String> {
	cmake_parser::find_calls(line, command)
		.into_iter()
		.next()
		.and_then(|call| call.args.into_iter().next())
}

fn is_cmake_target_line(line: &str, command: &str, target_name: &str) -> bool {
	cmake_target_name(line, command).is_some_and(|name| name == target_name)
}

const FIND_PACKAGE_KEYWORDS: &[&str] = &[
	"COMPONENTS",
	"OPTIONAL_COMPONENTS",
	"CONFIG",
	"CONFIGS",
	"EXACT",
	"MODULE",
	"NAMES",
	"NO_MODULE",
	"PATHS",
	"HINTS",
	"PATH_SUFFIXES",
	"QUIET",
	"REQUIRED",
	"REGISTRY_VIEW",
	"NO_DEFAULT_PATH",
	"NO_PACKAGE_ROOT_PATH",
	"NO_CMAKE_PATH",
	"NO_CMAKE_ENVIRONMENT_PATH",
	"NO_SYSTEM_ENVIRONMENT_PATH",
	"NO_CMAKE_SYSTEM_PATH",
	"NO_CMAKE_SYSTEM_PACKAGE_REGISTRY",
	"CMAKE_FIND_ROOT_PATH_BOTH",
	"ONLY_CMAKE_FIND_ROOT_PATH",
	"NO_CMAKE_FIND_ROOT_PATH",
	"CMAKE_FIND_USE_PACKAGE_ROOT_PATH",
	"CMAKE_FIND_USE_CMAKE_SYSTEM_PATH",
	"CMAKE_FIND_USE_SYSTEM_ENVIRONMENT_PATH",
	"CMAKE_FIND_USE_CMAKE_ENVIRONMENT_PATH",
	"CMAKE_FIND_DEBUG_MODE",
	"BYPASS_PROVIDER",
];

fn find_component_clause(arguments: &[String], keyword: &str) -> Option<(usize, usize)> {
	let keyword_index = arguments
		.iter()
		.position(|argument| argument.eq_ignore_ascii_case(keyword))?;
	let start = keyword_index + 1;
	let end = arguments[start..]
		.iter()
		.position(|argument| {
			FIND_PACKAGE_KEYWORDS
				.iter()
				.any(|candidate| argument.eq_ignore_ascii_case(candidate))
		})
		.map_or(arguments.len(), |offset| start + offset);
	Some((start, end))
}

fn replace_component_clause(
	arguments: &[String],
	keyword: &str,
	components: &[String],
) -> Vec<String> {
	let mut result = arguments.to_vec();
	if let Some((start, end)) = find_component_clause(arguments, keyword) {
		if components.is_empty() {
			result.drain(start - 1..end);
		} else {
			result.splice(
				start - 1..end,
				std::iter::once(keyword.to_string()).chain(components.iter().cloned()),
			);
		}
		return result;
	}

	let insertion_index = arguments
		.iter()
		.position(|argument| {
			FIND_PACKAGE_KEYWORDS
				.iter()
				.any(|candidate| argument.eq_ignore_ascii_case(candidate))
		})
		.unwrap_or(arguments.len());
	result.splice(
		insertion_index..insertion_index,
		std::iter::once(keyword.to_string()).chain(components.iter().cloned()),
	);
	result
}

fn validate_cmake_identifier(value: &str, kind: &str) -> Result<()> {
	if value.is_empty() || value.len() > 100 {
		bail!("Invalid CMake {} name: {}", kind, value);
	}
	if !value.chars().all(|character| {
		character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | '+')
	}) {
		bail!("Invalid CMake {} name: {}", kind, value);
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_format_package_name() {
		assert_eq!(format_cmake_package_name("libcurl"), "CURL");
		assert_eq!(format_cmake_package_name("openssl"), "OPENSSL");
	}

	#[test]
	fn test_versioned_dependency_add_and_idempotency() {
		let cmake = "cmake_minimum_required(VERSION 3.16)\nproject(demo CXX)\n\nadd_executable(demo main.cpp)\n";
		let mut manager = CMakeDependencyManager::parse(cmake);
		let added = manager
			.add_dependencies(&["fmt@10.2.1".to_string()])
			.unwrap();
		assert_eq!(added, vec!["fmt@10.2.1".to_string()]);
		assert!(
			manager
				.content
				.contains("find_package(fmt 10.2.1 REQUIRED)")
		);
		assert!(manager.content.contains("fmt::fmt"));

		// Second identical add changes nothing.
		let mut again = CMakeDependencyManager::parse(&manager.content);
		assert!(
			again
				.add_dependencies(&["fmt@10.2.1".to_string()])
				.unwrap()
				.is_empty()
		);
		assert_eq!(again.content, manager.content);

		// A different version updates the existing call instead of duplicating.
		let mut updated = CMakeDependencyManager::parse(&manager.content);
		updated
			.add_dependencies(&["fmt@11.0.0".to_string()])
			.unwrap();
		assert!(
			updated
				.content
				.contains("find_package(fmt 11.0.0 REQUIRED)")
		);
		assert_eq!(updated.content.matches("find_package(fmt").count(), 1);
	}

	#[test]
	fn test_versioned_dependency_merges_into_existing_find_package() {
		let cmake = "cmake_minimum_required(VERSION 3.16)\nproject(demo CXX)\n\nfind_package(fmt REQUIRED)\n\nadd_executable(demo main.cpp)\n";
		let mut manager = CMakeDependencyManager::parse(cmake);
		manager
			.add_dependencies(&["fmt@10.2.1".to_string()])
			.unwrap();
		assert!(
			manager
				.content
				.contains("find_package(fmt 10.2.1 REQUIRED)")
		);
	}
}
