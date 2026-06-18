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
			let mut changed = self.ensure_find_package(&parsed.package, &parsed.components)?;
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

	fn ensure_find_package(&mut self, package: &str, components: &[String]) -> Result<bool> {
		let Some(line_index) = self.find_package_line(package) else {
			if self.project_line().is_none() {
				bail!("CMakeLists.txt does not contain a project() declaration");
			}
			let command = if components.is_empty() {
				format!("find_package({} REQUIRED)\n", package)
			} else {
				format!(
					"find_package({} REQUIRED COMPONENTS {})\n",
					package,
					components.join(" ")
				)
			};
			self.insert_after_project(&command);
			return Ok(true);
		};

		if components.is_empty() {
			return Ok(false);
		}

		let line = self
			.content
			.lines()
			.nth(line_index)
			.unwrap_or_default()
			.to_string();
		let arguments = cmake_call_arguments(&line, "find_package")?;
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
		let replacement = format!("find_package({})", replacement_arguments.join(" "));
		self.replace_line(line_index, &replacement);
		Ok(true)
	}

	fn remove_find_package(&mut self, package: &str, components: &[String]) -> Result<bool> {
		let Some(line_index) = self.find_package_line(package) else {
			return Ok(false);
		};
		if components.is_empty() {
			self.remove_line(line_index);
			return Ok(true);
		}

		let line = self
			.content
			.lines()
			.nth(line_index)
			.unwrap_or_default()
			.to_string();
		let arguments = cmake_call_arguments(&line, "find_package")?;
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
		let replacement = format!("find_package({})", replacement_arguments.join(" "));
		self.replace_line(line_index, &replacement);
		Ok(true)
	}

	fn add_to_target_link_libraries(&mut self, link_name: &str) -> Result<bool> {
		if let Some(line_index) = self.target_link_line() {
			let line = self
				.content
				.lines()
				.nth(line_index)
				.unwrap_or_default()
				.to_string();
			let mut arguments = cmake_call_arguments(&line, "target_link_libraries")?;
			if arguments
				.iter()
				.skip(1)
				.any(|value| cmake_link_names_equal(value, link_name))
			{
				return Ok(false);
			}
			arguments.push(link_name.to_string());
			let replacement = format!("target_link_libraries({})", arguments.join(" "));
			self.replace_line(line_index, &replacement);
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
		let mut line_indices = self
			.content
			.lines()
			.enumerate()
			.filter_map(|(index, line)| {
				(!line.trim_start().starts_with('#') && line.contains("target_link_libraries("))
					.then_some(index)
			})
			.collect::<Vec<_>>();
		for line_index in line_indices.drain(..).rev() {
			let line = self
				.content
				.lines()
				.nth(line_index)
				.unwrap_or_default()
				.to_string();
			let arguments = cmake_call_arguments(&line, "target_link_libraries")?;
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
				self.replace_line(
					line_index,
					&format!("target_link_libraries({})", filtered.join(" ")),
				);
			} else {
				self.remove_line(line_index);
			}
		}
		Ok(())
	}

	fn find_package_line(&self, package: &str) -> Option<usize> {
		self.content.lines().enumerate().find_map(|(index, line)| {
			let arguments = cmake_call_arguments(line, "find_package").ok()?;
			let found = arguments
				.first()
				.is_some_and(|name| canonical_cmake_package_name(name) == *package);
			found.then_some(index)
		})
	}

	fn target_link_line(&self) -> Option<usize> {
		let primary_target = self.first_target_name()?;
		self.content.lines().enumerate().find_map(|(index, line)| {
			let arguments = cmake_call_arguments(line, "target_link_libraries").ok()?;
			arguments
				.first()
				.is_some_and(|target| target == &primary_target)
				.then_some(index)
		})
	}

	fn first_target_name(&self) -> Option<String> {
		let mut fallback = None;
		for line in self.content.lines() {
			for command in ["add_executable", "add_library"] {
				let Some(name) = cmake_target_name(line, command) else {
					continue;
				};
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
		self.content
			.lines()
			.enumerate()
			.find_map(|(index, line)| line.contains("project(").then_some(index))
	}

	fn insert_after_project(&mut self, text: &str) {
		self.insert_at(
			self.project_line().map(|index| index + 1).unwrap_or(0),
			text,
		);
	}

	fn insert_after_target(&mut self, target_name: &str, text: &str) {
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
				self.content.lines().enumerate().find_map(|(index, line)| {
					(line.contains("add_executable(") || line.contains("add_library("))
						.then_some(index + 1)
				})
			})
			.unwrap_or_else(|| self.content.lines().count());
		self.insert_at(index, text);
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
	components: Vec<String>,
}

impl DependencySpec {
	fn parse(value: &str) -> Result<Self> {
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
		vec![self.package.clone()]
	}
}

fn cmake_call_arguments(line: &str, command: &str) -> Result<Vec<String>> {
	if line.trim_start().starts_with('#') {
		bail!("CMake command is commented out");
	}
	let start = line
		.find(&format!("{}(", command))
		.ok_or_else(|| anyhow::anyhow!("Missing {} call", command))?
		+ command.len()
		+ 1;
	let end = line[start..]
		.find(')')
		.map(|offset| start + offset)
		.ok_or_else(|| anyhow::anyhow!("Unterminated {} call", command))?;
	Ok(line[start..end]
		.split_whitespace()
		.map(|value| value.trim_matches('"').to_string())
		.collect())
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
	let start = line.find(&format!("{}(", command))? + command.len() + 1;
	let end = line[start..].find(')')? + start;
	line[start..end]
		.split_whitespace()
		.next()
		.map(|name| name.trim_matches('"').to_string())
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
}
