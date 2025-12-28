#!/usr/bin/env python3
"""
Version Sync Script for Blue Mancing

This script reads the version from the VERSION file and updates all files
that contain version information. This ensures consistency across:
- Cargo.toml
- latest.json
- installer.nsi
- blue-mancing.manifest
- html/main.html

Usage:
    python scripts/sync_version.py           # Sync version from VERSION file
    python scripts/sync_version.py --check   # Check if versions are in sync
    python scripts/sync_version.py --set 2.1.0  # Set new version and sync
"""

import argparse
import json
import re
import sys
from pathlib import Path
from datetime import datetime, timezone

# Get the root directory (parent of scripts folder)
ROOT_DIR = Path(__file__).parent.parent


def read_version():
    """Read version from VERSION file."""
    version_file = ROOT_DIR / "VERSION"
    if not version_file.exists():
        print(f"ERROR: VERSION file not found at {version_file}")
        sys.exit(1)
    
    version = version_file.read_text().strip()
    return version


def write_version(version: str):
    """Write version to VERSION file."""
    version_file = ROOT_DIR / "VERSION"
    version_file.write_text(f"{version}\n")
    print(f"✓ Updated VERSION file to {version}")


def update_cargo_toml(version: str) -> bool:
    """Update version in Cargo.toml."""
    file_path = ROOT_DIR / "Cargo.toml"
    content = file_path.read_text()
    
    # Match version line in [package] section
    pattern = r'(^\s*version\s*=\s*")[^"]*(")'
    new_content = re.sub(pattern, rf'\g<1>{version}\2', content, count=1, flags=re.MULTILINE)
    
    if content != new_content:
        file_path.write_text(new_content)
        print(f"✓ Updated Cargo.toml to {version}")
        return True
    return False


def update_latest_json(version: str) -> bool:
    """Update version in latest.json."""
    file_path = ROOT_DIR / "latest.json"
    
    # Read existing data
    with open(file_path, 'r') as f:
        data = json.load(f)
    
    # Update version and URL
    old_version = data.get('version', '')
    data['version'] = f"v{version}"
    data['date'] = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.000Z")
    data['url'] = f"https://github.com/bayusegara27/blue-mancing/releases/download/v{version}/blue-mancing_{version}_x64-Setup.exe"
    
    # Write back
    with open(file_path, 'w') as f:
        json.dump(data, f, indent=2)
        f.write('\n')
    
    if old_version != f"v{version}":
        print(f"✓ Updated latest.json to v{version}")
        return True
    return False


def update_installer_nsi(version: str) -> bool:
    """Update version in installer.nsi."""
    file_path = ROOT_DIR / "installer.nsi"
    content = file_path.read_text()
    
    # Match the default AppVersion definition
    pattern = r'(!define AppVersion\s+")[^"]*(")'
    new_content = re.sub(pattern, rf'\g<1>{version}\2', content)
    
    if content != new_content:
        file_path.write_text(new_content)
        print(f"✓ Updated installer.nsi to {version}")
        return True
    return False


def update_manifest(version: str) -> bool:
    """Update version in blue-mancing.manifest."""
    file_path = ROOT_DIR / "blue-mancing.manifest"
    content = file_path.read_text()
    
    # Convert version to 4-part format (e.g., 2.0.0 -> 2.0.0.0)
    version_parts = version.split('.')
    while len(version_parts) < 4:
        version_parts.append('0')
    manifest_version = '.'.join(version_parts[:4])
    
    # Match version attribute in assemblyIdentity tag specifically
    # Use multiline pattern to match the version line within assemblyIdentity
    pattern = r'(<assemblyIdentity\s+version=")[^"]*(")'
    new_content = re.sub(pattern, rf'\g<1>{manifest_version}\2', content, flags=re.MULTILINE)
    
    if content != new_content:
        file_path.write_text(new_content)
        print(f"✓ Updated blue-mancing.manifest to {manifest_version}")
        return True
    return False


def update_main_html(version: str) -> bool:
    """Update version in html/main.html."""
    file_path = ROOT_DIR / "html" / "main.html"
    content = file_path.read_text()
    
    # Match version-pill span
    pattern = r'(<span class="version-pill">)v?[^<]*(</span>)'
    new_content = re.sub(pattern, rf'\g<1>v{version}\2', content)
    
    if content != new_content:
        file_path.write_text(new_content)
        print(f"✓ Updated html/main.html to v{version}")
        return True
    return False


def sync_all(version: str):
    """Sync version to all files."""
    print(f"\n🔄 Syncing version {version} to all files...\n")
    
    updated = []
    
    if update_cargo_toml(version):
        updated.append("Cargo.toml")
    if update_latest_json(version):
        updated.append("latest.json")
    if update_installer_nsi(version):
        updated.append("installer.nsi")
    if update_manifest(version):
        updated.append("blue-mancing.manifest")
    if update_main_html(version):
        updated.append("html/main.html")
    
    if updated:
        print(f"\n✅ Updated {len(updated)} file(s)")
    else:
        print("\n✅ All files already in sync")
    
    return len(updated)


def check_versions():
    """Check if all versions are in sync."""
    version = read_version()
    print(f"\n🔍 Checking version sync (expected: {version})...\n")
    
    errors = []
    
    # Check Cargo.toml
    cargo_path = ROOT_DIR / "Cargo.toml"
    cargo_content = cargo_path.read_text()
    cargo_match = re.search(r'^\s*version\s*=\s*"([^"]*)"', cargo_content, re.MULTILINE)
    if cargo_match:
        cargo_version = cargo_match.group(1)
        if cargo_version != version:
            errors.append(f"Cargo.toml: {cargo_version} (expected {version})")
        else:
            print(f"✓ Cargo.toml: {cargo_version}")
    
    # Check latest.json
    latest_path = ROOT_DIR / "latest.json"
    with open(latest_path, 'r') as f:
        latest_data = json.load(f)
    latest_version = latest_data.get('version', '').lstrip('v')
    if latest_version != version:
        errors.append(f"latest.json: v{latest_version} (expected v{version})")
    else:
        print(f"✓ latest.json: v{latest_version}")
    
    # Check installer.nsi
    nsi_path = ROOT_DIR / "installer.nsi"
    nsi_content = nsi_path.read_text()
    nsi_match = re.search(r'!define AppVersion\s+"([^"]*)"', nsi_content)
    if nsi_match:
        nsi_version = nsi_match.group(1)
        if nsi_version != version:
            errors.append(f"installer.nsi: {nsi_version} (expected {version})")
        else:
            print(f"✓ installer.nsi: {nsi_version}")
    
    # Check manifest
    manifest_path = ROOT_DIR / "blue-mancing.manifest"
    manifest_content = manifest_path.read_text()
    # Match version in assemblyIdentity tag specifically
    manifest_match = re.search(r'<assemblyIdentity\s+version="([^"]*)"', manifest_content)
    if manifest_match:
        manifest_version = manifest_match.group(1)
        expected_manifest = version + ".0" if version.count('.') == 2 else version
        if not manifest_version.startswith(version):
            errors.append(f"blue-mancing.manifest: {manifest_version} (expected {expected_manifest})")
        else:
            print(f"✓ blue-mancing.manifest: {manifest_version}")
    
    # Check main.html
    html_path = ROOT_DIR / "html" / "main.html"
    html_content = html_path.read_text()
    html_match = re.search(r'<span class="version-pill">v?([^<]*)</span>', html_content)
    if html_match:
        html_version = html_match.group(1)
        if html_version != version:
            errors.append(f"html/main.html: v{html_version} (expected v{version})")
        else:
            print(f"✓ html/main.html: v{html_version}")
    
    if errors:
        print(f"\n❌ Found {len(errors)} version mismatch(es):")
        for error in errors:
            print(f"   - {error}")
        print("\nRun 'python scripts/sync_version.py' to fix.")
        return False
    else:
        print("\n✅ All versions are in sync!")
        return True


def main():
    parser = argparse.ArgumentParser(
        description="Sync version across all project files",
        epilog="""
Examples:
  python scripts/sync_version.py           # Sync version from VERSION file
  python scripts/sync_version.py --check   # Check if versions are in sync
  python scripts/sync_version.py --set 2.1.0  # Set new version and sync
        """,
        formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        '--check',
        action='store_true',
        help='Check if all versions are in sync without making changes'
    )
    parser.add_argument(
        '--set',
        metavar='VERSION',
        help='Set a new version and sync to all files'
    )
    
    args = parser.parse_args()
    
    if args.check:
        success = check_versions()
        sys.exit(0 if success else 1)
    elif args.set:
        # Validate version format
        if not re.match(r'^\d+\.\d+\.\d+$', args.set):
            print(f"ERROR: Invalid version format '{args.set}'. Expected format: X.Y.Z (e.g., 2.1.0)")
            sys.exit(1)
        write_version(args.set)
        sync_all(args.set)
    else:
        version = read_version()
        sync_all(version)


if __name__ == "__main__":
    main()
