use crate::file_handler::write_atomic;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetType {
	Executable,
	StaticLibrary,
	SharedLibrary,
}

impl FromStr for TargetType {
	type Err = anyhow::Error;

	fn from_str(value: &str) -> Result<Self> {
		match value.to_lowercase().as_str() {
			"exe" | "executable" | "app" => Ok(Self::Executable),
			"static" | "static-lib" | "a" => Ok(Self::StaticLibrary),
			"shared" | "shared-lib" | "so" | "dll" | "dylib" => Ok(Self::SharedLibrary),
			_ => bail!(
				"Invalid target type: {}. Valid options: exe, static, shared",
				value
			),
		}
	}
}

impl TargetType {
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Executable => "executable",
			Self::StaticLibrary => "static library",
			Self::SharedLibrary => "shared library",
		}
	}

	pub fn cmake_target_type(&self) -> &'static str {
		match self {
			Self::Executable => "add_executable",
			Self::StaticLibrary | Self::SharedLibrary => "add_library",
		}
	}

	pub fn file_extension(&self) -> &'static str {
		match self {
			Self::Executable => "",
			Self::StaticLibrary => ".a",
			Self::SharedLibrary => ".so",
		}
	}
}

#[derive(Debug, Clone)]
pub struct BuildTarget {
	pub name: String,
	pub target_type: TargetType,
	pub sources: Vec<String>,
	pub dependencies: Vec<String>,
}

impl BuildTarget {
	pub fn new(name: String, target_type: TargetType) -> Self {
		Self {
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

	pub fn add_dependency(&mut self, dependency: String) {
		if !self.dependencies.contains(&dependency) {
			self.dependencies.push(dependency);
		}
	}
}

pub struct MultiTargetManager {
	pub targets: Vec<BuildTarget>,
}

impl MultiTargetManager {
	pub fn new() -> Self {
		Self {
			targets: Vec::new(),
		}
	}

	pub fn parse_from_cmake(cmake_path: &Path) -> Result<Self> {
		let content = fs::read_to_string(cmake_path).context("Failed to read CMakeLists.txt")?;
		let mut manager = Self::new();

		for line in content.lines() {
			if let Some(arguments) = cmake_command_arguments(line, "add_executable") {
				let Some(name) = arguments.first().copied() else {
					continue;
				};
				if !is_manageable_cmake_target(name) || arguments.contains(&"ALIAS") {
					continue;
				}
				let mut target = BuildTarget::new(name.to_string(), TargetType::Executable);
				for source in arguments
					.iter()
					.skip(1)
					.copied()
					.filter(|argument| is_cmake_source_token(argument))
				{
					target.add_source(source.to_string());
				}
				manager.add_target(target);
				continue;
			}

			if let Some(arguments) = cmake_command_arguments(line, "add_library") {
				let Some(name) = arguments.first().copied() else {
					continue;
				};
				if !is_manageable_cmake_target(name)
					|| arguments
						.iter()
						.any(|argument| matches!(*argument, "ALIAS" | "IMPORTED"))
				{
					continue;
				}
				let target_type = if arguments
					.iter()
					.any(|argument| matches!(*argument, "SHARED" | "MODULE"))
				{
					TargetType::SharedLibrary
				} else {
					TargetType::StaticLibrary
				};
				let mut target = BuildTarget::new(name.to_string(), target_type);
				for source in arguments
					.iter()
					.skip(1)
					.copied()
					.filter(|argument| is_cmake_source_token(argument))
				{
					target.add_source(source.to_string());
				}
				manager.add_target(target);
			}
		}

		for line in content.lines() {
			for command in ["target_link_libraries", "add_dependencies"] {
				let Some(arguments) = cmake_command_arguments(line, command) else {
					continue;
				};
				let Some(name) = arguments.first().copied() else {
					continue;
				};
				let Some(target) = manager
					.targets
					.iter_mut()
					.find(|target| target.name == name)
				else {
					continue;
				};
				for dependency in arguments.iter().skip(1).copied() {
					if command == "target_link_libraries" && is_cmake_link_keyword(dependency) {
						continue;
					}
					target.add_dependency(dependency.to_string());
				}
			}
		}
		Ok(manager)
	}

	pub fn parse_from_makefile(makefile_path: &Path) -> Result<Self> {
		let content = fs::read_to_string(makefile_path).context("Failed to read Makefile")?;
		let mut rules = Vec::new();

		for line in content.lines() {
			let Some((target_names, prerequisites)) = split_make_rule(line) else {
				continue;
			};
			let Some(rule_name) = target_names.split_whitespace().next() else {
				continue;
			};
			let resolved_rule_name = resolve_make_references(rule_name, &content, 0);
			let rule_name = resolved_rule_name.as_str();
			if !is_parseable_make_target_name(rule_name)
				|| is_make_meta_target(rule_name)
				|| is_make_object_token(rule_name)
				|| is_make_utility_target(rule_name)
			{
				continue;
			}
			let (target_name, target_type) = canonical_make_target(rule_name);
			rules.push(ParsedMakeRule {
				rule_name: rule_name.to_string(),
				target_name,
				target_type,
				prerequisites: prerequisites
					.split_whitespace()
					.map(str::to_string)
					.collect(),
				link_flags: Vec::new(),
			});
		}

		let mut current_rule: Option<usize> = None;
		for line in content.lines() {
			if is_make_recipe_line(line) {
				if let Some(index) = current_rule {
					for token in line.split_whitespace() {
						if let Some(flag) = make_linker_flag(token) {
							rules[index].link_flags.push(flag);
						}
					}
				}
				continue;
			}
			if let Some((target_names, _)) = split_make_rule(line) {
				current_rule = target_names
					.split_whitespace()
					.next()
					.and_then(|rule_name| {
						let resolved = resolve_make_references(rule_name, &content, 0);
						rules.iter().position(|rule| rule.rule_name == resolved)
					});
			} else if !line.trim().is_empty() && !line.trim().starts_with('#') {
				current_rule = None;
			}
		}

		let mut manager = Self::new();
		for rule in &rules {
			let mut target = BuildTarget::new(rule.target_name.clone(), rule.target_type);
			for prerequisite in &rule.prerequisites {
				if is_ignored_make_prerequisite(prerequisite) {
					continue;
				}
				if let Some(local_name) = find_make_local_target(prerequisite, &rules) {
					if local_name != target.name {
						target.add_dependency(local_name.to_string());
					}
				} else if is_make_source_token(prerequisite) && !is_make_library_token(prerequisite)
				{
					target.add_source(prerequisite.to_string());
				} else {
					validate_dependency_name(prerequisite)?;
					let dependency = if is_imported_target_dependency(prerequisite)
						|| is_explicit_linker_dependency(prerequisite)
					{
						prerequisite.to_string()
					} else {
						make_bare_linker_flag(prerequisite)
					};
					target.add_dependency(dependency);
				}
			}
			for flag in &rule.link_flags {
				validate_dependency_name(flag)?;
				target.add_dependency(flag.clone());
			}
			manager.add_target(target);
		}
		Ok(manager)
	}

	pub fn add_target(&mut self, target: BuildTarget) {
		if self.get_target(&target.name).is_none() {
			self.targets.push(target);
		}
	}

	pub fn remove_target(&mut self, name: &str) -> Result<()> {
		let Some(index) = self
			.targets
			.iter()
			.position(|target| target.name == name || target_rule_name(target) == name)
		else {
			bail!("Target '{}' not found", name);
		};
		let identifiers = [
			self.targets[index].name.clone(),
			target_rule_name(&self.targets[index]),
		];
		self.targets.remove(index);
		for target in &mut self.targets {
			target.dependencies.retain(|dependency| {
				!identifiers
					.iter()
					.any(|identifier| dependency == identifier)
			});
		}
		Ok(())
	}

	pub fn get_target(&self, name: &str) -> Option<&BuildTarget> {
		self.targets
			.iter()
			.find(|target| target.name == name || target_rule_name(target) == name)
	}

	pub fn remove_from_cmake(&self, cmake_path: &Path, name: &str) -> Result<()> {
		let content = fs::read_to_string(cmake_path).context("Failed to read CMakeLists.txt")?;
		let names = cmake_target_name_candidates(name);
		let direct_commands = [
			"add_executable",
			"add_library",
			"target_link_libraries",
			"target_compile_definitions",
			"target_include_directories",
			"target_compile_features",
			"target_compile_options",
			"target_link_options",
			"target_sources",
			"add_dependencies",
		];
		let mut output = Vec::new();
		for line in content.lines() {
			let mut remove_line = false;
			let mut replacement = None;
			for command in direct_commands {
				let Some(arguments) = cmake_command_arguments(line, command) else {
					continue;
				};
				let Some(first_argument) = arguments.first().copied() else {
					continue;
				};
				if names.iter().any(|candidate| candidate == first_argument) {
					remove_line = true;
					break;
				}
				if !matches!(
					command,
					"target_link_libraries" | "add_dependencies" | "target_sources"
				) || !arguments
					.iter()
					.skip(1)
					.any(|argument| names.iter().any(|candidate| candidate == argument))
				{
					continue;
				}
				let filtered = arguments
					.iter()
					.copied()
					.filter(|argument| !names.iter().any(|candidate| candidate == argument))
					.collect::<Vec<_>>();
				let has_remaining_dependency = if command == "target_link_libraries" {
					filtered
						.iter()
						.skip(1)
						.any(|argument| !is_cmake_link_keyword(argument))
				} else {
					filtered.len() > 1
				};
				if !has_remaining_dependency {
					remove_line = true;
				} else {
					replacement = Some(format!("{}({})", command, filtered.join(" ")));
				}
			}
			if remove_line {
				continue;
			}
			if let Some(replacement) = replacement {
				output.push(replacement);
			} else {
				output.push(line.to_string());
			}
		}
		write_atomic(cmake_path, &format!("{}\n", output.join("\n")))
			.context("Failed to write CMakeLists.txt")
	}

	pub fn remove_from_makefile(&self, makefile_path: &Path, name: &str) -> Result<()> {
		let content = fs::read_to_string(makefile_path).context("Failed to read Makefile")?;
		let identifiers = make_target_identifiers(name);
		let (canonical_name, _) = canonical_make_target(name);
		let object_prefix = format!("$(BUILD_DIR)/{canonical_name}_");
		let mut lines = content.lines().peekable();
		let mut output = Vec::new();
		while let Some(line) = lines.next() {
			let trimmed = line.trim_start();
			let rule = split_make_rule(line);
			if let Some((target_names, prerequisites)) = rule {
				let target_names = target_names.split_whitespace().collect::<Vec<_>>();
				let resolved_target_names = target_names
					.iter()
					.map(|target_name| resolve_make_references(target_name, &content, 0))
					.collect::<Vec<_>>();
				let removes_target = target_names.iter().zip(resolved_target_names.iter()).any(
					|(target_name, resolved_name)| {
						identifiers.iter().any(|identifier| {
							*target_name == identifier
								|| resolved_name == identifier
								|| Path::new(resolved_name)
									.file_name()
									.and_then(|value| value.to_str())
									.is_some_and(|value| value == identifier)
						}) || target_name.starts_with(&object_prefix)
					},
				);
				if removes_target {
					while lines.peek().is_some_and(|next| is_make_recipe_line(next)) {
						lines.next();
					}
					continue;
				}

				let filtered = prerequisites
					.split_whitespace()
					.filter(|dependency| {
						let resolved = resolve_make_references(dependency, &content, 0);
						!identifiers.iter().any(|identifier| {
							*dependency == identifier
								|| resolved == *identifier
								|| Path::new(&resolved)
									.file_name()
									.and_then(|value| value.to_str())
									.is_some_and(|value| value == identifier)
						})
					})
					.map(str::to_string)
					.collect::<Vec<_>>();
				let filtered = if target_names.contains(&"all") {
					deduplicate_strings(filtered)
				} else {
					filtered
				};
				let prerequisites_changed =
					filtered.len() != prerequisites.split_whitespace().count();
				if prerequisites_changed
					&& filtered.is_empty()
					&& target_names
						.iter()
						.any(|target_name| matches!(*target_name, "run" | "test"))
				{
					while lines.peek().is_some_and(|next| is_make_recipe_line(next)) {
						lines.next();
					}
					continue;
				}
				if prerequisites_changed || target_names.contains(&"all") {
					let suffix = if filtered.is_empty() {
						String::new()
					} else {
						format!(" {}", filtered.join(" "))
					};
					let indent = &line[..line.len() - trimmed.len()];
					output.push(format!("{indent}{}:{suffix}", target_names.join(" ")));
				} else {
					output.push(line.to_string());
				}
			} else {
				output.push(line.to_string());
			}
		}
		write_atomic(makefile_path, &format!("{}\n", output.join("\n")))
			.context("Failed to write Makefile")
	}

	pub fn add_to_cmake(&self, cmake_path: &Path) -> Result<()> {
		let mut content =
			fs::read_to_string(cmake_path).context("Failed to read CMakeLists.txt")?;
		for target in &self.targets {
			validate_target_name(&target.name)?;
			for dependency in &target.dependencies {
				validate_dependency_name(dependency)?;
			}
			let command = target.target_type.cmake_target_type();
			let already_present = target.name.starts_with('$')
				|| content.lines().any(|line| {
					let trimmed = line.trim_start();
					trimmed.starts_with(&format!("add_executable({}", target.name))
						|| trimmed.starts_with(&format!("add_library({}", target.name))
				});
			if !already_present {
				validate_new_target(target)?;
			}
			let sources = target.sources.join(" ");
			let dependencies = deduplicate_strings(target.dependencies.clone());
			let definition = if target.target_type == TargetType::SharedLibrary {
				format!("{}({} SHARED {})\n", command, target.name, sources)
			} else {
				format!("{}({} {})\n", command, target.name, sources)
			};
			if !already_present {
				append_generated_line(&mut content, &definition);
			}
			if !dependencies.is_empty()
				&& !is_cmake_target_line(&content, "target_link_libraries", &target.name)
			{
				append_generated_line(
					&mut content,
					&format!(
						"target_link_libraries({} {})\n",
						target.name,
						dependencies.join(" ")
					),
				);
			}
		}
		write_atomic(cmake_path, &content).context("Failed to write CMakeLists.txt")
	}

	pub fn add_to_makefile(&self, makefile_path: &Path) -> Result<()> {
		let mut content = fs::read_to_string(makefile_path).context("Failed to read Makefile")?;
		let mut added = Vec::new();
		for target in &self.targets {
			validate_target_name(&target.name)?;
			for dependency in &target.dependencies {
				validate_dependency_name(dependency)?;
			}
			let rule_identifier = target_rule_name(target);
			if makefile_has_target(&content, &rule_identifier) {
				continue;
			}
			validate_new_target(target)?;
			let (local_dependencies, external_linker_inputs) =
				makefile_dependencies(target, &self.targets);
			let mut prerequisites = target.sources.clone();
			for dependency in &local_dependencies {
				push_unique(&mut prerequisites, dependency.clone());
			}
			let prerequisites = prerequisites.join(" ");
			let mut link_inputs = target.sources.clone();
			for dependency in local_dependencies
				.iter()
				.chain(external_linker_inputs.iter())
			{
				push_unique(&mut link_inputs, dependency.clone());
			}
			let link_inputs = link_inputs.join(" ");
			let link_inputs = if link_inputs.is_empty() {
				String::new()
			} else {
				format!(" {link_inputs}")
			};
			let cpp_target = content.contains("CXXFLAGS")
				|| target.sources.iter().any(|source| {
					Path::new(source)
						.extension()
						.and_then(|value| value.to_str())
						.is_some_and(|value| {
							matches!(
								value.to_ascii_lowercase().as_str(),
								"cpp" | "cc" | "cxx" | "c++"
							) || value == "C"
						})
				});
			let compiler = if cpp_target { "$(CXX)" } else { "$(CC)" };
			let flags = if cpp_target {
				"$(CXXFLAGS)"
			} else {
				"$(CFLAGS)"
			};
			let definition = match target.target_type {
				TargetType::Executable => format!(
					"{}: {}\n\t{} {} $(CPPFLAGS) -o {}{} $(LDFLAGS) $(LDLIBS)\n",
					target.name, prerequisites, compiler, flags, target.name, link_inputs
				),
				TargetType::StaticLibrary => {
					make_static_library_definition(target, compiler, flags, &self.targets)
				}
				TargetType::SharedLibrary => format!(
					"lib{}.so: {}\n\t{} {} $(CPPFLAGS) -shared -fPIC -o lib{}.so{} $(LDFLAGS) $(LDLIBS)\n",
					target.name, prerequisites, compiler, flags, target.name, link_inputs
				),
			};
			append_generated_line(&mut content, &definition);

			added.push(target.clone());
		}
		ensure_all_dependencies(&mut content, &added);
		write_atomic(makefile_path, &content).context("Failed to write Makefile")
	}
}

impl Default for MultiTargetManager {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug)]
struct ParsedMakeRule {
	rule_name: String,
	target_name: String,
	target_type: TargetType,
	prerequisites: Vec<String>,
	link_flags: Vec<String>,
}

fn cmake_command_arguments<'a>(line: &'a str, command: &str) -> Option<Vec<&'a str>> {
	let trimmed = line.trim_start();
	let after_command = trimmed.strip_prefix(command)?;
	let after_command = after_command.trim_start();
	let arguments = after_command.strip_prefix('(')?;
	let end = arguments.find(')').unwrap_or(arguments.len());
	let mut result = Vec::new();
	for argument in arguments[..end].split_whitespace() {
		if argument.starts_with('#') {
			break;
		}
		result.push(argument);
	}
	Some(result)
}

fn is_manageable_cmake_target(name: &str) -> bool {
	!name.is_empty() && !name.contains("::")
}

fn is_cmake_source_token(value: &str) -> bool {
	!value.starts_with('$')
		&& !matches!(
			value,
			"STATIC"
				| "SHARED" | "MODULE"
				| "INTERFACE"
				| "ALIAS" | "IMPORTED"
				| "WIN32" | "MACOSX_BUNDLE"
				| "EXCLUDE_FROM_ALL"
		)
}

fn is_cmake_link_keyword(value: &str) -> bool {
	matches!(
		value,
		"PUBLIC" | "PRIVATE" | "INTERFACE" | "debug" | "optimized" | "general"
	)
}

fn resolve_make_references(value: &str, content: &str, depth: usize) -> String {
	if depth > 8 || !value.contains("$(") {
		return value.to_string();
	}
	let mut result = value.to_string();
	let mut cursor = 0;
	while let Some(relative_start) = result[cursor..].find("$(") {
		let start = cursor + relative_start;
		let Some(relative_end) = result[start + 2..].find(')') else {
			break;
		};
		let end = start + 2 + relative_end;
		let name = &result[start + 2..end];
		let replacement = make_variable_value(content, name)
			.map(|value| resolve_make_references(&value, content, depth + 1))
			.unwrap_or_else(|| result[start..=end].to_string());
		result.replace_range(start..=end, &replacement);
		cursor = start + replacement.len();
	}
	result
}

fn make_variable_value(content: &str, name: &str) -> Option<String> {
	for line in content.lines() {
		let trimmed = line.trim_start();
		let Some(after_name) = trimmed.strip_prefix(name) else {
			continue;
		};
		let after_name = after_name.trim_start();
		let Some(value) = after_name
			.strip_prefix(":=")
			.or_else(|| after_name.strip_prefix("::="))
			.or_else(|| after_name.strip_prefix("?="))
			.or_else(|| after_name.strip_prefix("+="))
			.or_else(|| after_name.strip_prefix('='))
		else {
			continue;
		};
		return Some(value.trim().to_string());
	}
	None
}

fn split_make_rule(line: &str) -> Option<(&str, &str)> {
	let trimmed = line.trim();
	if trimmed.is_empty()
		|| line.starts_with('\t')
		|| trimmed.starts_with('#')
		|| trimmed.starts_with('.')
	{
		return None;
	}
	let colon = trimmed.find(':')?;
	let target_names = trimmed[..colon].trim();
	let prerequisites = trimmed[colon + 1..].trim();
	let assignment = target_names.contains('=')
		|| prerequisites.starts_with('=')
		|| prerequisites.starts_with(':')
		|| prerequisites.starts_with('?')
		|| prerequisites.starts_with('+');
	if target_names.is_empty() || assignment {
		return None;
	}
	Some((target_names, prerequisites))
}

fn is_make_recipe_line(line: &str) -> bool {
	line.starts_with('\t')
}

fn is_make_utility_target(name: &str) -> bool {
	matches!(
		name,
		"all" | "clean" | "run" | "rebuild" | "test" | "lint" | "docs" | "install-deps"
	)
}

fn is_make_meta_target(name: &str) -> bool {
	matches!(
		name,
		"ifeq"
			| "ifneq" | "ifdef"
			| "ifndef"
			| "else" | "endif"
			| "define"
			| "endef" | "export"
			| "unexport"
			| "undefine"
			| "override"
			| "private"
			| "vpath"
	)
}

fn is_parseable_make_target_name(name: &str) -> bool {
	validate_dependency_name(name).is_ok()
}

fn canonical_make_target(name: &str) -> (String, TargetType) {
	let name = Path::new(name)
		.file_name()
		.and_then(|value| value.to_str())
		.unwrap_or(name);
	if let Some(name_without_prefix) = name.strip_prefix("lib") {
		if let Some(name_without_extension) = name_without_prefix.strip_suffix(".a")
			&& !name_without_extension.is_empty()
		{
			return (
				name_without_extension.to_string(),
				TargetType::StaticLibrary,
			);
		}
		if let Some(name_without_extension) = name_without_prefix.strip_suffix(".so")
			&& !name_without_extension.is_empty()
		{
			return (
				name_without_extension.to_string(),
				TargetType::SharedLibrary,
			);
		}
		if let Some(name_without_extension) = name_without_prefix.strip_suffix(".dylib")
			&& !name_without_extension.is_empty()
		{
			return (
				name_without_extension.to_string(),
				TargetType::SharedLibrary,
			);
		}
	}
	(name.to_string(), TargetType::Executable)
}

fn find_make_local_target<'a>(dependency: &str, rules: &'a [ParsedMakeRule]) -> Option<&'a str> {
	rules
		.iter()
		.find(|rule| rule.rule_name == dependency || rule.target_name == dependency)
		.map(|rule| rule.target_name.as_str())
}

fn is_ignored_make_prerequisite(value: &str) -> bool {
	value.contains('$') || value.contains('%')
}

fn is_make_source_token(value: &str) -> bool {
	Path::new(value)
		.extension()
		.and_then(|extension| extension.to_str())
		.is_some_and(|extension| {
			matches!(
				extension.to_ascii_lowercase().as_str(),
				"c" | "cc" | "cpp" | "cxx" | "c++" | "h" | "hh" | "hpp" | "o" | "obj"
			)
		})
}

fn is_make_object_token(value: &str) -> bool {
	value.to_ascii_lowercase().ends_with(".o") || value.to_ascii_lowercase().ends_with(".obj")
}

fn is_make_library_token(value: &str) -> bool {
	value.to_ascii_lowercase().ends_with(".a")
		|| value.to_ascii_lowercase().ends_with(".so")
		|| value.to_ascii_lowercase().ends_with(".dylib")
		|| value.to_ascii_lowercase().ends_with(".lib")
		|| value.to_ascii_lowercase().ends_with(".dll")
}

fn make_linker_flag(token: &str) -> Option<String> {
	if token.starts_with("-l") && token.len() > 2 {
		Some(token.to_string())
	} else {
		None
	}
}

fn makefile_dependencies(
	target: &BuildTarget,
	all_targets: &[BuildTarget],
) -> (Vec<String>, Vec<String>) {
	let mut prerequisites = Vec::new();
	let mut linker_inputs = Vec::new();
	for dependency in &target.dependencies {
		if let Some(local_target) = all_targets.iter().find(|candidate| {
			dependency == &candidate.name || dependency == &target_rule_name(candidate)
		}) {
			push_unique(&mut prerequisites, target_rule_name(local_target));
		} else if is_imported_target_dependency(dependency) {
			push_unique(
				&mut linker_inputs,
				make_imported_target_linker_flag(dependency),
			);
		} else if is_explicit_linker_dependency(dependency) {
			push_unique(&mut linker_inputs, dependency.clone());
		} else {
			push_unique(&mut linker_inputs, make_bare_linker_flag(dependency));
		}
	}
	(prerequisites, linker_inputs)
}

fn is_imported_target_dependency(dependency: &str) -> bool {
	dependency.contains("::")
}

fn is_explicit_linker_dependency(dependency: &str) -> bool {
	dependency.starts_with('-')
		|| is_make_library_token(dependency)
		|| dependency.contains('/')
		|| Path::new(dependency)
			.extension()
			.and_then(|extension| extension.to_str())
			.is_some_and(|extension| extension.eq_ignore_ascii_case("o"))
}

fn make_imported_target_linker_flag(dependency: &str) -> String {
	let component = dependency.rsplit("::").next().unwrap_or(dependency);
	match component.to_ascii_lowercase().as_str() {
		"threads" => "-lpthread".to_string(),
		"crypto" => "-lcrypto".to_string(),
		"ssl" => "-lssl".to_string(),
		"zlib" => "-lz".to_string(),
		_ => make_bare_linker_flag(component),
	}
}

fn make_bare_linker_flag(dependency: &str) -> String {
	let name = dependency
		.strip_prefix("lib")
		.filter(|name| !name.is_empty())
		.unwrap_or(dependency);
	format!("-l{name}")
}

fn push_unique(values: &mut Vec<String>, value: String) {
	if !values.contains(&value) {
		values.push(value);
	}
}

fn deduplicate_strings(values: Vec<String>) -> Vec<String> {
	let mut result = Vec::new();
	for value in values {
		push_unique(&mut result, value);
	}
	result
}

fn make_static_library_definition(
	target: &BuildTarget,
	compiler: &str,
	flags: &str,
	all_targets: &[BuildTarget],
) -> String {
	let mut objects = Vec::new();
	let mut output = String::new();
	for source in &target.sources {
		let object = static_object_name(target, source);
		objects.push(object.clone());
		output.push_str(&format!(
			"{}: {}\n\t@mkdir -p $(dir $@)\n\t{} {} $(CPPFLAGS) -c {} -o {}\n",
			object, source, compiler, flags, source, object
		));
	}
	let (local_dependencies, _) = makefile_dependencies(target, all_targets);
	let mut prerequisites = objects.clone();
	for dependency in local_dependencies {
		push_unique(&mut prerequisites, dependency);
	}
	output.push_str(&format!(
		"lib{}.a: {}\n\t@mkdir -p $(dir $@)\n\tar rcs $@ {}\n",
		target.name,
		prerequisites.join(" "),
		objects.join(" ")
	));
	output
}

fn static_object_name(target: &BuildTarget, source: &str) -> String {
	let source_name = source
		.chars()
		.map(|character| {
			if character.is_ascii_alphanumeric() || matches!(character, '/' | '_' | '-' | '.' | '+')
			{
				character
			} else {
				'_'
			}
		})
		.collect::<String>()
		.replace('/', "_");
	format!("$(BUILD_DIR)/{}_{source_name}.o", target.name)
}

fn validate_new_target(target: &BuildTarget) -> Result<()> {
	validate_target_name(&target.name)?;
	if target.sources.is_empty() {
		bail!("Target '{}' must define at least one source", target.name);
	}
	for source in &target.sources {
		validate_source_path(source)?;
	}
	for dependency in &target.dependencies {
		validate_dependency_name(dependency)?;
	}
	Ok(())
}

fn validate_target_name(name: &str) -> Result<()> {
	if (name.starts_with("${") && name.ends_with('}'))
		|| (name.starts_with("$(") && name.ends_with(')'))
	{
		let inner = name
			.trim_start_matches("${")
			.trim_end_matches('}')
			.trim_start_matches("$(")
			.trim_end_matches(')');
		if !inner.is_empty()
			&& inner.chars().all(|character| {
				character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
			}) {
			return Ok(());
		}
	}
	crate::validate_project_name(name)
}

fn validate_source_path(source: &str) -> Result<()> {
	let path = Path::new(source);
	if source.is_empty()
		|| source == "."
		|| path.is_absolute()
		|| source.contains('\\')
		|| source.chars().any(|character| {
			character.is_control()
				|| character.is_whitespace()
				|| matches!(
					character,
					';' | '$' | '`' | '"' | '\'' | '|' | '&' | '>' | '<' | '%' | ':'
				)
		}) {
		bail!("Source path must be a safe relative path: {}", source);
	}
	if path
		.components()
		.any(|component| matches!(component, std::path::Component::ParentDir))
	{
		bail!("Source path cannot leave the project: {}", source);
	}
	Ok(())
}

fn validate_dependency_name(dependency: &str) -> Result<()> {
	if dependency.is_empty()
		|| dependency.chars().any(|character| {
			character.is_control()
				|| character.is_whitespace()
				|| matches!(
					character,
					'&' | '|'
						| '>' | '<' | '(' | ')'
						| '\\' | ';' | '$' | '`'
						| '"' | '\'' | '#' | '!'
						| '?' | '*' | '[' | ']'
						| '{' | '}' | '~' | '%'
				)
		}) {
		bail!("Invalid target dependency: {}", dependency);
	}
	if !dependency.chars().all(|character| {
		character.is_ascii_alphanumeric()
			|| matches!(
				character,
				'-' | '_' | '.' | '+' | ':' | '/' | '@' | '=' | ','
			)
	}) {
		bail!("Invalid target dependency: {}", dependency);
	}

	let imported_target = dependency.contains("::");
	if imported_target {
		let parts = dependency.split("::").collect::<Vec<_>>();
		if parts.iter().any(|part| {
			part.is_empty()
				|| !part.chars().all(|character| {
					character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | '+')
				})
		}) {
			bail!("Invalid target dependency: {}", dependency);
		}
	} else if dependency.contains(':') {
		bail!("Invalid target dependency: {}", dependency);
	}

	if dependency.starts_with('-') {
		if dependency.len() == 1 {
			bail!("Invalid target dependency: {}", dependency);
		}
	} else if dependency.contains('=') || dependency.contains(',') {
		bail!("Invalid target dependency: {}", dependency);
	}
	Ok(())
}

fn target_rule_name(target: &BuildTarget) -> String {
	match target.target_type {
		TargetType::Executable => target.name.clone(),
		TargetType::StaticLibrary => format!("lib{}.a", target.name),
		TargetType::SharedLibrary => format!("lib{}.so", target.name),
	}
}

fn make_target_identifiers(name: &str) -> Vec<String> {
	let (canonical_name, _) = canonical_make_target(name);
	deduplicate_strings(vec![
		name.to_string(),
		canonical_name.clone(),
		format!("lib{canonical_name}.a"),
		format!("lib{canonical_name}.so"),
		format!("lib{canonical_name}.dylib"),
	])
}

fn cmake_target_name_candidates(name: &str) -> Vec<String> {
	let mut candidates = vec![name.to_string()];
	let (canonical_name, _) = canonical_make_target(name);
	if canonical_name != name {
		candidates.push(canonical_name);
	}
	candidates
}

fn is_cmake_target_line(line: &str, command: &str, name: &str) -> bool {
	let trimmed = line.trim_start();
	let prefix = format!("{command}({name}");
	let Some(remainder) = trimmed.strip_prefix(&prefix) else {
		return false;
	};
	remainder
		.as_bytes()
		.first()
		.is_none_or(|byte| byte.is_ascii_whitespace() || *byte == b')')
}

fn makefile_has_target(content: &str, name: &str) -> bool {
	content.lines().any(|line| {
		split_make_rule(line).is_some_and(|(target_names, _)| {
			target_names.split_whitespace().any(|target| {
				let resolved = resolve_make_references(target, content, 0);
				resolved == name
					|| Path::new(&resolved)
						.file_name()
						.and_then(|value| value.to_str())
						.is_some_and(|value| value == name)
			})
		})
	})
}

fn append_generated_line(content: &mut String, line: &str) {
	if !content.is_empty() && !content.ends_with('\n') {
		content.push('\n');
	}
	content.push_str(line);
}

fn ensure_all_dependencies(content: &mut String, targets: &[BuildTarget]) {
	let existing = content.lines().enumerate().find_map(|(index, line)| {
		split_make_rule(line).and_then(|(target_names, prerequisites)| {
			(target_names
				.split_whitespace()
				.any(|target| target == "all"))
			.then_some((index, prerequisites))
		})
	});
	let Some((index, prerequisites)) = existing else {
		let dependencies = targets.iter().map(target_rule_name).collect::<Vec<_>>();
		if dependencies.is_empty() {
			return;
		}
		content.insert_str(0, &format!("all: {}\n\n", dependencies.join(" ")));
		return;
	};
	let mut dependencies = prerequisites
		.split_whitespace()
		.map(str::to_string)
		.collect::<Vec<_>>();
	dependencies = deduplicate_strings(dependencies);
	for target in targets {
		push_unique(&mut dependencies, target_rule_name(target));
	}
	let mut lines = content.lines().map(str::to_string).collect::<Vec<_>>();
	lines[index] = if dependencies.is_empty() {
		"all:".to_string()
	} else {
		format!("all: {}", dependencies.join(" "))
	};
	*content = lines.join("\n");
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
}
