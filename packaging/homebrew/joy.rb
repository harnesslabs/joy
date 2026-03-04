class Joy < Formula
  desc "Native C++ package and build manager with a cargo-like CLI"
  homepage "https://github.com/harnesslabs/joy"
  license "MIT"
  version "0.1.0"
  # Checksum values here are bootstrap defaults for local syntax validation.
  # Release automation publishes a generated formula with release-specific checksums.

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/harnesslabs/joy/releases/download/v#{version}/joy-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    elsif Hardware::CPU.intel?
      url "https://github.com/harnesslabs/joy/releases/download/v#{version}/joy-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    else
      odie "Unsupported macOS architecture for joy."
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/harnesslabs/joy/releases/download/v#{version}/joy-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    else
      odie "Only x86_64 Linux binaries are currently published for joy."
    end
  end

  def install
    bin.install "joy"
  end

  test do
    output = shell_output("#{bin}/joy --json doctor")
    assert_match '"command": "doctor"', output
  end
end
