# 🪨 ClaudeMiner

Visual process monitor for Claude Code with metaverse-style UI. Real-time CPU/memory tracking, zombie detection, and multi-language support.

![ClaudeMiner Screenshot](screenshots/main.png)

## ✨ Features

- 🎨 **Metaverse-style Visualization** - Beautiful animated miners representing your Claude Code processes
- ⛏️ **Real-time Monitoring** - Track CPU usage, memory consumption, and process status
- 👻 **Zombie Detection** - Automatically identify and terminate zombie processes
- 🔔 **Smart Notifications** - Get notified when processes complete or encounter issues
- 🌍 **Multi-language Support** - English, Korean (한국어), Japanese (日本語), Spanish (Español)
- 🍎 **Native macOS App** - System tray integration with low memory footprint

## 📦 Installation

### Homebrew (Recommended)

```bash
brew tap JUKI-J/tap
brew install --cask claudeminer
```

### Direct Download

Download the latest DMG from [Releases](https://github.com/JUKI-J/claudeminer/releases)

1. Download `ClaudeMiner_1.0.0_aarch64.dmg`
2. Open the DMG file
3. Drag ClaudeMiner to Applications folder
4. Launch ClaudeMiner

**Note**: This app requires macOS 10.13 (High Sierra) or later and is currently only available for Apple Silicon (M1/M2/M3).

## 🚀 Usage

1. **Launch ClaudeMiner**
   - The app will appear in your system tray (menu bar)
   - A metaverse visualization window will open

2. **Monitor Processes**
   - Each "miner" represents a Claude Code process
   - Click on a miner to see its Process ID (PID)
   - Green glow = active, Red = zombie process

3. **Manage Processes**
   - Click the red "X" button on a miner to terminate that process
   - Zombie processes can be safely terminated

4. **System Tray Menu**
   - Show/Hide: Toggle the main window
   - Quit: Exit ClaudeMiner

## 🎨 What is the Metaverse UI?

ClaudeMiner transforms boring process monitoring into an immersive experience:

- **Active Processes** = Mining workers actively digging
- **High CPU Usage** = Faster mining animation
- **Zombie Processes** = Red-tinted miners (can be removed)
- **Process Grid** = Your personal metaverse mining operation

## 🛠️ Development

### Prerequisites

- Rust 1.70+
- Node.js 18+
- macOS 10.13+

### Build from Source

```bash
# Clone the repository
git clone https://github.com/JUKI-J/claudeminer.git
cd claudeminer

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

### Tech Stack

- **Framework**: [Tauri](https://tauri.app) - Rust + WebView
- **Process Monitoring**: [sysinfo](https://github.com/GuillaumeGomez/sysinfo)
- **UI**: Vanilla JavaScript with CSS animations
- **Internationalization**: Custom i18n system

## 📋 Project Structure

```
ClaudeMiner/
├── src/                    # Frontend (HTML/CSS/JS)
│   ├── index.html         # Main UI
│   ├── styles.css         # Metaverse styling
│   ├── app.js             # Process monitoring logic
│   └── i18n.js            # Multi-language support
├── src-tauri/             # Rust backend
│   ├── src/main.rs        # Process manager
│   └── tauri.conf.json    # App configuration
└── Casks/                 # Homebrew formula
    └── claudeminer.rb
```

## 🌍 Supported Languages

- 🇺🇸 English
- 🇰🇷 한국어 (Korean)
- 🇯🇵 日本語 (Japanese)
- 🇪🇸 Español (Spanish)

Change language from the settings menu in the top-right corner.

## 🔒 Privacy & Security

ClaudeMiner:
- ✅ Only monitors processes with "claude" in the name
- ✅ Does NOT collect or transmit any data
- ✅ Runs entirely locally on your machine
- ✅ Open source - audit the code yourself

## 🐛 Known Limitations

- **macOS Only**: Currently supports Apple Silicon Macs only (Intel support coming soon)
- **Claude Code Required**: Only monitors Claude Code processes
- **First Launch Security**: On first launch, you may need to right-click the app and select "Open" to bypass Gatekeeper

## 📝 Roadmap

- [ ] Intel Mac support
- [ ] Process history and statistics
- [ ] Customizable themes
- [ ] Export monitoring data
- [ ] Windows and Linux support

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Tauri](https://tauri.app)
- Process monitoring powered by [sysinfo](https://github.com/GuillaumeGomez/sysinfo)
- Inspired by the need to manage multiple Claude Code sessions efficiently

## 💬 Support

- 🐛 **Bug Reports**: [Open an issue](https://github.com/JUKI-J/claudeminer/issues)
- 💡 **Feature Requests**: [Open an issue](https://github.com/JUKI-J/claudeminer/issues)
- 📧 **Contact**: jju.ki@hotmail.com

---

Made with ❤️ by [JUKI-J](https://github.com/JUKI-J)

**If you find ClaudeMiner useful, please consider giving it a ⭐ on GitHub!**
