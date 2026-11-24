use std::str::FromStr;
use sticks::BuildSystem;
use sticks::ci_cd::{CiCdGenerator, CiPlatform};

#[test]
fn test_ci_platform_from_str() {
	assert_eq!(
		CiPlatform::from_str("github").unwrap(),
		CiPlatform::GitHubActions
	);
	assert_eq!(
		CiPlatform::from_str("gitlab").unwrap(),
		CiPlatform::GitLabCI
	);
	assert!(CiPlatform::from_str("invalid").is_err());
}

#[test]
fn test_ci_platform_strings() {
	assert_eq!(CiPlatform::GitHubActions.as_str(), "GitHub Actions");
	assert_eq!(CiPlatform::GitLabCI.as_str(), "GitLab CI");
}

#[test]
fn test_ci_cd_generator_new() {
	let generator = CiCdGenerator::new(CiPlatform::GitHubActions);
	assert_eq!(generator.platform, CiPlatform::GitHubActions);
}

#[test]
fn test_ci_cd_generate_github_actions() {
	let generator = CiCdGenerator::new(CiPlatform::GitHubActions);
	let workflow = generator.generate_for("test_project", "cpp", BuildSystem::CMake);

	assert!(workflow.contains("name: CI"));
	assert!(workflow.contains("build"));
	assert!(workflow.contains("Check formatting"));
	assert!(workflow.contains("format-check"));
}

#[test]
fn test_ci_cd_generate_gitlab_ci() {
	let generator = CiCdGenerator::new(CiPlatform::GitLabCI);
	let workflow = generator.generate_for("test_project", "c", BuildSystem::Makefile);

	assert!(workflow.contains("stages:"));
	assert!(workflow.contains("build"));
}

#[test]
fn test_ci_cd_checks_formatting_for_both_build_systems() {
	for build_system in [BuildSystem::CMake, BuildSystem::Makefile] {
		let generator = CiCdGenerator::new(CiPlatform::GitLabCI);
		let workflow = generator.generate_for("test_project", "c", build_system);
		assert!(
			workflow.contains("clang-format"),
			"GitLab workflow for {build_system:?} must install clang-format"
		);
		assert!(
			workflow.contains("format-check"),
			"GitLab workflow for {build_system:?} must check formatting"
		);
	}

	let generator = CiCdGenerator::new(CiPlatform::GitHubActions);
	let workflow = generator.generate_for("test_project", "cpp", BuildSystem::Makefile);
	assert!(workflow.contains("clang-format"));
	assert!(workflow.contains("format-check"));
}
