//! C/C++ toolchain detection and overrides.
//!
//! Generated Makefiles get a detected default compiler (`CC ?=`/`CXX ?=`)
//! while still honouring environment and `make` overrides. Generated
//! CMake projects only pin a compiler when the user asks for one
//! explicitly (`--cc`/`--cxx`), because CMake detects the platform
//! toolchain itself.

use std::process::Command;

/// Compiler selection for project generation.
///
/// `None` fields mean "auto": Makefiles fall back to a probed default,
/// CMake leaves compiler selection to CMake itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Toolchain {
	pub cc: Option<String>,
	pub cxx: Option<String>,
}

impl Toolchain {
	/// Probe `PATH` (and `CC`/`CXX` env vars) for an available compiler.
	/// Falls back to platform-generic names when nothing is found.
	pub fn detect() -> Self {
		Self {
			cc: Some(detect_compiler(
				&["CC", "cc", "gcc", "clang", "cl"],
				fallback_cc(),
			)),
			cxx: Some(detect_compiler(
				&["CXX", "c++", "g++", "clang++", "cl"],
				fallback_cxx(),
			)),
		}
	}

	/// Merge explicit overrides over detected defaults.
	pub fn resolved(&self) -> Self {
		let detected = Self::detect();
		Self {
			cc: self.cc.clone().or(detected.cc),
			cxx: self.cxx.clone().or(detected.cxx),
		}
	}
}

fn fallback_cc() -> &'static str {
	if cfg!(windows) { "cl" } else { "cc" }
}

fn fallback_cxx() -> &'static str {
	if cfg!(windows) { "cl" } else { "c++" }
}

/// Returns the first candidate that can be spawned. `CC`/`CXX` environment
/// variables are tried first so user environments are respected.
fn detect_compiler(env_and_candidates: &[&str], fallback: &str) -> String {
	for candidate in env_and_candidates {
		let resolved = match candidate_env_name(candidate) {
			Some(env_name) => match std::env::var(env_name) {
				Ok(value) if !value.trim().is_empty() => value,
				_ => continue,
			},
			None => (*candidate).to_string(),
		};
		let program = resolved.split_whitespace().next().unwrap_or(&resolved);
		if command_runs(program) {
			return resolved;
		}
	}
	fallback.to_string()
}

/// `CC`/`CXX` are environment variables; everything else is a PATH lookup.
fn candidate_env_name(candidate: &str) -> Option<&'static str> {
	match candidate {
		"CC" => Some("CC"),
		"CXX" => Some("CXX"),
		_ => None,
	}
}

fn command_runs(program: &str) -> bool {
	// Spawning is enough: `cl` does not understand `--version`, and a
	// missing binary fails to spawn at all.
	Command::new(program)
		.arg("--version")
		.stdin(std::process::Stdio::null())
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.status()
		.is_ok()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn detect_returns_some_compiler_names() {
		let toolchain = Toolchain::detect();
		assert!(toolchain.cc.as_deref().is_some_and(|name| !name.is_empty()));
		assert!(
			toolchain
				.cxx
				.as_deref()
				.is_some_and(|name| !name.is_empty())
		);
	}

	#[test]
	fn overrides_win_over_detection() {
		let explicit = Toolchain {
			cc: Some("clang".to_string()),
			cxx: None,
		};
		let resolved = explicit.resolved();
		assert_eq!(resolved.cc.as_deref(), Some("clang"));
		assert!(resolved.cxx.is_some());
	}

	#[test]
	fn empty_toolchain_resolves_to_detection() {
		let resolved = Toolchain::default().resolved();
		assert!(resolved.cc.is_some() && resolved.cxx.is_some());
	}
}
