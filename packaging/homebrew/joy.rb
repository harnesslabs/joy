class Joy < Formula
  desc "Native C++ package and build manager with a cargo-like CLI"
  homepage "https://github.com/harnesslabs/joy"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/harnesslabs/joy/releases/download/v#{version}/joy-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_DARWIN_ARM64_SHA256"
    else
      odie "x86_64 macOS binaries are not currently published; use cargo install --path or build from source."
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/harnesslabs/joy/releases/download/v#{version}/joy-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_LINUX_X86_64_SHA256"
    else
      odie "Only x86_64 Linux binaries are currently published for joy."
    end
  end

  def install
    bin.install "joy"
  end

  test do
    output = shell_output("#{bin}/joy --json doctor")
    assert_match '"command":"doctor"', output
  end
end
