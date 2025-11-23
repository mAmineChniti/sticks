use anyhow::Result;
use clap::{Parser, Subcommand};
use sticks::{
	add_dependencies, add_sources, init_project, new_project, remove_dependencies, update_project,
	Language,
};

#[derive(Parser)]
#[command(name = "sticks")]
#[command(version, about = "A tool for managing C and C++ projects", long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	C {
		project_name: Vec<String>,
	},
	Cpp {
		project_name: Vec<String>,
	},
	Init {
		#[arg(value_parser = ["c", "cpp"])]
		language: String,
	},
	Add {
		dependency_name: Vec<String>,
	},
	Remove {
		dependency_name: Vec<String>,
	},
	Src {
		source_names: Vec<String>,
	},
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
