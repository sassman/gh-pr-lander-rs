//! PR branch checkout helper
//!
//! Clones a repository and checks out a PR branch into a temporary directory.

use std::path::PathBuf;
use std::process::Command;

/// Parameters for checking out a PR branch
pub struct CheckoutParams {
    pub org: String,
    pub repo: String,
    pub pr_number: usize,
    /// SSH URL for setting remote origin (e.g., "git@github.com:org/repo.git")
    pub ssh_url: String,
    /// Optional GitHub Enterprise hostname
    pub host: Option<String>,
    /// Base temp directory
    pub temp_dir: String,
}

/// Clone and checkout a PR branch into a temporary directory.
///
/// Returns the path to the checked-out directory on success.
///
/// This reuses the same pattern as the IDE open feature:
/// 1. Create temp dir
/// 2. Clone via `gh repo clone`
/// 3. Checkout PR via `gh pr checkout`
/// 4. Set origin to SSH URL
pub fn checkout_pr_branch(params: &CheckoutParams) -> Result<PathBuf, String> {
    let temp_dir = PathBuf::from(&params.temp_dir);

    // Create temp directory if it doesn't exist
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    // Create unique directory name (include host for GHE)
    let host_prefix = match &params.host {
        Some(h) if h != "github.com" => format!("{}-", h.replace('.', "-")),
        _ => String::new(),
    };
    let dir_name = format!(
        "{}claude-{}-{}-pr-{}",
        host_prefix, params.org, params.repo, params.pr_number
    );
    let pr_dir = temp_dir.join(dir_name);

    // Remove existing directory if present
    if pr_dir.exists() {
        std::fs::remove_dir_all(&pr_dir)
            .map_err(|e| format!("Failed to remove existing directory: {}", e))?;
    }

    // Clone the repository using gh repo clone
    log::info!("Cloning {}/{} to {:?}", params.org, params.repo, pr_dir);
    let mut clone_args = vec![
        "repo".to_string(),
        "clone".to_string(),
        format!("{}/{}", params.org, params.repo),
        pr_dir.to_string_lossy().to_string(),
    ];
    if let Some(ref host) = params.host
        && host != "github.com"
    {
        clone_args.push("--hostname".to_string());
        clone_args.push(host.clone());
    }

    let clone_output = Command::new("gh")
        .args(&clone_args)
        .output()
        .map_err(|e| format!("Failed to run gh repo clone: {}", e))?;

    if !clone_output.status.success() {
        let stderr = String::from_utf8_lossy(&clone_output.stderr);
        return Err(format!("gh repo clone failed: {}", stderr));
    }

    // Checkout the PR using gh pr checkout
    log::info!("Checking out PR #{}", params.pr_number);
    let checkout_output = Command::new("gh")
        .args(["pr", "checkout", &params.pr_number.to_string()])
        .current_dir(&pr_dir)
        .output()
        .map_err(|e| format!("Failed to run gh pr checkout: {}", e))?;

    if !checkout_output.status.success() {
        let stderr = String::from_utf8_lossy(&checkout_output.stderr);
        return Err(format!("gh pr checkout failed: {}", stderr));
    }

    // Set origin URL to SSH
    let set_url_output = Command::new("git")
        .args(["remote", "set-url", "origin", &params.ssh_url])
        .current_dir(&pr_dir)
        .output();

    if let Err(err) = set_url_output {
        log::warn!("Failed to set SSH origin URL: {}", err);
        // Continue anyway - HTTPS will still work
    }

    Ok(pr_dir)
}
