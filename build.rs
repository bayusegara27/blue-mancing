//! Build script for Blue Mancing
//! Embeds Windows manifest for administrator privileges and sets application icon
//! Also reads VERSION file and exposes it as APP_VERSION environment variable

fn main() {
    // Read version from VERSION file and expose it to the build
    // VERSION file contains just the version number (e.g., "2.0.0")
    let version = std::fs::read_to_string("VERSION")
        .expect("VERSION file not found")
        .trim()
        .to_string();
    
    // Expose version to the code via environment variable
    // The 'v' prefix is added here for display purposes (e.g., "v2.0.0")
    // This matches the format expected by the update checker and UI
    println!("cargo:rustc-env=APP_VERSION=v{}", version);
    
    // Rebuild if VERSION changes
    println!("cargo:rerun-if-changed=VERSION");
    
    // Only run on Windows
    #[cfg(windows)]
    {
        embed_windows_resources();
    }
}

#[cfg(windows)]
fn embed_windows_resources() {
    // Use winres to embed the manifest and icon
    let mut res = winres::WindowsResource::new();
    
    // Set the manifest file for administrator privileges
    res.set_manifest_file("blue-mancing.manifest");
    
    // Set the application icon if it exists
    if std::path::Path::new("icons/icon.ico").exists() {
        res.set_icon("icons/icon.ico");
    }
    
    // Compile the resources
    if let Err(e) = res.compile() {
        eprintln!("Warning: Failed to compile Windows resources: {}", e);
        // Don't fail the build, just warn
    }
}
