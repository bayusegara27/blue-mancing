# Blue Mancing

A fishing automation tool for Blue Protocol: Star Resonance, built with Rust.

## About

Blue Mancing is a high-performance fishing automation tool built with Rust. It provides:
- Fast async handling with tokio runtime
- Native Windows API integration
- Screen capture and template matching for game state detection
- Automatic fishing rod casting and fish catching
- Mini-game arrow detection and lane management
- Session statistics tracking
- Compact, draggable overlay UI

## Download

Get the latest release from the [Releases](https://github.com/bayusegara27/blue-mancing/releases/latest) page.

> **Note:** Only available on Windows

## Instructions

1. Start Blue Protocol: Star Resonance.
2. Open Blue Mancing.
3. Set the game window to **1920x1080 resolution**. (Fullscreen or windowed version works)
4. Ensure the player character is in a **fishing position** before starting.
5. Check your amount of baits and rods, if you are not sure about the amount check **Recommendations**.
6. Press F9 to start and enjoy your fishing session!

## Usage

- Press **F9** to start the macro.
- Press **F10** to stop the macro.
- The tool tracks catches, fish types, XP, and sessions automatically.
- Use the overlay controls to start/stop, minimize, or toggle debug info.
- Access settings through the main dashboard window.

## Building from Source

```bash
# Build release version (Windows)
cargo build --release

# Run
cargo run --release
```

**Requirements for building:**
- Rust toolchain (1.70+)
- Windows SDK (for Windows API bindings)
- Visual Studio Build Tools

## Automated Releases

This project uses GitHub Actions to automatically build and publish releases.

### Version Management

The application version is managed centrally through the `VERSION` file. This ensures consistency across all project files.

**Files that contain version information:**
- `VERSION` - Single source of truth
- `Cargo.toml` - Rust package version
- `latest.json` - Update checker metadata
- `installer.nsi` - Windows installer version
- `blue-mancing.manifest` - Windows manifest version
- `html/main.html` - UI version display

**To update the version:**

```bash
# Set a new version and sync to all files
python scripts/sync_version.py --set 2.1.0

# Check if all versions are in sync
python scripts/sync_version.py --check

# Sync version from VERSION file to all other files
python scripts/sync_version.py
```

### Creating a New Release

1. **Update the version:**
   ```bash
   python scripts/sync_version.py --set 2.1.0
   git add .
   git commit -m "Bump version to 2.1.0"
   ```

2. **Via Git Tag** (Recommended):
   ```bash
   git tag v2.1.0
   git push origin v2.1.0
   ```
   This will trigger the workflow and create a release automatically.

3. **Via GitHub Actions UI**:
   - Go to Actions → "Build and Release"
   - Click "Run workflow"
   - Enter the version number (e.g., `2.1.0`)
   - Click "Run workflow"

## FAQ

- If the script presses the **Exit** button instead of **Continue**, restart the script.
- The script must be launched **after the game is opened**.
- The game must be placed on the **main monitor**.
- The game window must be **visible** for the script to work properly.
- To open the app, type **blue-mancing** in the Windows search.

## Recommendations

- For every hour of fishing, it is recommended to have at least **200 baits** and **10 rods**.

## License

See [LICENSE](LICENSE) for details.
