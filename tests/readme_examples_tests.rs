//! Exercises every `sticks ...` command shown in README.md by running
//! `<cmd> --help` against the built binary. This keeps the documentation
//! honest: a README example that references a removed or renamed command
//! fails the test suite.

use std::process::Command;

fn sticks_binary() -> &'static str {
	env!("CARGO_BIN_EXE_sticks")
}

/// First word after `sticks` in a README bash snippet, if it names a
/// subcommand. Lines inside comments or continuations are skipped.
fn command_tokens(readme: &str) -> Vec<String> {
	let mut tokens = Vec::new();
	let mut in_bash_block = false;
	for line in readme.lines() {
		let line = line.trim();
		if line.starts_with("```") {
			in_bash_block = !in_bash_block;
			continue;
		}
		if !in_bash_block {
			continue;
		}
		let Some(rest) = line.strip_prefix("sticks") else {
			continue;
		};
		// Require whitespace or end-of-line so `sticks-foo` never matches.
		if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
			continue;
		}
		let token = rest.split_whitespace().next();
		let Some(token) = token else { continue };
		// Skip flags and `<placeholder>` tokens (e.g. `sticks <command>`).
		if token.starts_with('-') || token.starts_with('<') {
			continue;
		}
		tokens.push(token.to_string());
	}
	tokens.sort();
	tokens.dedup();
	tokens
}

#[test]
fn readme_examples_reference_real_commands() {
	let readme = include_str!("../README.md");
	let tokens = command_tokens(readme);
	assert!(
		!tokens.is_empty(),
		"expected to find `sticks ...` examples in README.md"
	);

	for token in tokens {
		let output = Command::new(sticks_binary())
			.arg(&token)
			.arg("--help")
			.output()
			.unwrap_or_else(|err| panic!("failed to run sticks {token} --help: {err}"));
		assert!(
			output.status.success(),
			"README example uses unknown command `{token}`:\n{}",
			String::from_utf8_lossy(&output.stderr)
		);
	}
}

#[test]
fn readme_documents_update_and_eject() {
	let readme = include_str!("../README.md");
	for needle in [
		"sticks update",
		"sticks eject",
		"sticks build",
		"sticks run",
	] {
		assert!(
			readme.contains(needle),
			"README.md should mention `{needle}`"
		);
	}
}
