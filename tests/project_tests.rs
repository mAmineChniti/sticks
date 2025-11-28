use serial_test::serial;
use std::env;
use std::fs;
use std::path::Path;
use sticks::{create_project, init_project, new_project, Language};

#[test]
#[serial]
fn test_create_project_c() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_project_c_{}_{}",
		std::process::id(),
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let original_dir = env::current_dir().unwrap();

	fs::remove_dir_all(&temp_dir).ok();
	fs::create_dir_all(&temp_dir).unwrap();
	env::set_current_dir(&temp_dir).unwrap();

	let result = create_project("test_c_proj", Language::C);
	assert!(
		result.is_ok(),
		"Failed to create project: {:?}",
		result.err()
	);

	assert!(Path::new("src").exists());
	assert!(Path::new("src/main.c").exists());
	assert!(Path::new("Makefile").exists());

	let main_content = fs::read_to_string("src/main.c").unwrap();
	assert!(main_content.contains("#include <stdio.h>"));

	let makefile_content = fs::read_to_string("Makefile").unwrap();
	assert!(makefile_content.contains("CC = gcc"));

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_create_project_cpp() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_project_cpp_{}_{}",
		std::process::id(),
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let original_dir = env::current_dir().unwrap();

	fs::remove_dir_all(&temp_dir).ok();
	fs::create_dir_all(&temp_dir).unwrap();
	env::set_current_dir(&temp_dir).unwrap();

	let result = create_project("test_cpp_proj", Language::Cpp);
	assert!(
		result.is_ok(),
		"Failed to create project: {:?}",
		result.err()
	);

	assert!(Path::new("src").exists());
	assert!(Path::new("src/main.cpp").exists());
	assert!(Path::new("Makefile").exists());

	let main_content = fs::read_to_string("src/main.cpp").unwrap();
	assert!(main_content.contains("#include <iostream>"));

	let makefile_content = fs::read_to_string("Makefile").unwrap();
	assert!(makefile_content.contains("CC = g++"));

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_new_project() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_new_{}_{}",
		std::process::id(),
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let original_dir = env::current_dir().unwrap();

	fs::remove_dir_all(&temp_dir).ok();
	fs::create_dir_all(&temp_dir).unwrap();
	env::set_current_dir(&temp_dir).unwrap();

	let result = new_project("my_project", Language::C);
	assert!(result.is_ok());

	assert!(Path::new("src").exists());
	assert!(Path::new("Makefile").exists());

	env::set_current_dir(&original_dir).unwrap();
	assert!(temp_dir.join("my_project").exists());
	assert!(temp_dir.join("my_project/src").exists());
	assert!(temp_dir.join("my_project/Makefile").exists());
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_init_project() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_init_{}_{}",
		std::process::id(),
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let original_dir = env::current_dir().unwrap();

	fs::remove_dir_all(&temp_dir).ok();
	fs::create_dir_all(&temp_dir).unwrap();
	env::set_current_dir(&temp_dir).unwrap();

	let result = init_project(Language::Cpp);
	assert!(result.is_ok());

	assert!(Path::new("src").exists());
	assert!(Path::new("src/main.cpp").exists());
	assert!(Path::new("Makefile").exists());

	env::set_current_dir(original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}
