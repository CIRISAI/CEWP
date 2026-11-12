// Stamps the CEWP commit the crate is built from (CEWP_GIT_COMMIT), so evidence bundles and the page's
// source link never carry a hand-written, stale hash. Outside a git checkout (a published crate) the
// stamp is "unknown".
use std::process::Command;
fn main() {
    let sha = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| s.len() == 40)
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=CEWP_GIT_COMMIT={sha}");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/heads");
}
