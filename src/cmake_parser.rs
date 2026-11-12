/// Shared, multiline-aware CMake call parser.
/// Handles `command ( args )` split across lines, quoted parens, and `#` comments.
#[derive(Debug, Clone)]
pub struct CmakeCall {
	pub command: String,
	pub args: Vec<String>,
	pub start_byte: usize,
	pub end_byte: usize,
	pub start_line: usize,
	pub end_line: usize,
}

fn strip_comment(segment: &str) -> &str {
	let mut in_single = false;
	let mut in_double = false;
	let mut chars = segment.char_indices().peekable();
	while let Some((index, character)) = chars.next() {
		match character {
			'\\' if in_single || in_double => {
				chars.next();
			}
			'\'' if !in_double => in_single = !in_single,
			'"' if !in_single => in_double = !in_double,
			'#' if !in_single && !in_double => return &segment[..index],
			_ => {}
		}
	}
	segment
}

/// Tracks CMake quoting and comment state across a forward scan so that
/// command names appearing inside string literals or `#` comments are not
/// mistaken for real invocations.
#[derive(Debug, Clone, Copy, Default)]
struct QuoteState {
	in_single: bool,
	in_double: bool,
	in_comment: bool,
}

impl QuoteState {
	/// True when the current byte cannot start a real command invocation.
	fn is_inert(&self) -> bool {
		self.in_single || self.in_double || self.in_comment
	}

	/// Consumes `text`, updating the state.
	fn advance(&mut self, text: &str) {
		let mut chars = text.chars();
		while let Some(character) = chars.next() {
			if self.in_comment {
				if character == '\n' {
					self.in_comment = false;
				}
				continue;
			}
			match character {
				// A backslash escapes the next character inside a quoted
				// string, so that character must not affect the state.
				'\\' if self.in_single || self.in_double => {
					chars.next();
				}
				'\'' if !self.in_double => self.in_single = !self.in_single,
				'"' if !self.in_single => self.in_double = !self.in_double,
				'#' if !self.in_single && !self.in_double => self.in_comment = true,
				_ => {}
			}
		}
	}
}

fn line_start_offsets(content: &str) -> Vec<usize> {
	let mut offsets = vec![0];
	for (index, byte) in content.bytes().enumerate() {
		if byte == b'\n' {
			offsets.push(index + 1);
		}
	}
	offsets
}

fn byte_to_line(offsets: &[usize], byte: usize) -> usize {
	match offsets.binary_search(&byte) {
		Ok(line) => line,
		Err(next) => next.saturating_sub(1),
	}
}

fn find_closing_paren(content: &str, open: usize) -> Option<usize> {
	let bytes = content.as_bytes();
	let mut depth = 0usize;
	let mut in_single = false;
	let mut in_double = false;
	let mut index = open;
	while index < bytes.len() {
		let byte = bytes[index];
		if in_single {
			if byte == b'\'' {
				in_single = false;
			} else if byte == b'\\' {
				index += 1;
			}
		} else if in_double {
			if byte == b'"' {
				in_double = false;
			} else if byte == b'\\' {
				index += 1;
			}
		} else {
			match byte {
				b'\'' => in_single = true,
				b'"' => in_double = true,
				b'#' => {
					while index < bytes.len() && bytes[index] != b'\n' {
						index += 1;
					}
					continue;
				}
				b'(' => depth += 1,
				b')' => {
					if depth == 0 {
						return None;
					}
					depth -= 1;
					if depth == 0 {
						return Some(index);
					}
				}
				_ => {}
			}
		}
		index += 1;
	}
	None
}

pub fn parse_args(args_str: &str) -> Vec<String> {
	let mut args = Vec::new();
	let mut current = String::new();
	let mut in_single = false;
	let mut in_double = false;
	let mut chars = args_str.chars().peekable();
	while let Some(character) = chars.next() {
		if in_single {
			if character == '\'' {
				in_single = false;
			} else if character == '\\' {
				if let Some(next) = chars.next() {
					current.push(next);
				}
			} else {
				current.push(character);
			}
			continue;
		}
		if in_double {
			if character == '"' {
				in_double = false;
			} else if character == '\\' {
				if let Some(next) = chars.next() {
					current.push(next);
				}
			} else {
				current.push(character);
			}
			continue;
		}
		match character {
			'\'' => in_single = true,
			'"' => in_double = true,
			'#' => {
				// Comment runs to end of line when outside quotes.
				for next in chars.by_ref() {
					if next == '\n' {
						if !current.is_empty() {
							args.push(std::mem::take(&mut current));
						}
						break;
					}
				}
			}
			character if character.is_whitespace() => {
				if !current.is_empty() {
					args.push(std::mem::take(&mut current));
				}
			}
			';' => {
				if !current.is_empty() {
					args.push(std::mem::take(&mut current));
				}
			}
			_ => current.push(character),
		}
	}
	if !current.is_empty() {
		args.push(current);
	}
	args
}

/// Finds all calls to `command` in `content`, spanning multiple lines.
/// Matches `command` case-sensitively, tolerating whitespace between the
/// command name and `(`, and skipping occurrences inside string literals and
/// `#` comments.
pub fn find_calls(content: &str, command: &str) -> Vec<CmakeCall> {
	let mut calls = Vec::new();
	let offsets = line_start_offsets(content);
	let bytes_len = content.len();
	let mut search_from = 0;
	// `state` describes the quoting context at byte offset `cursor`.
	// Invariant: `cursor <= search_from <= bytes_len`.
	let mut cursor = 0;
	let mut state = QuoteState::default();
	while search_from < bytes_len {
		let Some(relative) = content[search_from..].find(command) else {
			break;
		};
		let command_start = search_from + relative;
		state.advance(&content[cursor..command_start]);
		cursor = command_start;
		// Skip occurrences that live inside a quoted string or a comment.
		if state.is_inert() {
			search_from = command_start + command.len();
			continue;
		}
		// Ensure word boundary before command (start, whitespace, `;`, `(`, `)`).
		if command_start > 0 {
			let previous = content.as_bytes()[command_start - 1] as char;
			if previous.is_ascii_alphanumeric() || previous == '_' {
				search_from = command_start + command.len();
				continue;
			}
		}
		let mut after = command_start + command.len();
		while after < bytes_len && content.as_bytes()[after].is_ascii_whitespace() {
			after += 1;
		}
		if after >= bytes_len || content.as_bytes()[after] != b'(' {
			search_from = after.max(command_start + 1);
			continue;
		}
		let Some(close) = find_closing_paren(content, after) else {
			search_from = after + 1;
			continue;
		};
		let args_str = &content[after + 1..close];
		let args = parse_args(&strip_comment_whole(args_str));
		calls.push(CmakeCall {
			command: command.to_string(),
			args,
			start_byte: command_start,
			end_byte: close + 1,
			start_line: byte_to_line(&offsets, command_start),
			end_line: byte_to_line(&offsets, close),
		});
		search_from = close + 1;
		cursor = close + 1;
	}
	calls
}

fn strip_comment_whole(text: &str) -> String {
	text.lines()
		.map(strip_comment)
		.collect::<Vec<_>>()
		.join("\n")
}

/// Re-quotes a parsed argument value so it round-trips through CMake.
/// CMake treats whitespace, `;`, and unescaped `"` as argument separators, so
/// any value containing them must be wrapped in quotes.
pub fn render_arg(value: &str) -> String {
	if value.is_empty() {
		return "\"\"".to_string();
	}
	if value.contains('"') && !value.contains('\\') {
		return format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""));
	}
	if value
		.chars()
		.any(|character| character.is_whitespace() || character == ';' || character == '"')
	{
		let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
		return format!("\"{}\"", escaped);
	}
	value.to_string()
}

/// Renders a full call, preserving the original command name and spacing is
/// not attempted; callers use this when they must rewrite an entire call.
pub fn render_call(command: &str, args: &[String]) -> String {
	format!("{}({})", command, render_args(args))
}

/// Renders arguments for a call, re-quoting each value.
pub fn render_args(args: &[String]) -> String {
	args.iter()
		.map(|arg| render_arg(arg))
		.collect::<Vec<_>>()
		.join(" ")
}

/// Replaces the byte span `[start, end)` with `replacement`.
pub fn replace_span(content: &mut String, start: usize, end: usize, replacement: &str) {
	content.replace_range(start..end, replacement);
}

/// Backwards-compatible single-line helper used by existing tests:
/// parses `command(...)` from one logical line (which may itself contain
/// embedded newlines after callers join spans).
pub fn call_arguments(logical_line: &str, command: &str) -> anyhow::Result<Vec<String>> {
	let calls = find_calls(logical_line, command);
	calls
		.into_iter()
		.next()
		.map(|call| call.args)
		.ok_or_else(|| anyhow::anyhow!("Missing {} call", command))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn finds_multiline_call() {
		let content = "project(foo)\nfind_package(OpenSSL\n  REQUIRED)\n";
		let calls = find_calls(content, "find_package");
		assert_eq!(calls.len(), 1);
		assert_eq!(calls[0].args, vec!["OpenSSL", "REQUIRED"]);
		assert_eq!(calls[0].start_line, 1);
		assert_eq!(calls[0].end_line, 2);
	}

	#[test]
	fn ignores_commented_call() {
		let content = "# find_package(Foo REQUIRED)\nfind_package(Bar REQUIRED)\n";
		let calls = find_calls(content, "find_package");
		assert_eq!(calls.len(), 1);
		assert_eq!(calls[0].args[0], "Bar");
	}

	#[test]
	fn tolerates_space_before_paren() {
		let content = "add_executable (foo main.cpp)\n";
		let calls = find_calls(content, "add_executable");
		assert_eq!(calls.len(), 1);
		assert_eq!(calls[0].args[0], "foo");
	}

	#[test]
	fn ignores_call_inside_string_literal() {
		let content = "set(DOC \"see find_package(Foo REQUIRED) for details\")\n";
		assert!(find_calls(content, "find_package").is_empty());
	}

	#[test]
	fn ignores_call_inside_single_quoted_string() {
		let content = "set(DOC 'find_package(Foo REQUIRED)')\n";
		assert!(find_calls(content, "find_package").is_empty());
	}

	#[test]
	fn finds_real_call_after_string_literal() {
		let content = "set(DOC \"see find_package(Foo REQUIRED)\")\nfind_package(Bar REQUIRED)\n";
		let calls = find_calls(content, "find_package");
		assert_eq!(calls.len(), 1);
		assert_eq!(calls[0].args[0], "Bar");
	}

	#[test]
	fn handles_escaped_quote_before_hash() {
		let content = "add_definitions(X \"a\\\"#b\")\nfind_package(Bar REQUIRED)\n";
		let calls = find_calls(content, "find_package");
		assert_eq!(calls.len(), 1);
	}

	#[test]
	fn render_arg_requotes_when_needed() {
		assert_eq!(render_arg("simple"), "simple");
		assert_eq!(render_arg("with space"), "\"with space\"");
		assert_eq!(render_arg("a;b"), "\"a;b\"");
		assert_eq!(render_arg(""), "\"\"");
	}

	#[test]
	fn render_call_preserves_spaces_in_args() {
		let args = parse_args("Pkg PATHS \"/some path/lib\"");
		assert_eq!(args, vec!["Pkg", "PATHS", "/some path/lib"]);
		assert_eq!(
			render_call("find_package", &args),
			"find_package(Pkg PATHS \"/some path/lib\")"
		);
	}
}
