# Homebrew formula for Atlas.
#
# To publish: copy this file to demwunz/homebrew-tap/Formula/atlas.rb
# and update the version, url, and sha256 for each release.
#
# Usage:
#   brew tap demwunz/tap
#   brew install atlas

class Atlas < Formula
  desc "Fast codebase indexer and file selector for LLMs"
  homepage "https://github.com/demwunz/atlas"
  license "MIT"
  version "0.1.0"

  on_macos do
    on_arm do
      url "https://github.com/demwunz/atlas/releases/download/v#{version}/atlas-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_ARM64_MACOS"
    end

    on_intel do
      url "https://github.com/demwunz/atlas/releases/download/v#{version}/atlas-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_X86_64_MACOS"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/demwunz/atlas/releases/download/v#{version}/atlas-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_X86_64_LINUX"
    end
  end

  def install
    bin.install "atlas"
  end

  test do
    assert_match "atlas v#{version}", shell_output("#{bin}/atlas --version 2>&1", 0).strip
    # Verify describe command outputs valid JSON
    output = shell_output("#{bin}/atlas describe --format json 2>&1", 0)
    json = JSON.parse(output)
    assert_equal "atlas", json["name"]
  end
end
