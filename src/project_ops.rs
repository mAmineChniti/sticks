//! `sticks build|run|test|clean` — drive the project's real build system.
//!
//! CMake projects are built through `CMakePresets.json` when it provides
//! the `debug`/`release` presets sticks generates; otherwise sticks falls
//! back to plain `cmake -S/-B` commands, so the output is always standard
//! CMake/Make and never requires sticks to build.

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::BuildSystem;
use crate::features::detect_build_system_in;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildKind {
	Debug,
	Release,
}

impl BuildKind {
	fn preset_name(self) -> &'static str {
		match self {
			Self::Debug => "debug",
			Self::Release => "release",
		}
	}

	fn cmake_build_type(self) -> &'static str {
		match self {
			Self::Debug => "Debug",
			Self::Release => "Release",
		}
	}

	fn binary_dir_name(self) -> &'static str {
		self.preset_name()
	}
}

fn run_command(dir: &Path, program: &str, args: &[String]) -> Result<()> {
	let status = Command::new(program)
		.args(args)
		.current_dir(dir)
		.status()
		.with_context(|| format!("Failed to run `{program}`"))?;
	if !status.success() {
		bail!("`{program} {}` failed with {}", args.join(" "), status);
	}
	Ok(())
}

/// `ninja` on PATH → presets/commands prefer it as the CMake generator.
pub fn ninja_available() -> bool {
	Command::new("ninja")
		.arg("--version")
		.stdin(std::process::Stdio::null())
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.status()
		.is_ok()
}

/// Build presets sticks knows how to drive. `generator` is `Some("Ninja")`
/// only when ninja was available at generation time.
fn preset_generator(ninja: bool) -> Option<&'static str> {
	ninja.then_some("Ninja")
}

fn configure_preset(name: &str, build_type: &str, ninja: bool) -> Value {
	let mut preset = json!({
		"name": name,
		"displayName": format!("{} ({})", build_type, if ninja { "Ninja" } else { "default" }),
		"binaryDir": format!("${{sourceDir}}/build/{name}"),
		"cacheVariables": {
			"CMAKE_BUILD_TYPE": build_type,
			"CMAKE_EXPORT_COMPILE_COMMANDS": "ON"
		}
	});
	if let Some(generator) = preset_generator(ninja)
		&& let Some(object) = preset.as_object_mut()
	{
		object.insert("generator".to_string(), json!(generator));
	}
	preset
}

fn conan_configure_preset(ninja: bool) -> Value {
	let mut preset = json!({
		"name": "conan",
		"hidden": true,
		"binaryDir": "${sourceDir}/build",
		"cacheVariables": {
			"CMAKE_TOOLCHAIN_FILE": "${sourceDir}/build/conan_toolchain.cmake"
		}
	});
	if let Some(generator) = preset_generator(ninja)
		&& let Some(object) = preset.as_object_mut()
	{
		object.insert("generator".to_string(), json!(generator));
	}
	preset
}

/// Fresh `CMakePresets.json` content for a new CMake project.
pub fn generate_cmake_presets(include_conan: bool, ninja: bool) -> String {
	let mut configure = vec![
		configure_preset("debug", "Debug", ninja),
		configure_preset("release", "Release", ninja),
	];
	if include_conan {
		configure.push(conan_configure_preset(ninja));
	}
	let presets = json!({
		"version": 6,
		"cmakeMinimumRequired": {"major": 3, "minor": 21, "patch": 0},
		"configurePresets": configure,
		"buildPresets": [
			{"name": "debug", "configurePreset": "debug"},
			{"name": "release", "configurePreset": "release"}
		],
		"testPresets": [
			{"name": "debug", "configurePreset": "debug", "output": {"outputOnFailure": true}},
			{"name": "release", "configurePreset": "release", "output": {"outputOnFailure": true}}
		]
	});
	format!(
		"{}\n",
		serde_json::to_string_pretty(&presets).unwrap_or_default()
	)
}

fn preset_names(presets: &Value, array: &str) -> Vec<String> {
	presets
		.get(array)
		.and_then(Value::as_array)
		.map(|items| {
			items
				.iter()
				.filter_map(|item| item.get("name").and_then(Value::as_str))
				.map(str::to_string)
				.collect()
		})
		.unwrap_or_default()
}

/// Merge sticks' debug/release presets into an existing CMakePresets.json,
/// preserving user presets and never overwriting a same-named preset.
/// Returns the (possibly unchanged) file content.
pub fn merge_cmake_presets(content: &str, include_conan: bool, ninja: bool) -> Result<String> {
	let mut root: Value =
		serde_json::from_str(content).context("CMakePresets.json is not valid JSON")?;
	let object = root
		.as_object_mut()
		.context("CMakePresets.json root must be an object")?;

	let mut changed = false;
	let mut merge_array = |array: &str, wanted: Vec<Value>| -> Result<()> {
		let existing = object.entry(array.to_string()).or_insert_with(|| json!([]));
		let Some(items) = existing.as_array_mut() else {
			return Ok(());
		};
		let mut names: Vec<String> = items
			.iter()
			.filter_map(|item| item.get("name").and_then(Value::as_str))
			.map(str::to_string)
			.collect();
		for preset in wanted {
			let name = preset
				.get("name")
				.and_then(Value::as_str)
				.unwrap_or_default()
				.to_string();
			if names.iter().any(|existing| existing == &name) {
				continue;
			}
			names.push(name);
			items.push(preset);
			changed = true;
		}
		Ok(())
	};

	let mut configure = vec![
		configure_preset("debug", "Debug", ninja),
		configure_preset("release", "Release", ninja),
	];
	if include_conan {
		configure.push(conan_configure_preset(ninja));
	}
	merge_array("configurePresets", configure)?;
	merge_array(
		"buildPresets",
		vec![
			json!({"name": "debug", "configurePreset": "debug"}),
			json!({"name": "release", "configurePreset": "release"}),
		],
	)?;
	merge_array(
		"testPresets",
		vec![
			json!({"name": "debug", "configurePreset": "debug", "output": {"outputOnFailure": true}}),
			json!({"name": "release", "configurePreset": "release", "output": {"outputOnFailure": true}}),
		],
	)?;
	if !changed {
		return Ok(content.to_string());
	}
	serde_json::to_string_pretty(&root)
		.map(|mut text| {
			text.push('\n');
			text
		})
		.context("Failed to serialize CMakePresets.json")
}

/// Ensure `dir/CMakePresets.json` exists and contains the debug/release
/// presets. Existing files are merged, never overwritten; invalid JSON is
/// left alone (callers then use the direct cmake fallback).
pub fn ensure_cmake_presets(dir: &Path) -> Result<()> {
	let path = dir.join("CMakePresets.json");
	let include_conan = dir.join("conanfile.txt").is_file() || dir.join("conanfile.py").is_file();
	let ninja = ninja_available();
	match fs::read_to_string(&path) {
		Ok(content) => {
			if let Ok(merged) = merge_cmake_presets(&content, include_conan, ninja)
				&& merged != content
			{
				crate::file_handler::write_atomic(&path, &merged)?;
			}
		}
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
			crate::file_handler::write_new_file(
				&path,
				&generate_cmake_presets(include_conan, ninja),
			)?;
		}
		Err(error) => {
			return Err(error).with_context(|| format!("Failed to read {}", path.display()));
		}
	}
	Ok(())
}

/// Whether `path/CMakePresets.json` offers build+configure presets for `kind`.
fn has_presets_for(path: &Path, kind: BuildKind) -> bool {
	let Ok(content) = fs::read_to_string(path.join("CMakePresets.json")) else {
		return false;
	};
	let Ok(root) = serde_json::from_str::<Value>(&content) else {
		return false;
	};
	let name = kind.preset_name();
	preset_names(&root, "configurePresets")
		.iter()
		.any(|n| n == name)
		&& preset_names(&root, "buildPresets")
			.iter()
			.any(|n| n == name)
}

fn cmake_configure(dir: &Path, kind: BuildKind) -> Result<()> {
	let args = [
		"-S".to_string(),
		".".to_string(),
		"-B".to_string(),
		format!("build/{}", kind.binary_dir_name()),
		"-DCMAKE_BUILD_TYPE".to_string(),
		kind.cmake_build_type().to_string(),
	];
	if ninja_available() {
		// `-G Ninja` is only safe when the user has no generator preference
		// recorded; presets handle that case before we get here.
		let mut with_generator = vec!["-G".to_string(), "Ninja".to_string()];
		with_generator.extend(args);
		run_command(dir, "cmake", &with_generator)?;
	} else {
		run_command(dir, "cmake", &args)?;
	}
	Ok(())
}

fn cmake_build(dir: &Path, kind: BuildKind) -> Result<()> {
	let build_dir = format!("build/{}", kind.binary_dir_name());
	run_command(
		dir,
		"cmake",
		&[
			"--build".to_string(),
			build_dir,
			"--config".to_string(),
			kind.cmake_build_type().to_string(),
		],
	)
}

/// Configure (if needed) and build the project in `dir`.
pub fn build_project(dir: &Path, release: bool) -> Result<()> {
	let kind = if release {
		BuildKind::Release
	} else {
		BuildKind::Debug
	};
	match detect_build_system_in(dir)? {
		Some(BuildSystem::Makefile) => {
			println!("Building with make...");
			run_command(dir, "make", &[])
		}
		Some(BuildSystem::CMake) => {
			ensure_cmake_presets(dir)?;
			if has_presets_for(dir, kind) {
				let name = kind.preset_name();
				println!("Configuring with cmake --preset {name}...");
				run_command(dir, "cmake", &["--preset".to_string(), name.to_string()])?;
				println!("Building with cmake --build --preset {name}...");
				return run_command(
					dir,
					"cmake",
					&[
						"--build".to_string(),
						"--preset".to_string(),
						name.to_string(),
					],
				);
			}
			println!("Configuring with cmake...");
			cmake_configure(dir, kind)?;
			println!("Building with cmake...");
			cmake_build(dir, kind)
		}
		None => bail!("No build system detected. Run `sticks cpp <name>` or `sticks init` first."),
	}
}

/// Primary executable path inside `dir` (`bin/<name>`), if it exists.
fn primary_binary(dir: &Path) -> Result<PathBuf> {
	let name = match detect_build_system_in(dir)? {
		Some(BuildSystem::CMake) => cmake_project_name(dir)?,
		Some(BuildSystem::Makefile) => makefile_target_name(dir)?,
		None => bail!("No build system detected."),
	};
	let mut binary = dir.join("bin").join(name);
	if cfg!(windows) {
		binary.set_extension("exe");
	}
	Ok(binary)
}

fn cmake_project_name(dir: &Path) -> Result<String> {
	let content =
		fs::read_to_string(dir.join("CMakeLists.txt")).context("Failed to read CMakeLists.txt")?;
	for line in content.lines() {
		let trimmed = line.trim_start();
		if let Some(rest) = trimmed.strip_prefix("project(")
			&& let Some(name) = rest.split(|c: char| c.is_whitespace() || c == ')').next()
			&& !name.is_empty()
		{
			return Ok(crate::project_build_name(name));
		}
	}
	let fallback = dir
		.file_name()
		.and_then(|name| name.to_str())
		.unwrap_or("project");
	Ok(crate::project_build_name(fallback))
}

fn makefile_target_name(dir: &Path) -> Result<String> {
	for filename in ["Makefile", "makefile", "GNUmakefile"] {
		let path = dir.join(filename);
		if !path.is_file() {
			continue;
		}
		let content = fs::read_to_string(&path)
			.with_context(|| format!("Failed to read {}", path.display()))?;
		for line in content.lines() {
			let trimmed = line.trim_start();
			if let Some(rest) = trimmed.strip_prefix("TARGET") {
				let rest = rest.trim_start();
				if let Some(value) = rest
					.strip_prefix('=')
					.or_else(|| rest.strip_prefix("?="))
					.or_else(|| rest.strip_prefix(":="))
				{
					let value = value.trim();
					if let Some(name) = value.rsplit('/').next()
						&& !name.is_empty()
						&& !name.contains('$')
					{
						return Ok(name.to_string());
					}
				}
			}
		}
	}
	let fallback = dir
		.file_name()
		.and_then(|name| name.to_str())
		.unwrap_or("project");
	Ok(crate::project_build_name(fallback))
}

/// Build (unless skipped) then run the primary executable with `args`.
pub fn run_project(dir: &Path, release: bool, args: &[String], build: bool) -> Result<()> {
	if build {
		build_project(dir, release)?;
	}
	let binary = primary_binary(dir)?;
	if !binary.is_file() {
		bail!(
			"Executable not found at {}. Run `sticks build` first.",
			binary.display()
		);
	}
	println!("Running {}", binary.display());
	// `binary` may be relative to the process cwd while the command runs in
	// `dir`; canonicalize so the path resolves correctly either way.
	let binary = fs::canonicalize(&binary)
		.with_context(|| format!("Failed to resolve {}", binary.display()))?;
	run_command(dir, binary.to_str().context("Invalid binary path")?, args)
}

/// Run the project's tests: `ctest` for CMake, `make test` for Makefiles.
pub fn test_project(dir: &Path, release: bool) -> Result<()> {
	let kind = if release {
		BuildKind::Release
	} else {
		BuildKind::Debug
	};
	match detect_build_system_in(dir)? {
		Some(BuildSystem::CMake) => {
			ensure_cmake_presets(dir)?;
			let presets = fs::read_to_string(dir.join("CMakePresets.json")).unwrap_or_default();
			let has_test_preset =
				serde_json::from_str::<Value>(&presets)
					.ok()
					.is_some_and(|root| {
						preset_names(&root, "testPresets")
							.iter()
							.any(|n| n == kind.preset_name())
					});
			if has_test_preset {
				let name = kind.preset_name();
				println!("Running tests with ctest --preset {name}...");
				return run_command(dir, "ctest", &["--preset".to_string(), name.to_string()]);
			}
			let test_dir = format!("build/{}", kind.binary_dir_name());
			println!("Running tests with ctest...");
			run_command(
				dir,
				"ctest",
				&[
					"--test-dir".to_string(),
					test_dir,
					"--output-on-failure".to_string(),
				],
			)
		}
		Some(BuildSystem::Makefile) => {
			let makefile = ["Makefile", "makefile", "GNUmakefile"]
				.iter()
				.map(|name| dir.join(name))
				.find(|path| path.is_file())
				.ok_or_else(|| anyhow::anyhow!("No Makefile found"))?;
			let content = fs::read_to_string(&makefile)?;
			let has_test_target = content
				.lines()
				.any(|line| line.starts_with("test:") || line.starts_with("test :"));
			if !has_test_target {
				bail!(
					"No `test` target in {}. Add a test framework first: `sticks test add gtest`",
					makefile.display()
				);
			}
			println!("Running tests with make test...");
			run_command(dir, "make", &["test".to_string()])
		}
		None => bail!("No build system detected."),
	}
}

/// Remove build artifacts: `make clean` for Makefiles; the `build/` and
/// `bin/` output directories for CMake (same paths `make clean` removes).
pub fn clean_project(dir: &Path) -> Result<()> {
	match detect_build_system_in(dir)? {
		Some(BuildSystem::Makefile) => {
			println!("Cleaning with make clean...");
			run_command(dir, "make", &["clean".to_string()])
		}
		Some(BuildSystem::CMake) => {
			for name in ["build", "bin"] {
				let path = dir.join(name);
				if path.is_dir() {
					println!("Removing {}", path.display());
					fs::remove_dir_all(&path)
						.with_context(|| format!("Failed to remove {}", path.display()))?;
				}
			}
			Ok(())
		}
		None => bail!("No build system detected."),
	}
}
