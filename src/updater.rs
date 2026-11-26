use crate::quote::build_agent;
use serde_json::Value;

const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/tgsastudio/simple_stock/releases/latest";

#[derive(Debug, Clone)]
pub struct UpdateCheckResult {
    pub latest_version: String,
    pub download_url: String,
    pub has_update: bool,
}

fn parse_semver(s: &str) -> (u32, u32, u32) {
    let clean = s.trim().trim_start_matches(['v', 'V']);
    let mut parts = clean.split('.').map(|p| {
        p.chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<u32>()
            .unwrap_or(0)
    });
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    let patch = parts.next().unwrap_or(0);
    (major, minor, patch)
}

pub fn check_for_updates(current_version: &str) -> Result<UpdateCheckResult, String> {
    let agent = build_agent();
    let resp = match agent
        .get(GITHUB_LATEST_RELEASE_URL)
        .set("Accept", "application/vnd.github+json")
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => {
            return Ok(UpdateCheckResult {
                latest_version: current_version.to_string(),
                download_url: "https://github.com/tgsastudio/simple_stock/releases".to_string(),
                has_update: false,
            });
        }
        Err(e) => return Err(format!("{e}")),
    };

    let body: Value = resp.into_json().map_err(|e| format!("{e}"))?;
    let tag = body
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or(current_version);
    let latest_clean = tag.trim().trim_start_matches(['v', 'V']).to_string();

    let mut download_url = body
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://github.com/tgsastudio/simple_stock/releases")
        .to_string();

    if let Some(assets) = body.get("assets").and_then(|v| v.as_array()) {
        for asset in assets {
            if let (Some(name), Some(url)) = (
                asset.get("name").and_then(|v| v.as_str()),
                asset.get("browser_download_url").and_then(|v| v.as_str()),
            ) {
                if name.to_ascii_lowercase().ends_with(".exe") {
                    download_url = url.to_string();
                    break;
                }
            }
        }
    }

    let has_update = parse_semver(&latest_clean) > parse_semver(current_version);
    Ok(UpdateCheckResult {
        latest_version: latest_clean,
        download_url,
        has_update,
    })
}
