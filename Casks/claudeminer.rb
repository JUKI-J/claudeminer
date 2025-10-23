cask "claudeminer" do
  version "1.0.0"
  sha256 :no_check  # Will be calculated after first release

  # TODO: Update this URL after uploading to GitHub Releases
  url "https://github.com/YOUR_USERNAME/claudeminer/releases/download/v#{version}/ClaudeMiner_#{version}_aarch64.dmg"

  name "ClaudeMiner"
  desc "Visual Process Monitor for Claude Code"
  homepage "https://github.com/YOUR_USERNAME/claudeminer"

  livecheck do
    url :url
    strategy :github_latest
  end

  # Apple Silicon only for now
  depends_on arch: :arm64
  depends_on macos: ">= :high_sierra"

  app "ClaudeMiner.app"

  # Uninstall script (optional)
  uninstall quit: "com.claudeminer.app"

  zap trash: [
    "~/Library/Application Support/com.claudeminer.app",
    "~/Library/Caches/com.claudeminer.app",
    "~/Library/Preferences/com.claudeminer.app.plist",
    "~/Library/Saved Application State/com.claudeminer.app.savedState",
  ]

  # Post-install message
  caveats <<~EOS
    ClaudeMiner requires Claude Code to be running to monitor processes.

    On first launch, you may need to:
    1. Right-click the app in Applications
    2. Select "Open"
    3. Click "Open" in the security dialog

    For more information, visit: https://github.com/YOUR_USERNAME/claudeminer
  EOS
end
