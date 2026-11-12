use crate::file_handler::write_atomic;
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

	pub fn parse(content: &str) -> Result<Self> {
		let recipe_prefix = detect_recipe_prefix(content);
		let mut rules = HashMap::new();
		let mut current_rule: Option<Rule> = None;
		for (line_number, line) in content.lines().enumerate() {
			if let Some((target, dependencies)) = parse_rule_line_with(line, recipe_prefix) {
				if let Some(rule) = current_rule.take() {
					rules.insert(rule.target.clone(), rule);
				}
				current_rule = Some(Rule {
					target,
					dependencies,
					commands: Vec::new(),
					line_number,
				});
			} else if let Some(stripped) = strip_recipe_prefix(line, recipe_prefix)
				&& let Some(rule) = current_rule.as_mut()
			{
				rule.commands.push(stripped.to_string());
			}
		}

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

		// Read dependencies from every logical command, not just the first
		// recipe line, so a multi-command or `\`-continued recipe still parses.
		let mut current_deps: BTreeSet<String> = BTreeSet::new();
		for (_, _, text) in logical_commands(&install_deps.commands) {
			current_deps.extend(extract_install_dependencies(&text));
		}

		let mut added = Vec::new();
		for dep in deps {
			if current_deps.insert(dep.clone()) {
				added.push(dep.clone());
			}
		}

		if !added.is_empty() {
			// Replace only the package-manager command; other recipe lines
			// (e.g. `echo installing dependencies`) must be preserved.
			if let Some((first, last, text)) = find_install_command(&install_deps.commands) {
				let updated = update_install_command(&text, &added, &[])?;
				install_deps
					.commands
					.splice(first..=last, std::iter::once(updated));
			} else if !install_deps.commands.is_empty() {
				bail!(
					"install-deps has commands but none invoke a recognized package manager; add the dependency manually"
				);
			} else if let Some(command) = crate::os_detect::install_command_for(&added) {
				install_deps.commands.push(command);
			} else {
				bail!(
					"No supported system package manager detected; cannot generate install-deps command. Install '{}' manually.",
					added.join(", ")
				);
			}
		}

		Ok(added)
	}

	/// Remove dependencies from the install-deps rule
	pub fn remove_dependencies(&mut self, deps: &[String]) -> Result<Vec<String>> {
		if deps.is_empty() {
			return Ok(Vec::new());
		}
		for dep in deps {
			validate_dependency_name(dep)?;
		}

		let install_deps = match self.rules.get_mut("install-deps") {
			Some(rule) => rule,
			None => bail!("No install-deps rule found in Makefile"),
		};

		let mut current_deps: BTreeSet<String> = BTreeSet::new();
		for (_, _, text) in logical_commands(&install_deps.commands) {
			current_deps.extend(extract_install_dependencies(&text));
		}
		let deps_to_remove: std::collections::HashSet<_> = deps.iter().cloned().collect();
		let before_count = current_deps.len();
		let removed: Vec<String> = deps
			.iter()
			.filter(|dep| current_deps.contains(*dep))
			.cloned()
			.collect();
		current_deps.retain(|dep| !deps_to_remove.contains(dep));
		let removed_count = before_count - current_deps.len();

		if removed_count == 0 {
			bail!("Dependencies not found in Makefile: {:?}", deps);
		}

		if current_deps.is_empty() {
			self.rules.remove("install-deps");
			remove_install_deps_from_all(self);
		} else {
			// Update only the package-manager command, preserving any other
			// recipe lines the user wrote.
			let (first, last, text) = find_install_command(&install_deps.commands)
				.context("install-deps rule has no package manager command")?;
			let updated = update_install_command(&text, &[], &removed)?;
			install_deps
				.commands
				.splice(first..=last, std::iter::once(updated));
		}

		Ok(removed)
	}

	pub fn to_makefile_string(&self) -> String {
		let original = Self::parse(&self.content).ok();
		let original_rules = original.as_ref().map(|m| &m.rules);
		let recipe_prefix = detect_recipe_prefix(&self.content);
		let lines: Vec<&str> = self.content.split_inclusive('\n').collect();
		let mut output = String::new();
		let mut written_targets = std::collections::HashSet::new();
		let mut index = 0;

		while index < lines.len() {
			let line = lines[index].trim_end_matches(['\r', '\n']);
			if let Some((target, _)) = parse_rule_line_with(line, recipe_prefix) {
				let end = rule_block_end(&lines, index);
				let current = self.rules.get(&target);
				let original_rule = original_rules.and_then(|rules| rules.get(&target));
				if let Some(rule) = current {
					written_targets.insert(target.clone());
					if let Some(original_rule) = original_rule
						&& rules_equal(original_rule, rule)
					{
						for original_line in &lines[index..end] {
							output.push_str(original_line);
						}
					} else {
						output.push_str(&rule.to_makefile_format_with_prefix(recipe_prefix));
					}
				}
				index = end;
			} else {
				output.push_str(lines[index]);
				index += 1;
			}
		}

		let mut new_targets: Vec<&String> = self
			.rules
			.keys()
			.filter(|target| !written_targets.contains(*target))
			.collect();
		new_targets.sort();
		for target in new_targets {
			if !output.is_empty() && !output.ends_with('\n') {
				output.push('\n');
			}
			if !output.is_empty() && !output.ends_with("\n\n") {
				output.push('\n');
			}
			output.push_str(&self.rules[target].to_makefile_format_with_prefix(recipe_prefix));
		}

		if self.rules.contains_key("install-deps")
			&& !output.lines().any(|line| {
				line.trim_start().starts_with(".PHONY:")
					&& line.split_whitespace().any(|value| value == "install-deps")
			}) {
			if !output.ends_with('\n') {
				output.push('\n');
			}
			output.push_str("\n.PHONY: install-deps\n");
		}
		output
	}

	pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
		write_atomic(path.as_ref(), &self.to_makefile_string()).context("Failed to write Makefile")
	}
}

#[allow(dead_code)]
fn parse_rule_line(line: &str) -> Option<(String, Vec<String>)> {
	parse_rule_line_with(line, '\t')
}

fn detect_recipe_prefix(content: &str) -> char {
	for line in content.lines() {
		let trimmed = line.trim();
		if trimmed.starts_with(".RECIPEPREFIX")
			&& let Some(eq) = trimmed.find('=')
		{
			let value = trimmed[eq + 1..].trim();
			if let Some(character) = value.chars().next() {
				return character;
			}
		}
	}
	'\t'
}

fn strip_recipe_prefix(line: &str, prefix: char) -> Option<&str> {
	if prefix == '\t' {
		return line.strip_prefix('\t');
	}
	line.strip_prefix(prefix)
}

fn is_recipe_line(line: &str, prefix: char) -> bool {
	strip_recipe_prefix(line, prefix).is_some()
}

fn parse_rule_line_with(line: &str, recipe_prefix: char) -> Option<(String, Vec<String>)> {
	if is_recipe_line(line, recipe_prefix) || line.trim().is_empty() {
		return None;
	}
	let trimmed = line.trim_start();
	if trimmed.starts_with('#') {
		return None;
	}
	let colon = find_rule_colon(line)?;
	if is_make_assignment(trimmed) {
		return None;
	}
	let target = line[..colon].trim();
	if target.is_empty() {
		return None;
	}
	let dependencies = strip_make_comment(&line[colon + 1..])
		.split_whitespace()
		.map(str::to_string)
		.collect();
	Some((target.to_string(), dependencies))
}

fn find_rule_colon(line: &str) -> Option<usize> {
	let bytes = line.as_bytes();
	let mut index = 0;
	while index < bytes.len() {
		match bytes[index] {
			b'\\' => {
				index = (index + 2).min(bytes.len());
			}
			b'$' if index + 1 < bytes.len()
				&& (bytes[index + 1] == b'(' || bytes[index + 1] == b'{') =>
			{
				index = skip_make_expansion(bytes, index);
			}
			b':' => return Some(index),
			_ => index += 1,
		}
	}
	None
}

fn skip_make_expansion(bytes: &[u8], start: usize) -> usize {
	let opening = bytes[start + 1];
	let closing = if opening == b'(' { b')' } else { b'}' };
	let mut depth = 1;
	let mut index = start + 2;
	while index < bytes.len() {
		if bytes[index] == b'\\' {
			index = (index + 2).min(bytes.len());
			continue;
		}
		if bytes[index] == opening {
			depth += 1;
		} else if bytes[index] == closing {
			depth -= 1;
			if depth == 0 {
				return index + 1;
			}
		}
		index += 1;
	}
	bytes.len()
}

fn is_make_assignment(line: &str) -> bool {
	let mut start = 0;
	if line.starts_with("export ") || line.starts_with("override ") {
		start = line.find(char::is_whitespace).map_or(line.len(), |i| i + 1);
	}
	let name_end = line[start..]
		.find(char::is_whitespace)
		.map_or(line.len(), |index| start + index);
	if name_end == start {
		return false;
	}
	let rest = line[name_end..].trim_start();
	rest.starts_with('=')
		|| rest.starts_with(":=")
		|| rest.starts_with("::=")
		|| rest.starts_with("?=")
		|| rest.starts_with("+=")
}

fn strip_make_comment(value: &str) -> &str {
	let bytes = value.as_bytes();
	for (index, byte) in bytes.iter().enumerate() {
		if *byte == b'#' && (index == 0 || bytes[index - 1].is_ascii_whitespace()) {
			return &value[..index];
		}
	}
	value
}

fn rule_block_end(lines: &[&str], start: usize) -> usize {
	// Re-detect from joined content for RECIPEPREFIX support.
	let joined = lines.join("\n");
	let recipe_prefix = detect_recipe_prefix(&joined);
	let mut index = start + 1;
	while index < lines.len() && is_recipe_line(lines[index], recipe_prefix) {
		index += 1;
	}
	index
}

fn rules_equal(left: &Rule, right: &Rule) -> bool {
	left.target == right.target
		&& left.dependencies == right.dependencies
		&& left.commands == right.commands
}

impl Rule {
	/// Renders the rule using the default TAB recipe prefix.
	pub fn to_makefile_format(&self) -> String {
		self.to_makefile_format_with_prefix('\t')
	}

	/// Renders the rule using `recipe_prefix` for recipe lines. Makefiles that
	/// set `.RECIPEPREFIX` must keep using their own prefix, otherwise every
	/// regenerated recipe becomes unparseable.
	pub fn to_makefile_format_with_prefix(&self, recipe_prefix: char) -> String {
		let mut output = format!("{}: {}\n", self.target, self.dependencies.join(" "));
		for cmd in &self.commands {
			output.push(recipe_prefix);
			output.push_str(cmd);
			output.push('\n');
		}
		output
	}
}

/// Groups recipe lines into *logical* commands, joining `\` continuations into
/// a single unit. Returns `(first_line, last_line, joined_text)` triples.
fn logical_commands(commands: &[String]) -> Vec<(usize, usize, String)> {
	let mut logical = Vec::new();
	let mut index = 0;
	while index < commands.len() {
		let first = index;
		let mut text = String::new();
		loop {
			let line = commands[index].trim_end();
			if !text.is_empty() {
				text.push(' ');
			}
			text.push_str(line.trim_end_matches('\\').trim_end());
			let continues = line.trim_end().ends_with('\\');
			index += 1;
			if !continues || index >= commands.len() {
				break;
			}
		}
		if !text.trim().is_empty() {
			logical.push((first, index - 1, text.trim().to_string()));
		}
	}
	logical
}

/// Verbs that introduce a package list in a system package manager command.
fn is_install_verb(token: &str) -> bool {
	matches!(token, "install" | "-S" | "-i" | "-iA" | "add")
}

/// Finds the logical command in `commands` that invokes a package manager.
fn find_install_command(commands: &[String]) -> Option<(usize, usize, String)> {
	logical_commands(commands)
		.into_iter()
		.find(|(_, _, text)| text.split_whitespace().any(is_install_verb))
}

fn update_install_command(
	command: &str,
	additions: &[String],
	removals: &[String],
) -> Result<String> {
	let tokens = command.split_whitespace().collect::<Vec<_>>();
	let Some(verb_index) = tokens.iter().position(|token| is_install_verb(token)) else {
		bail!(
			"Cannot safely update unrecognized install-deps command: {}",
			command
		);
	};
	let mut package_names = BTreeSet::new();
	for token in tokens.iter().skip(verb_index + 1) {
		if !token.starts_with('-') && validate_dependency_name(token).is_ok() {
			package_names.insert((*token).to_string());
		}
	}
	if package_names.is_empty() && !removals.is_empty() {
		bail!("Cannot safely update install-deps command: {}", command);
	}
	for removal in removals {
		package_names.remove(removal);
	}
	for addition in additions {
		package_names.insert(addition.clone());
	}
	let first_package_index = tokens
		.iter()
		.skip(verb_index + 1)
		.position(|token| !token.starts_with('-') && validate_dependency_name(token).is_ok())
		.map(|offset| verb_index + 1 + offset);
	let Some(first_package_index) = first_package_index else {
		let mut result = tokens.into_iter().map(str::to_string).collect::<Vec<_>>();
		result.extend(package_names);
		return Ok(result.join(" "));
	};
	let mut result = Vec::new();
	result.extend(
		tokens[..first_package_index]
			.iter()
			.map(|token| (*token).to_string()),
	);
	result.extend(package_names);
	result.extend(
		tokens[first_package_index + 1..]
			.iter()
			.filter(|token| token.starts_with('-') || validate_dependency_name(token).is_err())
			.map(|token| (*token).to_string()),
	);
	Ok(result.join(" "))
}

fn extract_install_dependencies(command: &str) -> BTreeSet<String> {
	let tokens: Vec<&str> = command.split_whitespace().collect();
	let Some(verb_index) = tokens.iter().position(|token| is_install_verb(token)) else {
		return BTreeSet::new();
	};
	tokens
		.into_iter()
		.skip(verb_index + 1)
		.filter(|token| !token.starts_with('-') && validate_dependency_name(token).is_ok())
		.map(str::to_string)
		.collect()
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

	if name.starts_with('-') || name == "." || name == ".." {
		bail!("Invalid dependency name: {}", name);
	}

	if !name
		.chars()
		.all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+'))
	{
		bail!(
			"Invalid dependency name: {}. Only ASCII alphanumeric characters, dashes, underscores, dots, and plus signs are allowed.",
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

	#[test]
	fn regenerated_recipes_keep_custom_recipe_prefix() {
		let content = ".RECIPEPREFIX = >\nall: main\n>echo hi\n";
		let mut makefile = Makefile::parse(content).unwrap();
		makefile.add_dependencies(&["libcurl".to_string()]).unwrap();
		let rendered = makefile.to_makefile_string();
		// Every recipe line must use `>`, never a tab.
		for line in rendered.lines() {
			if line.starts_with('>') || line.starts_with('\t') {
				assert!(
					line.starts_with('>'),
					"recipe line must use the file's prefix: {line:?}"
				);
			}
		}
		// The generated install command must use the file's prefix too
		// (the package manager varies by OS: apt on Debian, pacman on
		// Arch, brew on macOS — verbs differ, so match on the prefix).
		assert!(
			recipe_line_after(&rendered, "install-deps").is_some_and(|line| line.starts_with('>')),
			"expected an install-deps recipe line with '>' prefix:\n{rendered}"
		);
	}

	#[test]
	fn default_recipe_prefix_is_tab() {
		let mut makefile = Makefile::parse("all: main\n\techo hi\n").unwrap();
		makefile.add_dependencies(&["libcurl".to_string()]).unwrap();
		let rendered = makefile.to_makefile_string();
		assert!(
			recipe_line_after(&rendered, "install-deps").is_some_and(|line| line.starts_with('\t')),
			"expected an install-deps recipe line with tab prefix:\n{rendered}"
		);
	}

	/// First recipe line (tab/`>`/space indented) following `target:`.
	fn recipe_line_after<'a>(rendered: &'a str, target: &str) -> Option<&'a str> {
		let mut in_target = false;
		for line in rendered.lines() {
			if line.starts_with(&format!("{target}:")) {
				in_target = true;
				continue;
			}
			if in_target {
				if line.starts_with('\t') || line.starts_with('>') || line.starts_with(' ') {
					return Some(line);
				}
				if !line.trim().is_empty() {
					return None;
				}
			}
		}
		None
	}

	#[test]
	fn preserves_other_recipe_lines_in_install_deps() {
		// A multi-command recipe must keep its non-package-manager lines.
		let content = "all: install-deps\n\ninstall-deps:\n\techo installing deps\n\tsudo apt install -y libcurl\n";
		let mut makefile = Makefile::parse(content).unwrap();
		let added = makefile.add_dependencies(&["libssl".to_string()]).unwrap();
		assert_eq!(added, vec!["libssl".to_string()]);
		let rendered = makefile.to_makefile_string();
		assert!(
			rendered.contains("echo installing deps"),
			"unrelated recipe line must survive:\n{rendered}"
		);
		assert!(rendered.contains("libcurl") && rendered.contains("libssl"));
	}

	#[test]
	fn joins_backslash_continuations_in_install_deps() {
		// A wrapped command must be read and rewritten as one unit, otherwise
		// the `\` swallows the next package name.
		let content = "all: install-deps\n\ninstall-deps:\n\tsudo apt install -y \\\n\t\tlibcurl\n";
		let mut makefile = Makefile::parse(content).unwrap();
		makefile.add_dependencies(&["libssl".to_string()]).unwrap();
		let rendered = makefile.to_makefile_string();
		assert!(
			rendered.contains("libcurl"),
			"existing dep kept:\n{rendered}"
		);
		assert!(rendered.contains("libssl"), "new dep added:\n{rendered}");
		assert!(
			!rendered.contains("\\\n"),
			"continuation must be collapsed:\n{rendered}"
		);
	}

	#[test]
	fn install_verb_variants_are_recognized() {
		let content = "all: install-deps\n\ninstall-deps:\n\tnix-env -iA nixpkgs.curl\n";
		let mut makefile = Makefile::parse(content).unwrap();
		makefile.add_dependencies(&["zlib".to_string()]).unwrap();
		let rendered = makefile.to_makefile_string();
		assert!(rendered.contains("zlib"), "{rendered}");
		assert!(rendered.contains("curl"), "{rendered}");
	}
}
