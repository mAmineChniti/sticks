## Contribute and Stay Updated!

🌟 If you find Sticks useful, consider giving it a star! It helps us a lot!

📢 We welcome contributions and feedback from the community. Here's how you can get involved:

### How to Contribute:

1. 🐛 **Found a bug?** Open an [issue](https://github.com/mAmineChniti/sticks/issues) to report it.
2. 🚀 **Want to add a feature?** Fork the repository, create a new branch, add your feature, and submit a [pull request](https://github.com/mAmineChniti/sticks/pulls).
3. 📝 **Spotted a typo or have improvements to the documentation?** Submit a pull request with your changes.

### Development Workflow:

#### Setting Up Development Environment

```bash
# Clone with submodules (for AUR packaging)
git clone --recurse-submodules https://github.com/mAmineChniti/sticks.git
cd sticks

# Build in debug mode
cargo build

# Run tests
cargo test

# Run specific test file
cargo test --test dependencies_tests

# Build release binary
cargo build --release
```

#### Writing Code

- Follow Rust naming conventions and idioms
- Keep functions focused and modular
- Use `anyhow::Result` for error handling with context
- Avoid comments in code - keep code self-documenting

#### Testing Guidelines

**All new features must include tests!** We maintain 100% test coverage for core functionality.

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run tests verbosely
cargo test --verbose
```

**Test Requirements:**
- Place tests in the `tests/` directory (not inline with source code)
- Use `#[serial]` attribute for tests that modify global state (like changing directories)
- Tests must clean up after themselves (delete temp directories, restore original state)
- Use unique temp directory names with process ID and timestamp
- Test both success and error cases

**Test Structure Example:**

```rust
use serial_test::serial;
use std::env;
use std::fs;

#[test]
#[serial]
fn test_my_feature() {
    // Setup: Create temp directory with unique name
    let temp_dir = env::temp_dir().join(format!(
        "sticks_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let original_dir = env::current_dir().unwrap();
    
    fs::create_dir_all(&temp_dir).unwrap();
    env::set_current_dir(&temp_dir).unwrap();
    
    // Test logic here
    // ...
    
    // Cleanup: Restore directory and remove temp files
    env::set_current_dir(&original_dir).unwrap();
    fs::remove_dir_all(&temp_dir).ok();
}
```

#### Continuous Integration

Our CI/CD pipeline automatically:

- ✅ Runs all tests on every push and pull request
- ✅ Tests must pass before any code can be merged
- ✅ Builds and releases packages on version changes
- ✅ Publishes to crates.io and creates GitHub releases
- ✅ Builds for multiple architectures (x86_64, aarch64)

**Before submitting a PR:**

1. Ensure all tests pass: `cargo test`
2. Check formatting: `cargo fmt`
3. Run linter: `cargo clippy`
4. Build successfully: `cargo build --release`

### Ways to Help Out:

- 👩‍💻 **Code Contributions:** Help us improve Sticks by contributing code changes.
- 🐞 **Bug Reports:** Report any issues you encounter and help us squash bugs.
- 📖 **Documentation:** Enhance the project's documentation to make it more accessible.
- 🧪 **Testing:** Add more tests to improve coverage and reliability.
- 🎨 **UX Improvements:** Suggest or implement better user experience.

### Code Review Process:

1. Submit your PR with a clear description
2. Automated tests run via GitHub Actions
3. Maintainer reviews code and provides feedback
4. Address any requested changes
5. Once approved and tests pass, PR is merged

### Release Process:

Releases are automated:
1. Update version in `Cargo.toml`
2. Push to master branch
3. CI automatically creates git tag, GitHub release, and publishes packages
4. AUR package is updated automatically

### Stay Updated:

- 🌐 **Watch this repository:** Click on the "Watch" button to receive notifications about new releases and updates.
- 📦 **Check Releases:** Visit [releases page](https://github.com/mAmineChniti/sticks/releases) for changelogs
- 💬 **Join Discussions:** Participate in issues and pull request discussions

Thank you for your support! Your contributions make Sticks even better! 🚀
