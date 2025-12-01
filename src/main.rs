use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;
use sticks::{add_dependencies, add_sources, remove_dependencies, update_project, Language};

#[derive(Parser)]
#[command(name = "sticks")]
#[command(version, about = "A tool for managing C and C++ projects")]
#[command(args_conflicts_with_subcommands = true)]
struct Cli {
	#[command(subcommand)]
	command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
	#[command(about = "Create a new C project in a subdirectory")]
	C {
		project_name: Vec<String>,
		#[arg(
			long,
			short,
			default_value = "makefile",
			help = "Build system: 'makefile' or 'cmake'"
		)]
		build: String,
		#[arg(long, short = 'p', help = "Package manager: 'conan' or 'vcpkg'")]
		package_manager: Option<String>,
	},
	#[command(about = "Create a new C++ project in a subdirectory")]
	Cpp {
		project_name: Vec<String>,
		#[arg(
			long,
			short,
			default_value = "makefile",
			help = "Build system: 'makefile' or 'cmake'"
		)]
		build: String,
		#[arg(long, short = 'p', help = "Package manager: 'conan' or 'vcpkg'")]
		package_manager: Option<String>,
	},
	#[command(about = "Initialize a project in the current directory")]
	#[command(visible_alias = "i")]
	Init {
		#[arg(value_parser = ["c", "cpp"])]
		language: Option<String>,
		#[arg(
			long,
			short,
			default_value = "makefile",
			help = "Build system: 'makefile' or 'cmake'"
		)]
		build: String,
		#[arg(long, short = 'p', help = "Package manager: 'conan' or 'vcpkg'")]
		package_manager: Option<String>,
	},
	#[command(about = "Add dependencies to your project's Makefile")]
	#[command(visible_alias = "a")]
	Add { dependency_name: Vec<String> },
	#[command(about = "Remove dependencies from your project's Makefile")]
	#[command(visible_alias = "r")]
	Remove { dependency_name: Vec<String> },
	#[command(about = "Add new source files to your project")]
	#[command(visible_alias = "s")]
	Src { source_names: Vec<String> },
	#[command(about = "Update sticks to the latest version")]
	#[command(visible_alias = "u")]
	Update,
	#[command(about = "Manage project features (build system, package managers)")]
	#[command(visible_alias = "f")]
	Feature {
		#[command(subcommand)]
		action: FeatureAction,
	},
}

#[derive(Subcommand)]
enum FeatureAction {
	#[command(about = "List detected project features")]
	List,
	#[command(about = "Convert between build systems (makefile <-> cmake)")]
	Convert {
		#[arg(value_parser = ["makefile", "cmake"])]
		to_system: String,
		#[arg(help = "Project name (auto-detected from current directory if not provided)")]
		project_name: Option<String>,
	},
	#[command(about = "Add a package manager to the project")]
	#[command(visible_alias = "add-pm")]
	AddPackageManager {
		#[arg(value_parser = ["conan", "vcpkg"])]
		package_manager: String,
		#[arg(help = "Project name (auto-detected if not provided)")]
		project_name: Option<String>,
	},
	#[command(about = "Remove a package manager from the project")]
	#[command(visible_alias = "rm-pm")]
	RemovePackageManager {
		#[arg(value_parser = ["conan", "vcpkg"])]
		package_manager: String,
	},
}

/// Program entry point that runs the CLI and terminates the process on failure.
///
/// If `run()` returns an error, prints the error (formatted with detail) to standard error
/// and exits the process with status code 1.
///
/// # Examples
///
/// ```no_run
/// main();
/// ```
fn main() {
	if let Err(e) = run() {
		eprintln!("Error: {:#}", e);
		std::process::exit(1);
	}
}

/// Parse command-line arguments and execute the selected CLI command.
///
/// This function reads program arguments, expands any single-letter shortcuts,
/// parses the CLI, and dispatches the requested command (project creation,
/// initialization, dependency/source management, update, or feature actions).
/// When invoked with no arguments or when no subcommand is provided, it runs
/// the interactive flow.
///
/// # Returns
///
/// `Ok(())` on success, or an `Err` describing the failure.
///
/// # Examples
///
/// ```no_run
/// // Typical invocation from main:
/// # use anyhow::Result;
/// # fn main() -> Result<()> {
/// run()?;
/// # Ok(())
/// # }
/// ```
fn run() -> Result<()> {
	let args: Vec<String> = env::args().collect();

	// Handle interactive mode when called with no arguments
	if args.len() == 1 {
		return sticks::interactive::run_interactive();
	}

	// Parse shortcuts and convert to full commands
	let args = handle_shortcuts(args);

	let cli = Cli::parse_from(&args);

	// If no command provided, show interactive mode
	let command = match cli.command {
		Some(cmd) => cmd,
		None => return sticks::interactive::run_interactive(),
	};

	match command {
		Commands::C {
			project_name,
			build,
			package_manager,
		} => {
			validate_project_names(&project_name)?;
			let build_system = build.parse::<sticks::BuildSystem>()?;
			for name in project_name {
				match package_manager {
					Some(ref pm_str) => {
						let pm = pm_str.parse::<sticks::PackageManager>()?;
						sticks::create_project_with_system_and_pm(
							&name,
							Language::C,
							build_system,
							pm,
						)?;
					}
					None => {
						sticks::new_project_with_system(&name, Language::C, build_system)?;
					}
				}
			}
		}
		Commands::Cpp {
			project_name,
			build,
			package_manager,
		} => {
			validate_project_names(&project_name)?;
			let build_system = build.parse::<sticks::BuildSystem>()?;
			for name in project_name {
				match package_manager {
					Some(ref pm_str) => {
						let pm = pm_str.parse::<sticks::PackageManager>()?;
						sticks::create_project_with_system_and_pm(
							&name,
							Language::Cpp,
							build_system,
							pm,
						)?;
					}
					None => {
						sticks::new_project_with_system(&name, Language::Cpp, build_system)?;
					}
				}
			}
		}
		Commands::Init {
			language,
			build,
			package_manager,
		} => {
			let lang = match language {
				Some(l) => l.parse::<Language>()?,
				None => {
					// If no language provided, trigger interactive selection
					sticks::interactive::select_language()
				}
			};
			let build_system = build.parse::<sticks::BuildSystem>()?;
			match package_manager {
				Some(pm_str) => {
					let pm = pm_str.parse::<sticks::PackageManager>()?;
					sticks::init_project_with_system_and_pm(lang, build_system, pm)?;
				}
				None => {
					sticks::init_project_with_system(lang, build_system)?;
				}
			}
		}
		Commands::Add { dependency_name } => {
			if dependency_name.is_empty() {
				anyhow::bail!("Please specify at least one dependency to add");
			}
			add_dependencies(&dependency_name)?;
		}
		Commands::Remove { dependency_name } => {
			if dependency_name.is_empty() {
				anyhow::bail!("Please specify at least one dependency to remove");
			}
			remove_dependencies(&dependency_name)?;
		}
		Commands::Src { source_names } => {
			if source_names.is_empty() {
				anyhow::bail!("Please specify at least one source file to add");
			}
			let sources: Vec<&str> = source_names.iter().map(|s| s.as_str()).collect();
			add_sources(&sources)?;
		}
		Commands::Update => {
			update_project()?;
		}
		Commands::Feature { action } => {
			handle_feature_action(action)?;
		}
	}

	Ok(())
}

/// Execute a feature management action such as listing detected features, converting the build system, or adding/removing a package manager.
///
/// The behavior depends on `action`:
/// - `List`: lists detected project features.
/// - `Convert { to_system, project_name }`: requires a detected current build system (returns an error if none), parses `to_system`, and uses `project_name` or the current directory name (falls back to `"project"`) as the project identifier before converting.
/// - `AddPackageManager { package_manager, project_name }`: parses `package_manager` and uses `project_name` or the current directory name (falls back to `"project"`) when adding the package manager to the project.
/// - `RemovePackageManager { package_manager }`: parses `package_manager` and removes it from the project.
///
/// Returns `Ok(())` on success, or an `Err` if parsing, detection, or the underlying operation fails.
///
/// # Examples
///
/// ```
/// # use anyhow::Result;
/// # use crate::FeatureAction;
/// # fn try_example() -> Result<()> {
/// handle_feature_action(FeatureAction::List)?;
/// # Ok(())
/// # }
/// ```
fn handle_feature_action(action: FeatureAction) -> Result<()> {
	use FeatureAction::*;

	match action {
		List => {
			sticks::list_features()?;
		}
		Convert {
			to_system,
			project_name,
		} => {
			let current_system = sticks::detect_build_system()?.ok_or_else(|| {
				anyhow::anyhow!("No build system detected in current project. Cannot convert.")
			})?;

			let target_system = to_system.parse::<sticks::BuildSystem>()?;
			let proj_name = project_name.unwrap_or_else(|| {
				std::env::current_dir()
					.ok()
					.and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
					.unwrap_or_else(|| "project".to_string())
			});

			sticks::convert_build_system(current_system, target_system, &proj_name)?;
		}
		AddPackageManager {
			package_manager,
			project_name,
		} => {
			let pm = package_manager.parse::<sticks::PackageManager>()?;
			let proj_name = project_name.unwrap_or_else(|| {
				std::env::current_dir()
					.ok()
					.and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
					.unwrap_or_else(|| "project".to_string())
			});

			sticks::add_package_manager_to_project(pm, &proj_name)?;
		}
		RemovePackageManager { package_manager } => {
			let pm = package_manager.parse::<sticks::PackageManager>()?;
			sticks::remove_package_manager_from_project(pm)?;
		}
	}

	Ok(())
}

/// Convert shorthand commands to full command names
fn handle_shortcuts(args: Vec<String>) -> Vec<String> {
	if args.len() < 2 {
		return args;
	}

	let mut new_args = vec![args[0].clone()];
	let first_arg = &args[1];

	let expanded = match first_arg.as_str() {
		"i" => "init",
		"s" => "src",
		"a" => "add",
		"r" => "remove",
		"u" => "update",
		_ => return args, // Not a shortcut
	};

	new_args.push(expanded.to_string());
	new_args.extend_from_slice(&args[2..]);
	new_args
}

fn validate_project_names(names: &[String]) -> Result<()> {
	if names.is_empty() {
		anyhow::bail!("Please specify at least one project name");
	}

	for name in names {
		if name.is_empty() {
			anyhow::bail!("Project name cannot be empty");
		}
		if name.starts_with('-') {
			anyhow::bail!("Project name cannot start with '-': {}", name);
		}
		if !name
			.chars()
			.all(|c| c.is_alphanumeric() || c == '_' || c == '-')
		{
			anyhow::bail!(
				"Project name can only contain alphanumeric characters, '-', or '_': {}",
				name
			);
		}
	}

	Ok(())
}