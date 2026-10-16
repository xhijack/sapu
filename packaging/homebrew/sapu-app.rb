# Homebrew cask for the desktop app (requires a signed, notarized .dmg).
cask "sapu-app" do
  version "X.Y.Z"
  sha256 "REPLACE"

  url "https://github.com/xhijack/sapu/releases/download/v#{version}/Sapu_#{version}_universal.dmg"
  name "Sapu"
  desc "Scan, clean and dedupe your disk safely"
  homepage "https://github.com/xhijack/sapu"

  app "Sapu.app"
end
