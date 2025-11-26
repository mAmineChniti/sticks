use anyhow::Result;
use clap::{Parser, Subcommand};
use sticks::{
	add_dependencies, add_sources, init_project, new_project, remove_dependencies, update_project,
	Language,
};

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
	},
	#[command(about = "Create a new C++ project in a subdirectory")]
	Cpp {
		project_name: Vec<String>,
	},
	#[command(about = "Initialize a project in the current directory")]
	Init {
		#[arg(value_parser = ["c", "cpp"])]
		language: String,
	},
	#[command(about = "Add dependencies to your project's Makefile")]
	Add {
		dependency_name: Vec<String>,
	},
	#[command(about = "Remove dependencies from your project's Makefile")]
	Remove {
		dependency_name: Vec<String>,
	},
	#[command(about = "Add new source files to your project")]
	Src {
		source_names: Vec<String>,
	},
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
		Commands::C { project_name } => {
			for name in project_name {
				new_project(&name, Language::C)?;
			}
		}
		Commands::Cpp { project_name } => {
			for name in project_name {
				new_project(&name, Language::Cpp)?;
			}
		}
		Commands::Init { language } => {
			let lang = language.parse::<Language>()?;
			init_project(lang)?;
		}
		Commands::Add { dependency_name } => {
			add_dependencies(&dependency_name)?;
		}
		Commands::Remove { dependency_name } => {
			remove_dependencies(&dependency_name)?;
		}
		Commands::Src { source_names } => {
			let sources: Vec<&str> = source_names.iter().map(|s| s.as_str()).collect();
			add_sources(&sources)?;
		}
		Commands::Update => {
			update_project()?;
		}
	}

	Ok(())
}
