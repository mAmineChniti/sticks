use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;
use sticks::{
	Language, add_dependencies, add_sources,
	build_config::{BuildConfig, CppStandard},
	ci_cd::CiCdGenerator,
	ci_cd::CiPlatform,
	docs::DocGenerator,
	docs::DocTool,
	multi_target::{BuildTarget, MultiTargetManager, TargetType},
	remove_dependencies,
	static_analysis::StaticAnalysisGenerator,
	static_analysis::StaticAnalysisTool,
	test_framework::TestFrameworkManager,
	update_project,
};

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
	#[command(
		after_help = "Examples:\n  sticks c myproject            # Create C project with Makefile\n  sticks c myproject --build cmake  # Create C project with CMake\n  sticks c myproject -p conan   # Create C project with Conan support"
	)]
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
	#[command(
		after_help = "Examples:\n  sticks cpp myproject          # Create C++ project with Makefile\n  sticks cpp myproject --build cmake  # Create C++ project with CMake\n  sticks cpp myproject -p vcpkg # Create C++ project with vcpkg support"
	)]
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
	#[command(
		after_help = "Examples:\n  sticks init c                 # Initialize C project\n  sticks i cpp --build cmake    # Initialize C++ project with CMake\n  sticks i c -p conan           # Initialize C project with Conan support"
	)]
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
	#[command(about = "Add dependencies to your project")]
	#[command(
		after_help = "Examples:\n  sticks add libcurl            # Add single dependency\n  sticks a libcurl openssl      # Add multiple dependencies\n  sticks add sqlite3 pthread    # Add libraries for C project\n\nSupports: Makefile (via apt), CMake (via find_package), Conan, Vcpkg"
	)]
	#[command(visible_alias = "a")]
	Add { dependency_name: Vec<String> },
	#[command(about = "Remove dependencies from your project")]
	#[command(
		after_help = "Examples:\n  sticks remove libcurl         # Remove single dependency\n  sticks r libcurl openssl      # Remove multiple dependencies\n  sticks remove sqlite3         # Remove library from project\n\nSupports: Makefile (via apt), CMake (via find_package), Conan, Vcpkg"
	)]
	#[command(visible_alias = "r")]
	Remove { dependency_name: Vec<String> },
	#[command(about = "Add new source files to your project")]
	#[command(
		after_help = "Examples:\n  sticks src utils              # Add utils.c/.cpp\n  sticks s math parser          # Add multiple source files\n  sticks src database           # Add database.c/.cpp"
	)]
	#[command(visible_alias = "s")]
	Src { source_names: Vec<String> },
	#[command(about = "Update sticks to the latest version")]
	#[command(visible_alias = "u")]
	Update,
	#[command(about = "Manage project features (build system, package managers)")]
	#[command(
		after_help = "Examples:\n  sticks feature list           # List current project features\n  sticks f convert cmake        # Convert to CMake build system\n  sticks f add-pm conan myapp   # Add Conan package manager\n  sticks f rm-pm vcpkg          # Remove vcpkg package manager"
	)]
	#[command(visible_alias = "f")]
	Feature {
		#[command(subcommand)]
		action: FeatureAction,
	},
	#[command(about = "Configure build settings (C++ standard, compiler flags, etc.)")]
	#[command(
		after_help = "Examples:\n  sticks config set-cpp-standard 17    # Set C++ standard to 17\n  sticks config add-flag -Wall -O2     # Add compiler flags\n  sticks config add-def DEBUG          # Add preprocessor definition\n  sticks config add-include include     # Add include directory\n  sticks config add-lib-dir lib         # Add library directory"
	)]
	#[command(visible_alias = "cfg")]
	Config {
		#[command(subcommand)]
		action: ConfigAction,
	},
	#[command(about = "Add test framework to project")]
	#[command(
		after_help = "Examples:\n  sticks test add gtest            # Add GoogleTest framework\n  sticks test add catch2           # Add Catch2 framework\n  sticks test add doctest          # Add Doctest framework"
	)]
	#[command(visible_alias = "t")]
	Test {
		#[command(subcommand)]
		action: TestAction,
	},
	#[command(about = "Generate CI/CD configuration")]
	#[command(
		after_help = "Examples:\n  sticks ci generate github         # Generate GitHub Actions workflow\n  sticks ci generate gitlab          # Generate GitLab CI configuration"
	)]
	Ci {
		#[command(subcommand)]
		action: CiAction,
	},
	#[command(about = "Manage build targets (executables, libraries)")]
	#[command(
		after_help = "Examples:\n  sticks target add mylib --type static    # Add static library target\n  sticks target add myapp --type exe       # Add executable target\n  sticks target list                       # List all targets"
	)]
	#[command(visible_alias = "tgt")]
	Target {
		#[command(subcommand)]
		action: TargetAction,
	},
	#[command(about = "Generate documentation configuration")]
	#[command(
		after_help = "Examples:\n  sticks docs add doxygen          # Add Doxygen documentation\n  sticks docs add sphinx            # Add Sphinx documentation"
	)]
	Docs {
		#[command(subcommand)]
		action: DocsAction,
	},
	#[command(about = "Add static analysis tools")]
	#[command(
		after_help = "Examples:\n  sticks lint add clang-tidy       # Add clang-tidy configuration\n  sticks lint add cppcheck          # Add cppcheck configuration"
	)]
	#[command(visible_alias = "l")]
	Lint {
		#[command(subcommand)]
		action: LintAction,
	},
}

#[derive(Subcommand)]
enum FeatureAction {
	#[command(about = "List detected project features")]
	List,
	#[command(about = "List project dependencies")]
	#[command(visible_alias = "deps")]
	Dependencies,
	#[command(about = "Convert between build systems (makefile <-> cmake)")]
	#[command(
		after_help = "Examples:\n  sticks f convert cmake        # Convert current project to CMake\n  sticks f convert makefile     # Convert current project to Makefile\n  sticks f convert cmake myapp  # Convert specific project to CMake"
	)]
	Convert {
		#[arg(value_parser = ["makefile", "cmake"])]
		to_system: String,
		#[arg(help = "Project name (auto-detected from current directory if not provided)")]
		project_name: Option<String>,
	},
	#[command(about = "Add a package manager to the project")]
	#[command(
		after_help = "Examples:\n  sticks f add-pm conan         # Add Conan to current project\n  sticks f add-pm vcpkg         # Add vcpkg to current project\n  sticks f add-pm conan myapp   # Add Conan to specific project"
	)]
	#[command(visible_alias = "add-pm")]
	AddPackageManager {
		#[arg(value_parser = ["conan", "vcpkg"])]
		package_manager: String,
		#[arg(help = "Project name (auto-detected if not provided)")]
		project_name: Option<String>,
	},
	#[command(about = "Remove a package manager from the project")]
	#[command(
		after_help = "Examples:\n  sticks f rm-pm conan          # Remove Conan from current project\n  sticks f rm-pm vcpkg          # Remove vcpkg from current project"
	)]
	#[command(visible_alias = "rm-pm")]
	RemovePackageManager {
		#[arg(value_parser = ["conan", "vcpkg"])]
		package_manager: String,
	},
}

#[derive(Subcommand)]
enum ConfigAction {
	#[command(about = "Set C++ standard (11, 14, 17, 20, 23)")]
	SetCppStandard {
		#[arg(value_parser = ["11", "14", "17", "20", "23"])]
		standard: String,
	},
	#[command(about = "Add compiler flags")]
	AddFlag { flags: Vec<String> },
	#[command(about = "Remove compiler flags")]
	RemoveFlag { flags: Vec<String> },
	#[command(about = "Add preprocessor definitions")]
	AddDef { defs: Vec<String> },
	#[command(about = "Remove preprocessor definitions")]
	RemoveDef { defs: Vec<String> },
	#[command(about = "Add include directories")]
	AddInclude { dirs: Vec<String> },
	#[command(about = "Remove include directories")]
	RemoveInclude { dirs: Vec<String> },
	#[command(about = "Add library directories")]
	AddLibDir { dirs: Vec<String> },
	#[command(about = "Remove library directories")]
	RemoveLibDir { dirs: Vec<String> },
	#[command(about = "Apply configuration to build files")]
	Apply,
}

#[derive(Subcommand)]
enum TestAction {
	#[command(about = "Add test framework to project")]
	Add {
		#[arg(value_parser = ["gtest", "googletest", "catch2", "doctest"])]
		framework: String,
	},
	#[command(about = "Generate test file")]
	Generate {
		project_name: String,
		#[arg(value_parser = ["gtest", "googletest", "catch2", "doctest"])]
		framework: Option<String>,
	},
}

#[derive(Subcommand)]
enum CiAction {
	#[command(about = "Generate CI/CD configuration")]
	Generate {
		#[arg(value_parser = ["github", "gitlab"])]
		platform: String,
	},
	#[command(about = "Write CI/CD configuration to file")]
	Write {
		#[arg(value_parser = ["github", "gitlab"])]
		platform: String,
	},
}

#[derive(Subcommand)]
enum TargetAction {
	#[command(about = "Add a build target")]
	Add {
		name: String,
		#[arg(long, short, value_parser = ["exe", "executable", "static", "shared"])]
		target_type: String,
		#[arg(long, short)]
		sources: Option<Vec<String>>,
		#[arg(long, short)]
		dependencies: Option<Vec<String>>,
	},
	#[command(about = "List all targets")]
	List,
	#[command(about = "Remove a target")]
	Remove { name: String },
	#[command(about = "Add targets to build files")]
	Apply,
}

#[derive(Subcommand)]
enum DocsAction {
	#[command(about = "Add documentation tool")]
	Add {
		#[arg(value_parser = ["doxygen", "sphinx"])]
		tool: String,
	},
	#[command(about = "Generate documentation configuration")]
	Generate {
		project_name: String,
		#[arg(value_parser = ["doxygen", "sphinx"])]
		tool: Option<String>,
	},
	#[command(about = "Write documentation configuration to file")]
	Write {
		project_name: String,
		#[arg(value_parser = ["doxygen", "sphinx"])]
		tool: Option<String>,
	},
}

#[derive(Subcommand)]
enum LintAction {
	#[command(about = "Add static analysis tool")]
	Add {
		#[arg(value_parser = ["clang-tidy", "cppcheck"])]
		tool: String,
	},
	#[command(about = "Generate static analysis configuration")]
	Generate {
		#[arg(value_parser = ["clang-tidy", "cppcheck"])]
		tool: Option<String>,
	},
	#[command(about = "Write static analysis configuration to file")]
	Write {
		#[arg(value_parser = ["clang-tidy", "cppcheck"])]
		tool: Option<String>,
	},
}

fn main() {
	if let Err(e) = run() {
		eprintln!("Error: {:#}", e);
		std::process::exit(1);
	}
}

fn run() -> Result<()> {
	let args: Vec<String> = env::args().collect();

	if args.len() == 1 {
		return sticks::interactive::run_interactive();
	}

	let args = handle_shortcuts(args);

	let cli = Cli::parse_from(&args);

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
				None => sticks::interactive::select_language(),
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
		Commands::Config { action } => {
			handle_config_action(action)?;
		}
		Commands::Test { action } => {
			handle_test_action(action)?;
		}
		Commands::Ci { action } => {
			handle_ci_action(action)?;
		}
		Commands::Target { action } => {
			handle_target_action(action)?;
		}
		Commands::Docs { action } => {
			handle_docs_action(action)?;
		}
		Commands::Lint { action } => {
			handle_lint_action(action)?;
		}
	}

	Ok(())
}

fn handle_feature_action(action: FeatureAction) -> Result<()> {
	use FeatureAction::*;

	match action {
		List => {
			sticks::list_features()?;
		}
		Dependencies => {
			sticks::DependencyManager::list()?;
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

			sticks::convert_build_system_interactive(current_system, target_system, &proj_name)?;
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

fn handle_config_action(action: ConfigAction) -> Result<()> {
	use ConfigAction::*;

	let cmake_path = std::path::Path::new("CMakeLists.txt");
	let makefile_path = std::path::Path::new("Makefile");

	if !cmake_path.exists() && !makefile_path.exists() {
		anyhow::bail!("No CMakeLists.txt or Makefile found in current directory");
	}

	// Parse existing configuration
	let mut config = if cmake_path.exists() {
		BuildConfig::parse_from_cmake(cmake_path)?
	} else {
		BuildConfig::parse_from_makefile(makefile_path)?
	};

	match action {
		SetCppStandard { standard } => {
			let std = standard.parse::<CppStandard>()?;
			config.set_cpp_standard(std);
			if cmake_path.exists() {
				config.apply_to_cmake(cmake_path)?;
				println!("✓ Set C++ standard to {} in CMakeLists.txt", standard);
			} else {
				config.apply_to_makefile(makefile_path)?;
				println!("✓ Set C++ standard to {} in Makefile", standard);
			}
		}
		AddFlag { flags } => {
			for flag in &flags {
				config.add_compiler_flag(flag);
			}
			if cmake_path.exists() {
				config.apply_to_cmake(cmake_path)?;
				println!(
					"✓ Added compiler flags: {} to CMakeLists.txt",
					flags.join(", ")
				);
			} else {
				config.apply_to_makefile(makefile_path)?;
				println!("✓ Added compiler flags: {} to Makefile", flags.join(", "));
			}
		}
		RemoveFlag { flags } => {
			for flag in &flags {
				config.remove_compiler_flag(flag);
			}
			if cmake_path.exists() {
				config.apply_to_cmake(cmake_path)?;
				println!(
					"✓ Removed compiler flags: {} from CMakeLists.txt",
					flags.join(", ")
				);
			} else {
				config.apply_to_makefile(makefile_path)?;
				println!(
					"✓ Removed compiler flags: {} from Makefile",
					flags.join(", ")
				);
			}
		}
		AddDef { defs } => {
			for def in &defs {
				config.add_preprocessor_def(def);
			}
			if cmake_path.exists() {
				config.apply_to_cmake(cmake_path)?;
				println!(
					"✓ Added preprocessor definitions: {} to CMakeLists.txt",
					defs.join(", ")
				);
			} else {
				config.apply_to_makefile(makefile_path)?;
				println!(
					"✓ Added preprocessor definitions: {} to Makefile",
					defs.join(", ")
				);
			}
		}
		RemoveDef { defs } => {
			for def in &defs {
				config.remove_preprocessor_def(def);
			}
			if cmake_path.exists() {
				config.apply_to_cmake(cmake_path)?;
				println!(
					"✓ Removed preprocessor definitions: {} from CMakeLists.txt",
					defs.join(", ")
				);
			} else {
				config.apply_to_makefile(makefile_path)?;
				println!(
					"✓ Removed preprocessor definitions: {} from Makefile",
					defs.join(", ")
				);
			}
		}
		AddInclude { dirs } => {
			for dir in &dirs {
				config.add_include_dir(dir);
			}
			if cmake_path.exists() {
				config.apply_to_cmake(cmake_path)?;
				println!(
					"✓ Added include directories: {} to CMakeLists.txt",
					dirs.join(", ")
				);
			} else {
				config.apply_to_makefile(makefile_path)?;
				println!(
					"✓ Added include directories: {} to Makefile",
					dirs.join(", ")
				);
			}
		}
		RemoveInclude { dirs } => {
			for dir in &dirs {
				config.remove_include_dir(dir);
			}
			if cmake_path.exists() {
				config.apply_to_cmake(cmake_path)?;
				println!(
					"✓ Removed include directories: {} from CMakeLists.txt",
					dirs.join(", ")
				);
			} else {
				config.apply_to_makefile(makefile_path)?;
				println!(
					"✓ Removed include directories: {} from Makefile",
					dirs.join(", ")
				);
			}
		}
		AddLibDir { dirs } => {
			for dir in &dirs {
				config.add_library_dir(dir);
			}
			if cmake_path.exists() {
				config.apply_to_cmake(cmake_path)?;
				println!(
					"✓ Added library directories: {} to CMakeLists.txt",
					dirs.join(", ")
				);
			} else {
				config.apply_to_makefile(makefile_path)?;
				println!(
					"✓ Added library directories: {} to Makefile",
					dirs.join(", ")
				);
			}
		}
		RemoveLibDir { dirs } => {
			for dir in &dirs {
				config.remove_library_dir(dir);
			}
			if cmake_path.exists() {
				config.apply_to_cmake(cmake_path)?;
				println!(
					"✓ Removed library directories: {} from CMakeLists.txt",
					dirs.join(", ")
				);
			} else {
				config.apply_to_makefile(makefile_path)?;
				println!(
					"✓ Removed library directories: {} from Makefile",
					dirs.join(", ")
				);
			}
		}
		Apply => {
			anyhow::bail!(
				"Configuration is now applied automatically. Use individual config commands to modify settings."
			);
		}
	}

	Ok(())
}

fn handle_test_action(action: TestAction) -> Result<()> {
	use TestAction::*;

	match action {
		Add { framework } => {
			let fw = framework.parse::<sticks::test_framework::TestFramework>()?;
			let manager = TestFrameworkManager::new(fw);

			let cmake_path = std::path::Path::new("CMakeLists.txt");
			let makefile_path = std::path::Path::new("Makefile");

			let project_name = std::env::current_dir()
				.ok()
				.and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
				.unwrap_or_else(|| "project".to_string());

			if cmake_path.exists() {
				manager.add_to_cmake(cmake_path, &project_name)?;
				println!("✓ Added {} to CMakeLists.txt", fw.as_str());
			} else if makefile_path.exists() {
				manager.add_to_makefile(makefile_path, &project_name)?;
				println!("✓ Added {} to Makefile", fw.as_str());
			} else {
				anyhow::bail!("No CMakeLists.txt or Makefile found in current directory");
			}
		}
		Generate {
			project_name,
			framework,
		} => {
			let fw_str = framework.unwrap_or_else(|| "gtest".to_string());
			let fw = fw_str.parse::<sticks::test_framework::TestFramework>()?;
			let manager = TestFrameworkManager::new(fw);
			let test_content = manager.generate_test_file(&project_name, "cpp");
			println!("{}", test_content);
		}
	}

	Ok(())
}

fn handle_ci_action(action: CiAction) -> Result<()> {
	use CiAction::*;

	match action {
		Generate { platform } => {
			let platform = platform.parse::<CiPlatform>()?;
			let generator = CiCdGenerator::new(platform);

			let project_name = std::env::current_dir()
				.ok()
				.and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
				.unwrap_or_else(|| "project".to_string());

			let content = generator.generate(&project_name, "cpp");
			println!("{}", content);
		}
		Write { platform } => {
			let platform = platform.parse::<CiPlatform>()?;
			let generator = CiCdGenerator::new(platform);

			let project_name = std::env::current_dir()
				.ok()
				.and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
				.unwrap_or_else(|| "project".to_string());

			generator.write_to_file(&project_name, "cpp")?;
			println!("✓ Wrote CI/CD configuration for {}", platform.as_str());
		}
	}

	Ok(())
}

fn handle_target_action(action: TargetAction) -> Result<()> {
	use TargetAction::*;

	let cmake_path = std::path::Path::new("CMakeLists.txt");
	let makefile_path = std::path::Path::new("Makefile");

	match action {
		Add {
			name,
			target_type,
			sources,
			dependencies,
		} => {
			if !cmake_path.exists() && !makefile_path.exists() {
				anyhow::bail!("No CMakeLists.txt or Makefile found in current directory");
			}

			let tt = target_type.parse::<TargetType>()?;

			// Parse existing targets
			let mut manager = if cmake_path.exists() {
				MultiTargetManager::parse_from_cmake(cmake_path)?
			} else {
				MultiTargetManager::parse_from_makefile(makefile_path)?
			};

			// Check if target already exists
			if manager.get_target(&name).is_some() {
				anyhow::bail!("Target '{}' already exists. Use a different name.", name);
			}

			let mut target = BuildTarget::new(name.clone(), tt);

			if let Some(srcs) = sources {
				for src in srcs {
					target.add_source(src);
				}
			}

			if let Some(deps) = dependencies {
				for dep in deps {
					target.add_dependency(dep);
				}
			}

			manager.add_target(target);

			if cmake_path.exists() {
				manager.add_to_cmake(cmake_path)?;
				println!(
					"✓ Added target '{}' ({}) to CMakeLists.txt",
					name,
					tt.as_str()
				);
			} else {
				manager.add_to_makefile(makefile_path)?;
				println!("✓ Added target '{}' ({}) to Makefile", name, tt.as_str());
			}
		}
		List => {
			if !cmake_path.exists() && !makefile_path.exists() {
				anyhow::bail!("No CMakeLists.txt or Makefile found in current directory");
			}

			let manager = if cmake_path.exists() {
				MultiTargetManager::parse_from_cmake(cmake_path)?
			} else {
				MultiTargetManager::parse_from_makefile(makefile_path)?
			};

			if manager.targets.is_empty() {
				println!("No targets found in build file.");
			} else {
				println!("Build targets:");
				for target in &manager.targets {
					println!("  - {} ({})", target.name, target.target_type.as_str());
					if !target.sources.is_empty() {
						println!("    Sources: {}", target.sources.join(", "));
					}
					if !target.dependencies.is_empty() {
						println!("    Dependencies: {}", target.dependencies.join(", "));
					}
				}
			}
		}
		Remove { name: _ } => {
			if !cmake_path.exists() && !makefile_path.exists() {
				anyhow::bail!("No CMakeLists.txt or Makefile found in current directory");
			}

			anyhow::bail!(
				"Target removal requires modifying build files. This feature is not yet implemented."
			);
		}
		Apply => {
			anyhow::bail!(
				"Targets are now added automatically. Use 'sticks target add' to add targets."
			);
		}
	}

	Ok(())
}

fn handle_docs_action(action: DocsAction) -> Result<()> {
	use DocsAction::*;

	match action {
		Add { tool } => {
			let tool = tool.parse::<DocTool>()?;
			let generator = DocGenerator::new(tool);

			let cmake_path = std::path::Path::new("CMakeLists.txt");
			let makefile_path = std::path::Path::new("Makefile");

			let project_name = std::env::current_dir()
				.ok()
				.and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
				.unwrap_or_else(|| "project".to_string());

			if cmake_path.exists() {
				generator.add_to_cmake(cmake_path, &project_name)?;
				println!("✓ Added {} to CMakeLists.txt", tool.as_str());
			} else if makefile_path.exists() {
				generator.add_to_makefile(makefile_path)?;
				println!("✓ Added {} to Makefile", tool.as_str());
			} else {
				anyhow::bail!("No CMakeLists.txt or Makefile found in current directory");
			}
		}
		Generate { project_name, tool } => {
			let tool_str = tool.unwrap_or_else(|| "doxygen".to_string());
			let tool = tool_str.parse::<DocTool>()?;
			let generator = DocGenerator::new(tool);
			let config = generator.generate_config(&project_name);
			println!("{}", config);
		}
		Write { project_name, tool } => {
			let tool_str = tool.unwrap_or_else(|| "doxygen".to_string());
			let tool = tool_str.parse::<DocTool>()?;
			let generator = DocGenerator::new(tool);
			generator.write_config(&project_name)?;
			println!("✓ Wrote documentation configuration");
		}
	}

	Ok(())
}

fn handle_lint_action(action: LintAction) -> Result<()> {
	use LintAction::*;

	match action {
		Add { tool } => {
			let tool = tool.parse::<StaticAnalysisTool>()?;
			let generator = StaticAnalysisGenerator::new(tool);

			let cmake_path = std::path::Path::new("CMakeLists.txt");
			let makefile_path = std::path::Path::new("Makefile");

			if cmake_path.exists() {
				generator.add_to_cmake(cmake_path)?;
				println!("✓ Added {} to CMakeLists.txt", tool.as_str());
			} else if makefile_path.exists() {
				generator.add_to_makefile(makefile_path)?;
				println!("✓ Added {} to Makefile", tool.as_str());
			} else {
				anyhow::bail!("No CMakeLists.txt or Makefile found in current directory");
			}
		}
		Generate { tool } => {
			let tool_str = tool.unwrap_or_else(|| "clang-tidy".to_string());
			let tool = tool_str.parse::<StaticAnalysisTool>()?;
			let generator = StaticAnalysisGenerator::new(tool);
			let config = generator.generate_config();
			println!("{}", config);
		}
		Write { tool } => {
			let tool_str = tool.unwrap_or_else(|| "clang-tidy".to_string());
			let tool = tool_str.parse::<StaticAnalysisTool>()?;
			let generator = StaticAnalysisGenerator::new(tool);
			generator.write_config()?;
			println!("✓ Wrote static analysis configuration");
		}
	}

	Ok(())
}

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
		"f" => "feature",
		"cfg" => "config",
		"t" => "test",
		"tgt" => "target",
		"l" => "lint",
		_ => return args,
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
