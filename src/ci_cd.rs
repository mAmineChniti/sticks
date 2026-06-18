use anyhow::{Context, Result};
use std::fs;
use std::str::FromStr;

/// Represents CI/CD platform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiPlatform {
	GitHubActions,
	GitLabCI,
}

impl FromStr for CiPlatform {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self> {
		match s.to_lowercase().as_str() {
			"github" | "github-actions" | "gha" => Ok(CiPlatform::GitHubActions),
			"gitlab" | "gitlab-ci" | "gl" => Ok(CiPlatform::GitLabCI),
			_ => anyhow::bail!("Invalid CI platform: {}. Valid options: github, gitlab", s),
		}
	}
}

impl CiPlatform {
	pub fn as_str(&self) -> &'static str {
		match self {
			CiPlatform::GitHubActions => "GitHub Actions",
			CiPlatform::GitLabCI => "GitLab CI",
		}
	}
}

/// CI/CD template generator
pub struct CiCdGenerator {
	pub platform: CiPlatform,
}

impl CiCdGenerator {
	pub fn new(platform: CiPlatform) -> Self {
		CiCdGenerator { platform }
	}

	/// Generate GitHub Actions workflow
	fn generate_github_actions(&self, _project_name: &str, language: &str) -> String {
		let lang_ext = if language == "cpp" { "cpp" } else { "c" };

		format!(
			r#"name: CI

on:
  push:
    branches: [ master, main ]
  pull_request:
    branches: [ master, main ]

jobs:
  build:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v4

    - name: Install dependencies
      run: |
        sudo apt-get update
        sudo apt-get install -y cmake g++ gcc

    - name: Configure CMake
      run: cmake -B build

    - name: Build
      run: cmake --build build

    - name: Test
      run: ctest --test-dir build

  lint:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v4

    - name: Install clang-tidy
      run: sudo apt-get install -y clang-tidy

    - name: Run clang-tidy
      run: |
        find src -name '*.{0}' -exec clang-tidy {{}} -- \;
"#,
			lang_ext
		)
	}

	/// Generate GitLab CI configuration
	fn generate_gitlab_ci(&self, _project_name: &str, language: &str) -> String {
		let lang_ext = if language == "cpp" { "cpp" } else { "c" };

		format!(
			r#"image: ubuntu:latest

stages:
  - build
  - test
  - lint

before_script:
  - apt-get update && apt-get install -y cmake g++ gcc clang-tidy

build:
  stage: build
  script:
    - cmake -B build
    - cmake --build build
  artifacts:
    paths:
      - build/

test:
  stage: test
  script:
    - ctest --test-dir build
  dependencies:
    - build

lint:
  stage: lint
  script:
    - find src -name '*.{0}' -exec clang-tidy {{}} -- \;
"#,
			lang_ext
		)
	}

	/// Generate CI/CD configuration file
	pub fn generate(&self, project_name: &str, language: &str) -> String {
		match self.platform {
			CiPlatform::GitHubActions => self.generate_github_actions(project_name, language),
			CiPlatform::GitLabCI => self.generate_gitlab_ci(project_name, language),
		}
	}

	/// Write CI/CD configuration to file
	pub fn write_to_file(&self, project_name: &str, language: &str) -> Result<()> {
		let content = self.generate(project_name, language);

		match self.platform {
			CiPlatform::GitHubActions => {
				fs::create_dir_all(".github/workflows")
					.context("Failed to create .github/workflows directory")?;
				fs::write(".github/workflows/ci.yml", content)
					.context("Failed to write GitHub Actions workflow")?;
			}
			CiPlatform::GitLabCI => {
				fs::write(".gitlab-ci.yml", content)
					.context("Failed to write GitLab CI configuration")?;
			}
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

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
}
