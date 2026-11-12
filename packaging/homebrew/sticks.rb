# Homebrew formula template for sticks.
#
# To publish: copy into a homebrew tap (e.g. `homebrew-sticks`), update the
# url/sha256 for each release, and `brew install <tap>/sticks`.
#
# sticks is a static Rust binary; the release tarball/asset names follow the
# release workflow: sticks-<os>-<arch>.

class Sticks < Formula
  desc "CLI for scaffolding and managing C/C++ projects (Makefile/CMake, Conan, vcpkg)"
  homepage "https://github.com/mAmineChniti/sticks"
  license "MIT"

  # Update per release:
  version "0.4.1"
  if OS.mac?
    if Hardware::CPU.intel?
      url "https://github.com/mAmineChniti/sticks/releases/download/v#{version}/sticks-macos-x86_64"
      sha256 "REPLACE_WITH_SHA256_FROM_RELEASES_SHA256SUMS"
    else
      url "https://github.com/mAmineChniti/sticks/releases/download/v#{version}/sticks-macos-aarch64"
      sha256 "REPLACE_WITH_SHA256_FROM_RELEASES_SHA256SUMS"
    end
  elsif OS.linux?
    if Hardware::CPU.intel?
      url "https://github.com/mAmineChniti/sticks/releases/download/v#{version}/sticks-linux-x86_64"
      sha256 "REPLACE_WITH_SHA256_FROM_RELEASES_SHA256SUMS"
    else
      url "https://github.com/mAmineChniti/sticks/releases/download/v#{version}/sticks-linux-aarch64"
      sha256 "REPLACE_WITH_SHA256_FROM_RELEASES_SHA256SUMS"
    end
  end

  def install
    bin.install "sticks-" + (OS.mac? ? "macos" : "linux") + "-" + (Hardware::CPU.intel? ? "x86_64" : "aarch64") => "sticks"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/sticks --version")
  end
end
