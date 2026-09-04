//! Discovers the address and API key of the local Syncthing instance.
//!
//! Order of precedence:
//!   1. Environment variables SYNCTHING_URL / SYNCTHING_APIKEY
//!   2. Syncthing's own config.xml (default location per OS)

use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AppConfig {
    /// e.g. "http://127.0.0.1:8384"
    pub base_url: String,
    pub api_key: String,
    pub poll_interval: Duration,
}

pub fn load() -> Result<AppConfig, String> {
    let poll_interval = Duration::from_secs(
        std::env::var("SYNCTHING_POLL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .filter(|s| *s >= 1)
            .unwrap_or(3),
    );

    let env_url = std::env::var("SYNCTHING_URL").ok().filter(|s| !s.is_empty());
    let env_key = std::env::var("SYNCTHING_APIKEY")
        .ok()
        .filter(|s| !s.is_empty());

    if let (Some(url), Some(key)) = (env_url.clone(), env_key.clone()) {
        return Ok(AppConfig {
            base_url: normalize_url(&url),
            api_key: key,
            poll_interval,
        });
    }

    let path = find_config_xml().ok_or_else(|| {
        "Could not find Syncthing's config.xml. Set SYNCTHING_URL and SYNCTHING_APIKEY."
            .to_string()
    })?;
    let xml = std::fs::read_to_string(&path)
        .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    let parsed = parse_config_xml(&xml)
        .ok_or_else(|| format!("No <gui> section found in {}", path.display()))?;

    Ok(AppConfig {
        base_url: env_url.map(|u| normalize_url(&u)).unwrap_or(parsed.base_url),
        api_key: env_key.unwrap_or(parsed.api_key),
        poll_interval,
    })
}

struct ParsedXml {
    base_url: String,
    api_key: String,
}

fn parse_config_xml(xml: &str) -> Option<ParsedXml> {
    let gui_start = xml.find("<gui")?;
    let tag_end = xml[gui_start..].find('>')? + gui_start;
    let gui_attrs = &xml[gui_start..tag_end];
    let gui_end = xml[tag_end..].find("</gui>").map(|i| i + tag_end)?;
    let gui_body = &xml[tag_end..gui_end];

    let tls = gui_attrs.contains("tls=\"true\"");
    let address = tag_value(gui_body, "address").unwrap_or_else(|| "127.0.0.1:8384".into());
    let api_key = tag_value(gui_body, "apikey").unwrap_or_default();

    // "0.0.0.0:8384" / ":8384" make no sense as a client address
    let address = match address.rsplit_once(':') {
        Some((host, port)) if host.is_empty() || host == "0.0.0.0" || host == "[::]" => {
            format!("127.0.0.1:{port}")
        }
        _ => address,
    };

    let scheme = if tls { "https" } else { "http" };
    Some(ParsedXml {
        base_url: format!("{scheme}://{address}"),
        api_key,
    })
}

fn tag_value(body: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = body.find(&open)? + open.len();
    let end = body[start..].find(&close)? + start;
    let v = body[start..end].trim().to_string();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

fn normalize_url(url: &str) -> String {
    let u = url.trim().trim_end_matches('/');
    if u.starts_with("http://") || u.starts_with("https://") {
        u.to_string()
    } else {
        format!("http://{u}")
    }
}

/// Default locations of Syncthing's config.xml.
fn find_config_xml() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(dir) = std::env::var("SYNCTHING_HOME") {
        candidates.push(Path::new(&dir).join("config.xml"));
    }

    #[cfg(windows)]
    {
        // v2 uses LOCALAPPDATA, v1 used APPDATA
        for var in ["LOCALAPPDATA", "APPDATA"] {
            if let Ok(dir) = std::env::var(var) {
                candidates.push(Path::new(&dir).join("Syncthing").join("config.xml"));
            }
        }
    }

    #[cfg(target_os = "macos")]
    if let Ok(home) = std::env::var("HOME") {
        candidates.push(
            Path::new(&home)
                .join("Library/Application Support/Syncthing/config.xml"),
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(dir) = std::env::var("XDG_STATE_HOME") {
            candidates.push(Path::new(&dir).join("syncthing/config.xml"));
        }
        if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
            candidates.push(Path::new(&dir).join("syncthing/config.xml"));
        }
        if let Ok(home) = std::env::var("HOME") {
            candidates.push(Path::new(&home).join(".local/state/syncthing/config.xml"));
            candidates.push(Path::new(&home).join(".config/syncthing/config.xml"));
        }
    }

    candidates.into_iter().find(|p| p.is_file())
}
