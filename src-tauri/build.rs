fn main() {
    // Embed git commit hash if available (packaged builds may not have git; fallback to "unknown").
    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
    {
        if output.status.success() {
            let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !hash.is_empty() {
                println!("cargo:rustc-env=GIT_COMMIT={hash}");
            }
        }
    }

    tauri_build::build()
}
