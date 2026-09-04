//! Minimal client for the Syncthing REST API.

use std::time::{Duration, Instant};

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    Ok,
    Syncing,
    Scanning,
    Paused,
    Error,
    Offline,
}

impl State {
    pub fn label(self) -> &'static str {
        match self {
            State::Ok => "Up to date",
            State::Syncing => "Syncing",
            State::Scanning => "Scanning",
            State::Paused => "Paused",
            State::Error => "Error",
            State::Offline => "Disconnected",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FolderStatus {
    pub id: String,
    pub label: String,
    pub paused: bool,
    /// "idle", "syncing", "scanning", "error", ...
    pub state: String,
    /// 0.0 - 100.0 (local completion)
    pub completion: f64,
    pub need_bytes: u64,
    pub need_items: u64,
    pub error: Option<String>,
}

impl FolderStatus {
    pub fn display_name(&self) -> &str {
        if self.label.is_empty() {
            &self.id
        } else {
            &self.label
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    pub online: bool,
    pub version: String,
    pub folders: Vec<FolderStatus>,
    pub devices_connected: usize,
    pub devices_total: usize,
    /// bytes/s
    pub in_rate: f64,
    pub out_rate: f64,
    pub errors: Vec<String>,
    /// Error message from the connection itself
    pub conn_error: Option<String>,
}

impl Default for Status {
    fn default() -> Self {
        Status {
            online: false,
            version: String::new(),
            folders: Vec::new(),
            devices_connected: 0,
            devices_total: 0,
            in_rate: 0.0,
            out_rate: 0.0,
            errors: Vec::new(),
            conn_error: None,
        }
    }
}

impl Status {
    pub fn state(&self) -> State {
        if !self.online {
            return State::Offline;
        }
        if !self.errors.is_empty() || self.folders.iter().any(|f| f.error.is_some() || f.state == "error") {
            return State::Error;
        }
        if self
            .folders
            .iter()
            .any(|f| !f.paused && (f.state.starts_with("sync") || f.need_items > 0))
        {
            return State::Syncing;
        }
        if self.folders.iter().any(|f| !f.paused && f.state == "scanning") {
            return State::Scanning;
        }
        if !self.folders.is_empty() && self.folders.iter().all(|f| f.paused) {
            return State::Paused;
        }
        State::Ok
    }

    /// Short summary for the tooltip
    pub fn summary(&self) -> String {
        if !self.online {
            let mut s = String::from("Syncthing: not connected");
            if let Some(e) = &self.conn_error {
                s.push_str("\n");
                s.push_str(e);
            }
            return s;
        }
        let st = self.state();
        let mut s = format!("Syncthing {} — {}", self.version, st.label());
        s.push_str(&format!(
            "\n{} of {} devices connected",
            self.devices_connected, self.devices_total
        ));
        if st == State::Syncing {
            let need_bytes: u64 = self.folders.iter().map(|f| f.need_bytes).sum();
            let need_items: u64 = self.folders.iter().map(|f| f.need_items).sum();
            s.push_str(&format!(
                "\nRemaining: {} ({} files)",
                fmt_bytes(need_bytes as f64),
                need_items
            ));
        }
        if self.in_rate > 1024.0 || self.out_rate > 1024.0 {
            s.push_str(&format!(
                "\n↓ {}/s  ↑ {}/s",
                fmt_bytes(self.in_rate),
                fmt_bytes(self.out_rate)
            ));
        }
        if let Some(err) = self.errors.first() {
            s.push_str(&format!("\nError: {err}"));
        }
        // The Windows tooltip is limited to 127 characters
        truncate(&s, 126)
    }
}

pub fn fmt_bytes(b: f64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = b;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{v:.0} {}", UNITS[i])
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}

// ---------- API models ----------

#[derive(Deserialize)]
struct VersionResp {
    version: String,
}

#[derive(Deserialize)]
struct FolderCfg {
    id: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    paused: bool,
}

#[derive(Deserialize)]
struct DeviceCfg {
    #[serde(rename = "deviceID")]
    device_id: String,
}

#[derive(Deserialize)]
struct DbStatus {
    #[serde(default)]
    state: String,
    #[serde(default, rename = "globalBytes")]
    global_bytes: u64,
    #[serde(default, rename = "needBytes")]
    need_bytes: u64,
    #[serde(default, rename = "needTotalItems")]
    need_total_items: u64,
    #[serde(default)]
    error: String,
}

#[derive(Deserialize)]
struct ConnectionsResp {
    #[serde(default)]
    connections: std::collections::HashMap<String, ConnectionEntry>,
    #[serde(default)]
    total: ConnTotals,
}

#[derive(Deserialize, Default)]
struct ConnectionEntry {
    #[serde(default)]
    connected: bool,
}

#[derive(Deserialize, Default)]
struct ConnTotals {
    #[serde(default, rename = "inBytesTotal")]
    in_bytes_total: u64,
    #[serde(default, rename = "outBytesTotal")]
    out_bytes_total: u64,
}

#[derive(Deserialize)]
struct ErrorsResp {
    #[serde(default)]
    errors: Vec<ErrorEntry>,
}

#[derive(Deserialize)]
struct ErrorEntry {
    #[serde(default)]
    message: String,
}

// ---------- Client ----------

pub struct Client {
    http: reqwest::blocking::Client,
    base_url: String,
    api_key: String,
    last_sample: Option<(Instant, u64, u64)>,
    my_id: Option<String>,
}

impl Client {
    pub fn new(base_url: String, api_key: String) -> Result<Self, String> {
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(8))
            // Syncthing uses a self-signed certificate for https
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Client {
            http,
            base_url,
            api_key,
            last_sample: None,
            my_id: None,
        })
    }

    fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, String> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .map_err(|e| short_err(&e.to_string()))?;
        if !resp.status().is_success() {
            return Err(format!("{} returned {}", path, resp.status().as_u16()));
        }
        resp.json::<T>().map_err(|e| format!("{path}: {e}"))
    }

    pub fn post(&self, path: &str) -> Result<(), String> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .map_err(|e| short_err(&e.to_string()))?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!("{} returned {}", path, resp.status().as_u16()))
        }
    }

    pub fn fetch(&mut self) -> Status {
        match self.fetch_inner() {
            Ok(s) => s,
            Err(e) => {
                self.last_sample = None;
                Status {
                    conn_error: Some(e),
                    ..Default::default()
                }
            }
        }
    }

    fn fetch_inner(&mut self) -> Result<Status, String> {
        let version: VersionResp = self.get("/rest/system/version")?;
        let folder_cfgs: Vec<FolderCfg> = self.get("/rest/config/folders")?;
        let device_cfgs: Vec<DeviceCfg> = self.get("/rest/config/devices")?;

        if self.my_id.is_none() {
            #[derive(Deserialize)]
            struct SysStatus {
                #[serde(rename = "myID")]
                my_id: String,
            }
            if let Ok(s) = self.get::<SysStatus>("/rest/system/status") {
                self.my_id = Some(s.my_id);
            }
        }

        let mut folders = Vec::with_capacity(folder_cfgs.len());
        for f in folder_cfgs {
            let (state, completion, need_bytes, need_items, error) = if f.paused {
                ("paused".to_string(), 100.0, 0, 0, None)
            } else {
                match self.get::<DbStatus>(&format!("/rest/db/status?folder={}", urlencode(&f.id)))
                {
                    Ok(s) => {
                        let completion = if s.global_bytes == 0 {
                            100.0
                        } else {
                            100.0 * (s.global_bytes.saturating_sub(s.need_bytes)) as f64
                                / s.global_bytes as f64
                        };
                        let err = if s.error.is_empty() {
                            None
                        } else {
                            Some(s.error)
                        };
                        (s.state, completion, s.need_bytes, s.need_total_items, err)
                    }
                    Err(e) => ("error".to_string(), 0.0, 0, 0, Some(e)),
                }
            };
            folders.push(FolderStatus {
                id: f.id,
                label: f.label,
                paused: f.paused,
                state,
                completion,
                need_bytes,
                need_items,
                error,
            });
        }

        // Devices other than ourselves
        let devices_total = device_cfgs
            .iter()
            .filter(|d| Some(&d.device_id) != self.my_id.as_ref())
            .count();

        let conns: ConnectionsResp = self.get("/rest/system/connections")?;
        let devices_connected = conns.connections.values().filter(|c| c.connected).count();

        let now = Instant::now();
        let (in_rate, out_rate) = match self.last_sample {
            Some((t, prev_in, prev_out)) => {
                let dt = now.duration_since(t).as_secs_f64();
                if dt > 0.2 {
                    (
                        conns.total.in_bytes_total.saturating_sub(prev_in) as f64 / dt,
                        conns.total.out_bytes_total.saturating_sub(prev_out) as f64 / dt,
                    )
                } else {
                    (0.0, 0.0)
                }
            }
            None => (0.0, 0.0),
        };
        self.last_sample = Some((
            now,
            conns.total.in_bytes_total,
            conns.total.out_bytes_total,
        ));

        let errors = self
            .get::<ErrorsResp>("/rest/system/error")
            .map(|e| e.errors.into_iter().map(|e| e.message).collect())
            .unwrap_or_default();

        Ok(Status {
            online: true,
            version: version.version,
            folders,
            devices_connected,
            devices_total,
            in_rate,
            out_rate,
            errors,
            conn_error: None,
        })
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn short_err(e: &str) -> String {
    match e.split_once(':') {
        Some((head, _)) if e.len() > 90 => head.to_string(),
        _ => e.to_string(),
    }
}
