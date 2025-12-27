# BPSR Fishing Macro


[![GitHub](https://img.shields.io/github/downloads/rdsp04/bpsr-fishing/total?style=for-the-badge&color=%23ff9800)](https://github.com/rdsp04/bpsr-fishing/releases/latest)

[![GitHub](https://img.shields.io/github/v/release/rdsp04/bpsr-fishing?style=flat)](https://github.com/rdsp04/bpsr-fishing/releases)
[![GitHub](https://img.shields.io/github/license/rdsp04/bpsr-fishing?style=flat)](https://github.com/rdsp04/bpsr-fishing/blob/master/LICENSE)

A fishing automation script for Blue Protocol: Star Resonance.

## About

This application is now available in two versions:
- **Python version** - Original implementation using Python with pynput, OpenCV, and pywebview
- **Rust version** - New high-performance implementation using Rust with tokio async runtime

The Rust version provides:
- Faster async handling with tokio runtime
- Better memory management and performance
- Native Windows API integration
- Same functionality as the Python version

## Download

https://github.com/rdsp04/bpsr-fishing/releases/latest

only available on windows

## Instructions

1. Start Blue Protocol: Star Resonance.
2. Open BPSR Fishing.
3. Set the game window to **1920x1080 resolution**. (Fullscreen or windowed version works)
4. Ensure the player character is in a **fishing position** before starting.
5. Check your amount of baits and rods, if you are not sure about the amount check **Recommendations**.
6. Now that you are fully ready press F9 and enjoy your exp.

## Usage

- Press **F9** to start the macro.
- Press **F10** to stop the macro.
- The script now keeps track of catches, fish types, XP, and sessions.

## Building from Source

### Python Version

```bash
# Install dependencies
pip install -r requirements.txt

# Or using uv
uv sync

# Run
python main.py
```

### Rust Version

```bash
# Build release version (Windows)
cargo build --release

# Run
cargo run --release
```

**Requirements for Rust build:**
- Rust toolchain (1.70+)
- Windows SDK (for Windows API bindings)
- Visual Studio Build Tools

## Automated Releases

This project uses GitHub Actions to automatically build and publish releases.

### Creating a New Release

1. **Via Git Tag** (Recommended):
   ```bash
   git tag v1.2.2
   git push origin v1.2.2
   ```
   This will trigger the workflow and create a release automatically.

2. **Via GitHub Actions UI**:
   - Go to Actions → "Build and Release"
   - Click "Run workflow"
   - Enter the version number (e.g., `1.2.2`)
   - Click "Run workflow"

The workflow will:
- Build the Rust application for Windows
- Create an NSIS installer (`bpsr-fishing_x.x.x_x64-Setup.exe`)
- Create a GitHub release with the installer and standalone executable

## FAQ

- If the script presses the **Exit** button instead of **Continue**, restart the script.
- The script must be launched **after the game is opened**.
- The game must be placed on the **main monitor**.
- The game window must be **visible** for the script to work properly.
- To open script type **bpsr-fishing** in the windows search.
- If you are unable to find script, open any folder > go to search bar and type > C:\Users\YourUsername\AppData\Local\bpsr-fishing

## Recommendations

- For every hour of fishing, it is recommended to have at least **200 baits** and **10 rods**.
