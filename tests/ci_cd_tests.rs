use std::str::FromStr;
use sticks::ci_cd::{CiCdGenerator, CiPlatform};
use tempfile::TempDir;

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
	let temp_dir = TempDir::new().unwrap();
	let original_dir = std::env::current_dir().unwrap();
	std::env::set_current_dir(temp_dir.path()).unwrap();

	let generator = CiCdGenerator::new(CiPlatform::GitHubActions);
	let workflow = generator.generate("test_project", "cpp");

	assert!(workflow.contains("name: CI"));
	assert!(workflow.contains("build"));
	assert!(workflow.contains("test"));

	std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_ci_cd_generate_gitlab_ci() {
	let temp_dir = TempDir::new().unwrap();
	let original_dir = std::env::current_dir().unwrap();
	std::env::set_current_dir(temp_dir.path()).unwrap();

	let generator = CiCdGenerator::new(CiPlatform::GitLabCI);
	let workflow = generator.generate("test_project", "c");

	assert!(workflow.contains("stages:"));
	assert!(workflow.contains("build"));
	assert!(workflow.contains("test"));

	std::env::set_current_dir(original_dir).unwrap();
}
