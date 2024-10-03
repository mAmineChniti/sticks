// main.rs
use clap::{App, Arg, SubCommand};
use std::env;
use sticks::{
	add_dependencies, add_sources, init_project, new_project, remove_dependencies, update_project,
	Language,
};

fn main() {
	let version = env!("CARGO_PKG_VERSION");
	let matches = App::new("sticks")
		.version(version)
		.about("A tool for managing C and C++ projects")
		.subcommand(
			SubCommand::with_name("c")
				.about("Create a C project")
				.arg(Arg::with_name("project_name").required(true).multiple(true)),
		)
		.subcommand(
			SubCommand::with_name("cpp")
				.about("Create a C++ project")
				.arg(Arg::with_name("project_name").required(true).multiple(true)),
		)
		.subcommand(
			SubCommand::with_name("init")
				.about("Initialize a project in the current directory")
				.arg(
					Arg::with_name("language")
						.required(true)
						.possible_values(&["c", "cpp"]),
				),
		)
		.subcommand(
			SubCommand::with_name("add")
				.about("Add dependencies to the Makefile")
				.arg(
					Arg::with_name("dependency_name")
						.required(true)
						.multiple(true),
				),
		)
		.subcommand(
			SubCommand::with_name("remove")
				.about("Remove dependencies from the Makefile")
				.arg(
					Arg::with_name("dependency_name")
						.required(true)
						.multiple(true),
				),
		)
		.subcommand(
			SubCommand::with_name("src")
				.about("Add more source files and their headers to your project")
				.arg(Arg::with_name("source_names").required(true).multiple(true)),
		)
		.subcommand(SubCommand::with_name("update").about("Update sticks to the latest version"))
		.get_matches();

	match matches.subcommand() {
		("c", Some(sub_m)) => {
			let project_names: Vec<&str> = sub_m.values_of("project_name").unwrap().collect();
			for project_name in project_names {
				if let Err(e) = new_project(project_name, Language::C) {
					eprintln!("Error: {}", e);
					std::process::exit(1);
				}
			}
		}
		("cpp", Some(sub_m)) => {
			let project_names: Vec<&str> = sub_m.values_of("project_name").unwrap().collect();
			for project_name in project_names {
				if let Err(e) = new_project(project_name, Language::Cpp) {
					eprintln!("Error: {}", e);
					std::process::exit(1);
				}
			}
		}
		("init", Some(sub_m)) => {
			let language = sub_m
				.value_of("language")
				.unwrap()
				.parse::<Language>()
				.unwrap_or_else(|_| {
					eprintln!("Invalid language");
					std::process::exit(1);
				});
			if let Err(e) = init_project(language) {
				eprintln!("Error: {}", e);
				std::process::exit(1);
			}
		}
		("add", Some(sub_m)) => {
			let dependencies: Vec<String> = sub_m
				.values_of("dependency_name")
				.unwrap()
				.map(|s| s.to_string())
				.collect();
			if let Err(e) = add_dependencies(&dependencies) {
				eprintln!("Error: {}", e);
				std::process::exit(1);
			}
		}
		("remove", Some(sub_m)) => {
			let dependencies: Vec<String> = sub_m
				.values_of("dependency_name")
				.unwrap()
				.map(|s| s.to_string())
				.collect();
			if let Err(e) = remove_dependencies(&dependencies) {
				eprintln!("Error: {}", e);
				std::process::exit(1);
			}
		}
		("src", Some(sub_m)) => {
			let sources: Vec<&str> = sub_m
				.values_of("source_names")
				.unwrap_or_default()
				.collect();
			if let Err(e) = add_sources(&sources) {
				eprintln!("Error: {}", e);
				std::process::exit(1);
			}
		}
		("update", Some(_)) => {
			if let Err(e) = update_project() {
				eprintln!("Error: {}", e);
				std::process::exit(1);
			}
		}
		_ => {
			println!("Use --help to see available commands.");
		}
	}
}
