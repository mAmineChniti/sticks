use sticks::{Language, LanguageConsts};

#[test]
fn test_language_display() {
	assert_eq!(format!("{}", Language::C), "C");
	assert_eq!(format!("{}", Language::Cpp), "C++");
}

#[test]
fn test_language_cc() {
	assert_eq!(Language::C.cc(), "gcc");
	assert_eq!(Language::Cpp.cc(), "g++");
}

#[test]
fn test_language_extension() {
	assert_eq!(Language::C.extension(), "c");
	assert_eq!(Language::Cpp.extension(), "cpp");
}

#[test]
fn test_language_helloworld() {
	let c_hello = Language::C.generate_helloworld_content();
	assert!(c_hello.contains("#include <stdio.h>"));
	assert!(c_hello.contains("printf"));

	let cpp_hello = Language::Cpp.generate_helloworld_content();
	assert!(cpp_hello.contains("#include <iostream>"));
	assert!(cpp_hello.contains("std::cout"));
}

#[test]
fn test_language_makefile() {
	let makefile = Language::C.generate_makefile_content("test_project");
	assert!(makefile.contains("CC = gcc"));
	assert!(makefile.contains("TARGET = test_project"));
	assert!(makefile.contains("all:"));
	assert!(makefile.contains("clean:"));
	assert!(makefile.contains("BUILD_DIR = build"));
}

#[test]
fn test_language_from_str() {
	assert!(matches!("c".parse::<Language>(), Ok(Language::C)));
	assert!(matches!("C".parse::<Language>(), Ok(Language::C)));
	assert!(matches!("cpp".parse::<Language>(), Ok(Language::Cpp)));
	assert!(matches!("CPP".parse::<Language>(), Ok(Language::Cpp)));
	assert!("rust".parse::<Language>().is_err());
	assert!("java".parse::<Language>().is_err());
}
