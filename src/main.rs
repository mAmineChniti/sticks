use anyhow::Result;
use clap::{Parser, Subcommand};
use sticks::{add_dependencies, add_sources, remove_dependencies, update_project, Language};

#[derive(Parser)]
#[command(name = "sticks")]
#[command(version, about = "A tool for managing C and C++ projects")]
struct Cli {
	#[command(subcommand)]
	command: Commands,
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
	Init {
		#[arg(value_parser = ["c", "cpp"])]
		language: String,
		#[arg(
			long,
			short,
			default_value = "makefile",
			help = "Build system: 'makefile' or 'cmake'"
		)]
		build: String,
	},
	#[command(about = "Add dependencies to your project's Makefile")]
	Add { dependency_name: Vec<String> },
	#[command(about = "Remove dependencies from your project's Makefile")]
	Remove { dependency_name: Vec<String> },
	#[command(about = "Add new source files to your project")]
	Src { source_names: Vec<String> },
	#[command(about = "Update sticks to the latest version")]
	Update,
}

fn main() {
	if let Err(e) = run() {
		eprintln!("Error: {:#}", e);
		std::process::exit(1);
	}
}

fn run() -> Result<()> {
	let cli = Cli::parse();

	match cli.command {
		Commands::C {
			project_name,
			build,
		} => {
			validate_project_names(&project_name)?;
			let build_system = build.parse::<sticks::BuildSystem>()?;
			for name in project_name {
				sticks::create_project_with_system(&name, Language::C, build_system)?;
			}
		}
		Commands::Cpp {
			project_name,
			build,
		} => {
			validate_project_names(&project_name)?;
			let build_system = build.parse::<sticks::BuildSystem>()?;
			for name in project_name {
				sticks::create_project_with_system(&name, Language::Cpp, build_system)?;
			}
		}
		Commands::Init { language, build } => {
			let lang = language.parse::<Language>()?;
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
