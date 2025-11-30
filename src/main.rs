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
}

fn main() {
	if let Err(e) = run() {
		eprintln!("Error: {:#}", e);
		std::process::exit(1);
	}
}

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
		} => {
			validate_project_names(&project_name)?;
			let build_system = build.parse::<sticks::BuildSystem>()?;
			for name in project_name {
				sticks::new_project_with_system(&name, Language::C, build_system)?;
			}
		}
		Commands::Cpp {
			project_name,
			build,
		} => {
			validate_project_names(&project_name)?;
			let build_system = build.parse::<sticks::BuildSystem>()?;
			for name in project_name {
				sticks::new_project_with_system(&name, Language::Cpp, build_system)?;
			}
		}
		Commands::Init { language, build } => {
			let lang = match language {
				Some(l) => l.parse::<Language>()?,
				None => {
					// If no language provided, trigger interactive selection
					sticks::interactive::select_language()
				}
			};
			let build_system = build.parse::<sticks::BuildSystem>()?;
			sticks::init_project_with_system(lang, build_system)?;
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
