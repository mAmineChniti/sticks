extern crate clap;

use clap::{App, Arg, SubCommand};
use std::fs::{self, File};
use std::io::{Write, Read};
use std::env;

fn remove_dependency(_dependency_name: &str) {
    // TODO: implement the removal of dependencies and the install_deps rule if 0 deps are left 
}

fn create_dir(project_name: &str) {
    // Create project directory
    fs::create_dir(project_name).expect("Failed to create project directory");
    env::set_current_dir(project_name).expect("Failed to change directory");
}

fn create_project(project_name: &str, language: &str) {
    // Create src directory
    fs::create_dir("src").expect("Failed to create src directory");

    // Create source file (project_name.c or project_name.cpp)
    let source_file = format!("src/{}.{}", project_name, language);
    File::create(&source_file).expect("Failed to create source file");

    // Determine the compiler based on the language
    let cc = match language {
        "c" => "gcc",
        "cpp" => "g++",
        _ => "gcc", // Default to C compiler
    };

    // Create Makefile with the appropriate CC variable
    let makefile_content = format!(
        "CC = {}\n\
        CFLAGS = -Wall -g\n\
        \n\
        all: clean {}\n\
        \n\
        {}: {}\n\
        \t$(CC) $(CFLAGS) -o {} $<\n\
        \n\
        clean:\n\
        \trm -f {}\n",
        cc, project_name, project_name, source_file, project_name, project_name
    );

    // Write "Hello, World!" code based on the selected language
    let hello_world_code = match language {
        "c" => {
            // Your C code goes here
            r#"
#include <stdio.h>

int main() {
    printf("Hello, World!\n");
    return 0;
}
"#
        }
        "cpp" => {
            // Your C++ code goes here
            r#"
#include <iostream>

int main() {
    std::cout << "Hello, World!" << std::endl;
    return 0;
}
"#
        }
        _ => "",
    };

    // Write the code to the source file
    let mut source_file = File::create(&source_file).expect("Failed to create source file");
    source_file.write_all(hello_world_code.as_bytes()).expect("Failed to write code to file");

    // Create the Makefile
    let mut makefile = File::create("Makefile").expect("Failed to create Makefile");
    makefile.write_all(makefile_content.as_bytes()).expect("Failed to write to Makefile");
}

fn new_project(project_name: &str, language: &str) {
    create_dir(project_name);
    create_project(project_name, language);
}

fn init_project(language: &str) {
    // Get the current directory name as the project name
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let current_dir_name = current_dir
        .file_name()
        .expect("Failed to get directory name")
        .to_str()
        .expect("Failed to convert to string");
    create_project(current_dir_name, language);
}

fn add_dependency(dependency_name: &str) {
    // Read the existing Makefile content
    let mut makefile_content = String::new();
    let mut makefile = fs::OpenOptions::new()
        .read(true)
        .open("Makefile")
        .expect("Failed to open Makefile");
    makefile
        .read_to_string(&mut makefile_content)
        .expect("Failed to read Makefile");
    makefile_content = makefile_content.replace("all: clean", "all: clean install-deps");

    // Find the "sudo apt install -y" line and append the new dependency to it
    if let Some(index) = makefile_content.find("sudo apt install -y") {
        let install_deps_line = &makefile_content[index..];
        if let Some(eol_index) = install_deps_line.find('\n') {
            // Split the line into two parts and add the dependency in between
            let (before, after) = install_deps_line.split_at(eol_index);
            let modified_line = format!("{} {}{}", before, dependency_name, after);

            // Replace the old line with the modified one
            makefile_content = makefile_content.replace(install_deps_line, &modified_line);
        }
    } else {
        // If the line doesn't exist, you can create it
        makefile_content.push_str(&format!(
            r#"
# Add a rule to install dependencies
install-deps:
    sudo apt install -y {}
"#,
            dependency_name
        ));
    }

    // Write the modified content back to the Makefile
    let mut makefile = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open("Makefile")
        .expect("Failed to open Makefile");
    makefile
        .write_all(makefile_content.as_bytes())
        .expect("Failed to write to Makefile");
}

fn main() {
    let matches = App::new("sticks")
        .version("0.1")
        .about("A tool for managing C and C++ projects")
        .subcommand(
            SubCommand::with_name("c")
                .about("Create a C project")
                .arg(Arg::with_name("project_name").required(true)),
        )
        .subcommand(
            SubCommand::with_name("cpp")
                .about("Create a C++ project")
                .arg(Arg::with_name("project_name").required(true)),
        )
        .subcommand(
            SubCommand::with_name("init")
                .about("Initialize a project")
                .arg(Arg::with_name("language").required(true).possible_values(&["c", "cpp"])),
        )
        .subcommand(
            SubCommand::with_name("add")
                .about("Add a dependency rule to the Makefile")
                .arg(Arg::with_name("dependency_name").required(true)),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .about("Remove a dependency from the Makefile")
                .arg(Arg::with_name("dependency_name").required(true)),
        )
        .get_matches();

    match matches.subcommand() {
        ("c", Some(sub_m)) => {
            new_project(sub_m.value_of("project_name").unwrap(), "c");
        }
        ("cpp", Some(sub_m)) => {
            new_project(sub_m.value_of("project_name").unwrap(), "cpp");
        }
        ("init", Some(sub_m)) => {
            init_project(sub_m.value_of("language").unwrap());
        }
        ("add", Some(sub_m)) => {
            add_dependency(sub_m.value_of("dependency_name").unwrap());
        }
        ("remove", Some(sub_m)) => {
            remove_dependency(sub_m.value_of("dependency_name").unwrap());
        }
        _ => println!("Unknown command"),
    }
}
