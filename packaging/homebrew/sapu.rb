# Homebrew formula for the CLI. Submit to a tap (e.g. xhijack/homebrew-sapu)
# after the first tagged release; update version and sha256 per release.
class Sapu < Formula
  desc "Smart Analyzer & Pruning Utility: scan, clean and dedupe your disk safely"
  homepage "https://github.com/xhijack/sapu"
  version "X.Y.Z"
  license "TBD" # PRD Q2

  on_macos do
    on_arm do
      url "https://github.com/xhijack/sapu/releases/download/v#{version}/sapu-cli-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE"
    end
    on_intel do
      url "https://github.com/xhijack/sapu/releases/download/v#{version}/sapu-cli-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE"
    end
  end

  on_linux do
    url "https://github.com/xhijack/sapu/releases/download/v#{version}/sapu-cli-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "REPLACE"
  end

  def install
    bin.install "sapu"
  end

  test do
    assert_match "sapu", shell_output("#{bin}/sapu --version")
  end
end
