use serde::Deserialize;
use console::style;
use std::fs;
use std::io::{self, Write};
use anyhow::Result;
use std::process::Command;

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

pub struct UpdateInfo {
    pub version: String,
    pub nixboost_url: Option<String>,
}

pub fn check_for_updates(current_version: &str) -> Option<UpdateInfo> {
    let url = "https://api.github.com/repos/NacreousDawn596/nixboost/releases/latest";
    
    let response = ureq::get(url)
        .set("User-Agent", "nixboost-updater")
        .timeout(std::time::Duration::from_secs(2))
        .call();

    if let Ok(res) = response {
        if let Ok(release) = res.into_json::<GithubRelease>() {
            let latest = release.tag_name.trim_start_matches('v');
            if latest != current_version {
                let mut info = UpdateInfo {
                    version: latest.to_string(),
                    nixboost_url: None,
                };
                for asset in release.assets {
                    if asset.name == "nixboost" {
                        info.nixboost_url = Some(asset.browser_download_url);
                    }
                }
                return Some(info);
            }
        }
    }
    None
}

pub fn perform_update(_info: UpdateInfo) -> Result<()> {
    println!("{}", style(":: starting automatic update...").bold().cyan());

    let status = Command::new("nix")
        .arg("profile")
        .arg("install")
        .arg("github:NacreousDawn596/nixboost")
        .status()?;

    if !status.success() {
        anyhow::bail!("nix profile install failed");
    }

    println!("{}", style(":: update completed successfully.").green().bold());
    Ok(())
}
