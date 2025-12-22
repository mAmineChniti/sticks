class Sticks < Formula
  desc "A tool for managing C and C++ projects"
  homepage "https://github.com/mAmineChniti/sticks"
  version "0.3.6"
  license "MIT"

  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/mAmineChniti/sticks/releases/download/v#{version}/sticks-darwin-x86_64"
      sha256 ""  # Will be filled in by the release workflow
    else
      url "https://github.com/mAmineChniti/sticks/releases/download/v#{version}/sticks-darwin-aarch64"
      sha256 ""  # Will be filled in by the release workflow
    end
  end

  def install
    if OS.mac?
      bin.install "sticks-darwin-#{Hardware::CPU.arch}" => "sticks"
      chmod 0555, bin/"sticks"
    end
  end

  test do
    system "#{bin}/sticks", "--version"
  end
end
