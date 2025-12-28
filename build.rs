//! Build script for Blue Mancing
//! Embeds Windows manifest for administrator privileges and sets application icon

fn main() {
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
