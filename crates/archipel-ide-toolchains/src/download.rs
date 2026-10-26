//! Proxy-aware HTTP with resumable, SHA-256 verified downloads.

use crate::ToolchainError;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Upper bound for a single download (Android system images are ~1.5 GB).
const MAX_DOWNLOAD: u64 = 8 * 1024 * 1024 * 1024;
/// Upper bound for metadata responses (catalog indexes, checksum files).
const MAX_TEXT: u64 = 32 * 1024 * 1024;

/// Bytes received so far and the total size when the server reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadProgress {
    pub received: u64,
    pub total: Option<u64>,
}

/// HTTP client honouring `HTTPS_PROXY`/`HTTP_PROXY`/`ALL_PROXY` and `NO_PROXY`
/// (ureq alone ignores `NO_PROXY`) and the system trust store, so corporate
/// proxies and internal mirrors work like in the reference IDE.
#[derive(Clone)]
pub struct HttpClient {
    direct: ureq::Agent,
    https: Option<ureq::Agent>,
    http: Option<ureq::Agent>,
    no_proxy: NoProxy,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

fn env_var(names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| std::env::var(name).ok().filter(|v| !v.trim().is_empty()))
}

fn agent(proxy: Option<&str>) -> Option<ureq::Agent> {
    // Proxies are chosen per URL here (ureq's own env lookup ignores NO_PROXY).
    let mut builder = ureq::AgentBuilder::new()
        .try_proxy_from_env(false)
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(Duration::from_secs(120))
        .user_agent(concat!("archipel-ide/", env!("CARGO_PKG_VERSION")));
    if let Some(proxy) = proxy {
        builder = builder.proxy(ureq::Proxy::new(proxy).ok()?);
    }
    Some(builder.build())
}

impl HttpClient {
    /// Proxy settings from the environment.
    pub fn new() -> Self {
        let all = env_var(&["ALL_PROXY", "all_proxy"]);
        let https = env_var(&["HTTPS_PROXY", "https_proxy"]).or_else(|| all.clone());
        let http = env_var(&["HTTP_PROXY", "http_proxy"]).or(all);
        Self::with_proxies(
            https.as_deref(),
            http.as_deref(),
            &env_var(&["NO_PROXY", "no_proxy"]).unwrap_or_default(),
        )
    }

    /// Explicit proxies (`http://host:port`) and a `NO_PROXY`-style list.
    pub fn with_proxies(https: Option<&str>, http: Option<&str>, no_proxy: &str) -> Self {
        Self {
            direct: agent(None).expect("agent without proxy"),
            https: https.and_then(|proxy| agent(Some(proxy))),
            http: http.and_then(|proxy| agent(Some(proxy))),
            no_proxy: NoProxy::parse(no_proxy),
        }
    }

    fn agent_for(&self, url: &str) -> &ureq::Agent {
        let proxied = if url.starts_with("https://") {
            self.https.as_ref()
        } else {
            self.http.as_ref()
        };
        match proxied {
            Some(agent) if !self.no_proxy.matches(&host_of(url)) => agent,
            _ => &self.direct,
        }
    }

    fn get(&self, url: &str) -> ureq::Request {
        let mut request = self.agent_for(url).get(url);
        // Authenticated GitHub API calls get a far higher rate limit.
        if url.starts_with("https://api.github.com/")
            && let Some(token) = ["ARCHIPEL_GITHUB_TOKEN", "GITHUB_TOKEN"]
                .iter()
                .find_map(|name| {
                    archipel_ide_compat::env_var(name)
                        .ok()
                        .filter(|v| !v.is_empty())
                })
        {
            request = request.set("Authorization", &format!("Bearer {token}"));
        }
        request
    }

    pub fn get_text(&self, url: &str) -> Result<String, ToolchainError> {
        let response = self.get(url).call().map_err(|e| map_error(url, e))?;
        let mut text = String::new();
        response
            .into_reader()
            .take(MAX_TEXT)
            .read_to_string(&mut text)
            .map_err(|error| network(url, error))?;
        Ok(text)
    }

    pub fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T, ToolchainError> {
        let text = self.get_text(url)?;
        serde_json::from_str(&text).map_err(|error| ToolchainError::Catalog {
            source_name: url.into(),
            detail: error.to_string(),
        })
    }

    /// Downloads `url` into `partial`, resuming an earlier partial download with
    /// a `Range` request when the server supports it, then verifies the whole
    /// file against `expected_sha256`. A mismatching file is deleted. On
    /// cancellation the partial file is kept so the next attempt resumes.
    pub fn download(
        &self,
        url: &str,
        partial: &Path,
        expected_sha256: &str,
        progress: &mut dyn FnMut(DownloadProgress),
        cancel: &AtomicBool,
    ) -> Result<(), ToolchainError> {
        let expected = expected_sha256.to_ascii_lowercase();
        if let Some(parent) = partial.parent() {
            fs::create_dir_all(parent)?;
        }
        for attempt in 0..2 {
            let existing = fs::metadata(partial).map(|m| m.len()).unwrap_or(0);
            let mut request = self.get(url);
            if existing > 0 {
                request = request.set("Range", &format!("bytes={existing}-"));
            }
            let response = match request.call() {
                Ok(response) => response,
                // The partial file already holds everything (or is bogus).
                Err(ureq::Error::Status(416, _)) if existing > 0 => {
                    if sha256_file(partial)? == expected {
                        return Ok(());
                    }
                    fs::remove_file(partial)?;
                    continue;
                }
                Err(error) => return Err(map_error(url, error)),
            };
            let resumed = existing > 0
                && response.status() == 206
                && content_range_start(response.header("Content-Range")) == Some(existing);
            let length = response
                .header("Content-Length")
                .and_then(|value| value.parse::<u64>().ok());
            let total = length.map(|length| if resumed { existing + length } else { length });
            let mut file = if resumed {
                OpenOptions::new().append(true).open(partial)?
            } else {
                File::create(partial)?
            };
            let mut received = if resumed { existing } else { 0 };
            progress(DownloadProgress { received, total });
            let mut reader = response.into_reader().take(MAX_DOWNLOAD);
            let mut buffer = vec![0u8; 64 * 1024];
            loop {
                if cancel.load(Ordering::Relaxed) {
                    file.flush()?;
                    return Err(ToolchainError::Cancelled);
                }
                let read = reader
                    .read(&mut buffer)
                    .map_err(|error| network(url, error))?;
                if read == 0 {
                    break;
                }
                file.write_all(&buffer[..read])?;
                received += read as u64;
                progress(DownloadProgress { received, total });
            }
            file.flush()?;
            drop(file);
            if let Some(total) = total
                && received < total
            {
                return Err(ToolchainError::Network {
                    url: url.into(),
                    detail: format!("connection closed after {received} of {total} bytes"),
                });
            }
            let actual = sha256_file(partial)?;
            if actual == expected {
                return Ok(());
            }
            fs::remove_file(partial)?;
            // A resumed download may have been appended to a stale partial
            // file; retry once from scratch before reporting a mismatch.
            if resumed && attempt == 0 {
                continue;
            }
            return Err(ToolchainError::Checksum {
                file: crate::model::file_name_from_url(url),
                expected,
                actual,
            });
        }
        Err(ToolchainError::Network {
            url: url.into(),
            detail: "download could not be completed".into(),
        })
    }
}

fn content_range_start(header: Option<&str>) -> Option<u64> {
    // `bytes 100-999/1000`
    let range = header?.strip_prefix("bytes ")?;
    range.split('-').next()?.trim().parse().ok()
}

fn network(url: &str, error: impl ToString) -> ToolchainError {
    ToolchainError::Network {
        url: url.into(),
        detail: error.to_string(),
    }
}

fn map_error(url: &str, error: ureq::Error) -> ToolchainError {
    match error {
        ureq::Error::Status(status, _) => ToolchainError::Http {
            url: url.into(),
            status,
        },
        ureq::Error::Transport(transport) => {
            use ureq::ErrorKind::*;
            match transport.kind() {
                Dns | ConnectionFailed | ProxyConnect | ProxyUnauthorized => {
                    ToolchainError::Offline {
                        host: host_of(url),
                        detail: transport.to_string(),
                    }
                }
                _ => network(url, transport),
            }
        }
    }
}

/// `host[:port]` of a URL.
fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    authority.rsplit('@').next().unwrap_or(authority).to_owned()
}

/// `NO_PROXY` entries: `*`, host names (`example.com` also matches
/// subdomains, as do `.example.com` and `*.example.com`), IPv4 addresses and
/// IPv4 CIDR ranges (`10.0.0.0/8`). Ports in entries are ignored.
#[derive(Debug, Clone, Default)]
struct NoProxy {
    all: bool,
    hosts: Vec<String>,
    networks: Vec<(u32, u32)>,
}

impl NoProxy {
    fn parse(list: &str) -> NoProxy {
        let mut no_proxy = NoProxy::default();
        for entry in list.split(',').map(str::trim).filter(|e| !e.is_empty()) {
            if entry == "*" {
                no_proxy.all = true;
            } else if let Some((ip, bits)) = entry.split_once('/') {
                if let (Ok(ip), Ok(bits)) = (ip.parse::<std::net::Ipv4Addr>(), bits.parse::<u32>())
                    && bits <= 32
                {
                    let mask = if bits == 0 {
                        0
                    } else {
                        u32::MAX << (32 - bits)
                    };
                    no_proxy.networks.push((u32::from(ip) & mask, mask));
                }
            } else {
                let host = entry.trim_start_matches("*.").trim_start_matches('.');
                no_proxy.hosts.push(host.to_ascii_lowercase());
            }
        }
        no_proxy
    }

    fn matches(&self, authority: &str) -> bool {
        if self.all {
            return true;
        }
        let host = if let Some(rest) = authority.strip_prefix('[') {
            rest.split(']').next().unwrap_or(rest)
        } else {
            authority.split(':').next().unwrap_or(authority)
        }
        .to_ascii_lowercase();
        if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
            let ip = u32::from(ip);
            if self.networks.iter().any(|(net, mask)| ip & mask == *net) {
                return true;
            }
        }
        self.hosts.iter().any(|entry| {
            // `host:port` entries have one colon; IPv6 addresses have several.
            let entry = match entry.matches(':').count() {
                1 => entry.split(':').next().unwrap_or(entry),
                _ => entry.trim_matches(['[', ']']),
            };
            host == entry
                || host
                    .strip_suffix(entry)
                    .is_some_and(|prefix| prefix.ends_with('.'))
        })
    }
}

/// Lower-case hex SHA-256 of a file, streamed.
pub fn sha256_file(path: &Path) -> Result<String, ToolchainError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn is_digest(text: &str) -> bool {
    text.len() == 64 && text.chars().all(|c| c.is_ascii_hexdigit())
}

/// Reads a `sha256sum`-style file: a bare digest, `<hex>  <name>` or several
/// such lines. With file names present, only the line naming `file_name`
/// counts; a single unnamed digest applies to the file it accompanies.
pub fn parse_checksum_file(text: &str, file_name: &str) -> Option<String> {
    let wanted = Path::new(file_name).file_name();
    let mut unnamed = Vec::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let Some(digest) = parts.next().filter(|d| is_digest(d)) else {
            continue;
        };
        let digest = digest.to_ascii_lowercase();
        match parts.next().map(|name| name.trim_start_matches('*')) {
            Some(name) if Path::new(name).file_name() == wanted => return Some(digest),
            Some(_) => {}
            None => unnamed.push(digest),
        }
    }
    (unnamed.len() == 1).then(|| unnamed.remove(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_checksum_files() {
        let a = "a".repeat(64);
        let b = "B".repeat(64);
        assert_eq!(parse_checksum_file(&a, "x.zip"), Some(a.clone()));
        let listing = format!("{a}  other.zip\n{b} *dist/x.zip\n");
        assert_eq!(
            parse_checksum_file(&listing, "x.zip"),
            Some(b.to_ascii_lowercase())
        );
        // A listing without our file must not fall back to another file's digest.
        assert_eq!(
            parse_checksum_file(&format!("{a}  other.zip"), "x.zip"),
            None
        );
        assert_eq!(parse_checksum_file("nope", "x.zip"), None);
    }

    #[test]
    fn parses_content_range_and_hosts() {
        assert_eq!(content_range_start(Some("bytes 100-999/1000")), Some(100));
        assert_eq!(content_range_start(Some("items 1-2")), None);
        assert_eq!(host_of("https://api.foojay.io/disco/v3.0"), "api.foojay.io");
        assert_eq!(host_of("http://user:pw@127.0.0.1:8080/x"), "127.0.0.1:8080");
    }

    #[test]
    fn no_proxy_matches_hosts_suffixes_and_networks() {
        let list = NoProxy::parse("localhost,.corp.example, *.svc.local,10.0.0.0/8,::1");
        assert!(list.matches("localhost:8080"));
        assert!(list.matches("mirror.corp.example"));
        assert!(list.matches("corp.example"));
        assert!(!list.matches("notcorp.example"));
        assert!(list.matches("a.svc.local"));
        assert!(list.matches("10.20.30.40:443"));
        assert!(!list.matches("11.0.0.1"));
        assert!(list.matches("[::1]:80"));
        assert!(!list.matches("api.foojay.io"));
        assert!(NoProxy::parse("*").matches("anything"));
    }
}
