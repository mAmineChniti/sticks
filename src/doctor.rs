//! `sticks doctor` — read-only environment health check.
//!
//! Probes the tools sticks and the projects it manages rely on, prints a
//! plain-text report, and exits non-zero when no usable C/C++ toolchain is
//! found. It never modifies anything.

use anyhow::Result;
use std::process::Command;

struct Row {
	name: &'static str,
	detail: String,
	status: Status,
	hint: &'static str,
}

#[derive(PartialEq)]
enum Status {
	Ok,
	Missing,
}

impl Status {
	fn marker(&self) -> &'static str {
		match self {
			Status::Ok => "ok     ",
			Status::Missing => "MISSING",
		}
	}
}

/// First candidate that spawns successfully, with its `--version` output.
fn probe(candidates: &[&str]) -> Option<(String, String)> {
	for candidate in candidates {
		if let Some(version) = version_of(candidate, &["--version"]) {
			return Some(((*candidate).to_string(), version));
		}
	}
	None
}

/// Spawn `program --version` (or `args`) and return the first non-empty
/// output line, trimmed to a single readable line.
fn version_of(program: &str, args: &[&str]) -> Option<String> {
	let output = Command::new(program)
		.args(args)
		.stdin(std::process::Stdio::null())
		.output()
		.ok()?;
	let text = if output.status.success() || !output.stdout.is_empty() {
		String::from_utf8_lossy(&output.stdout)
	} else {
		String::from_utf8_lossy(&output.stderr)
	};
	text.lines()
		.map(str::trim)
		.find(|line| !line.is_empty())
		.map(|line| line.chars().take(72).collect())
}

fn tool_row(name: &'static str, candidates: &[&str], hint: &'static str) -> Row {
	match probe(candidates) {
		Some((program, version)) => Row {
			name,
			detail: format!("{} — {}", program, version),
			status: Status::Ok,
			hint,
		},
		None => Row {
			name,
			detail: "not found".to_string(),
			status: Status::Missing,
			hint,
		},
	}
}

/// Collect the report without printing — kept separate so tests can assert
/// on the rows.
fn collect() -> Vec<Row> {
	// Compilers: same candidate order sticks' toolchain detection uses.
	let mut rows = vec![
		tool_row(
			"cc",
			&["cc", "gcc", "clang", "cl"],
			"required to build C projects (override with CC or --cc)",
		),
		tool_row(
			"c++",
			&["c++", "g++", "clang++", "cl"],
			"required to build C++ projects (override with CXX or --cxx)",
		),
		tool_row(
			"make",
			&["make", "gmake", "mingw32-make"],
			"needed for Makefile projects; CMake projects can use Ninja",
		),
		tool_row(
			"cmake",
			&["cmake"],
			"needed for CMake projects and sticks build/run/test on them",
		),
		tool_row(
			"ninja",
			&["ninja", "ninja-build"],
			"optional — preferred CMake generator when present",
		),
		tool_row(
			"git",
			&["git"],
			"optional — sticks auto-inits repos when present",
		),
		tool_row("curl", &["curl"], "needed for sticks update version checks"),
		tool_row(
			"conan",
			&["conan"],
			"optional — only needed for Conan projects (conan install)",
		),
		tool_row(
			"vcpkg",
			&["vcpkg"],
			"optional — only needed for vcpkg projects (manifest mode)",
		),
	];

	// vcpkg is often a repo checkout rather than a PATH binary.
	if let (Some(root), Some(row)) = (
		std::env::var_os("VCPKG_ROOT"),
		rows.iter_mut().find(|row| row.name == "vcpkg"),
	) && row.status == Status::Missing
	{
		row.detail = format!("VCPKG_ROOT={}", root.to_string_lossy());
		row.status = Status::Ok;
	}

	rows
}

/// Print the report. Returns an error when no C or C++ compiler is usable.
pub fn run() -> Result<()> {
	let rows = collect();
	println!("sticks doctor\n");
	for row in &rows {
		println!(
			"  {:<8} {:<10} {}",
			row.status.marker(),
			row.name,
			row.detail
		);
		if row.status != Status::Ok {
			println!("           └─ {}", row.hint);
		}
	}

	let has_c = rows
		.iter()
		.any(|row| row.name == "cc" && row.status == Status::Ok);
	let has_cxx = rows
		.iter()
		.any(|row| row.name == "c++" && row.status == Status::Ok);
	if !has_c && !has_cxx {
		anyhow::bail!(
			"no usable C/C++ compiler found — install gcc, clang or MSVC, \
			 or set CC/CXX"
		);
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn collect_reports_all_expected_tools() {
		let names: Vec<&str> = collect().iter().map(|row| row.name).collect();
		for expected in [
			"cc", "c++", "make", "cmake", "ninja", "git", "curl", "conan", "vcpkg",
		] {
			assert!(names.contains(&expected), "missing probe for {expected}");
		}
	}

	#[test]
	fn version_of_returns_none_for_missing_program() {
		assert!(version_of("definitely-not-a-real-tool-xyz", &["--version"]).is_none());
	}
}
