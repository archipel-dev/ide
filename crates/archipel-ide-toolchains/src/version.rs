//! Tool versions and version requests.
//!
//! Toolchain version strings are not semver (`21.0.4+7`, `1.80.0`, `2024-09-30`,
//! `7.4.0rc1`), so ordering is segment-wise: numeric segments compare
//! numerically, alphanumeric segments lexically, and a trailing alphabetic
//! segment marks a pre-release (`1.0.0-rc1 < 1.0.0`).

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolVersion(String);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment<'a> {
    Number(u64),
    Text(&'a str),
}

impl ToolVersion {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into().trim().trim_start_matches('v').to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn segments(&self) -> Vec<Segment<'_>> {
        split_segments(&self.0)
    }

    /// Whether this version looks like a pre-release (`-rc1`, `-beta`, `ea`).
    pub fn is_prerelease(&self) -> bool {
        self.segments()
            .iter()
            .skip(1)
            .any(|segment| matches!(segment, Segment::Text(_)))
    }

    /// Whether `prefix` matches this version on segment boundaries
    /// (`21` matches `21.0.4+7` but not `210.1`).
    pub fn starts_with_version(&self, prefix: &str) -> bool {
        let prefix = prefix.trim().trim_start_matches('v');
        let mine = self.segments();
        let theirs = split_segments(prefix);
        !theirs.is_empty()
            && theirs.len() <= mine.len()
            && theirs.iter().zip(&mine).all(|(a, b)| a == b)
    }
}

/// Splits on `.`, `-`, `+`, `_` and on digit/letter boundaries.
fn split_segments(text: &str) -> Vec<Segment<'_>> {
    let mut ranges = Vec::new();
    let mut start = None;
    let mut digits = false;
    for (index, byte) in text.bytes().enumerate() {
        let separator = matches!(byte, b'.' | b'-' | b'+' | b'_');
        let is_digit = byte.is_ascii_digit();
        match start {
            Some(begin) if separator || is_digit != digits => {
                ranges.push(begin..index);
                start = (!separator).then_some(index);
            }
            None if !separator => start = Some(index),
            _ => {}
        }
        if !separator {
            digits = is_digit;
        }
    }
    if let Some(begin) = start {
        ranges.push(begin..text.len());
    }
    ranges
        .into_iter()
        .map(|range| {
            let part = &text[range];
            // Numbers too long for u64 compare lexically, which keeps order stable.
            part.parse::<u64>()
                .map_or(Segment::Text(part), Segment::Number)
        })
        .collect()
}

impl Ord for ToolVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        let (a, b) = (self.segments(), other.segments());
        for index in 0..a.len().max(b.len()) {
            let ordering = match (a.get(index), b.get(index)) {
                (Some(Segment::Number(x)), Some(Segment::Number(y))) => x.cmp(y),
                (Some(Segment::Text(x)), Some(Segment::Text(y))) => x.cmp(y),
                (Some(Segment::Number(_)), Some(Segment::Text(_))) => Ordering::Greater,
                (Some(Segment::Text(_)), Some(Segment::Number(_))) => Ordering::Less,
                // A trailing text segment is a pre-release; a trailing number a build.
                (Some(Segment::Text(_)), None) => Ordering::Less,
                (Some(Segment::Number(_)), None) => Ordering::Greater,
                (None, Some(Segment::Text(_))) => Ordering::Greater,
                (None, Some(Segment::Number(_))) => Ordering::Less,
                (None, None) => Ordering::Equal,
            };
            if ordering != Ordering::Equal {
                return ordering;
            }
        }
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for ToolVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for ToolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Which version of a component is wanted.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "kind", content = "value")]
pub enum VersionRequest {
    /// The newest stable release.
    Latest,
    /// A named channel the provider understands (`stable`, `lts`, `nightly`).
    Channel(String),
    /// A version or version prefix (`21`, `21.0.4`, `1.80.0`).
    Version(String),
}

impl VersionRequest {
    /// `""`/`latest` → latest, purely alphabetic → channel, else a version prefix.
    pub fn parse(text: &str) -> VersionRequest {
        let text = text.trim();
        if text.is_empty() || text.eq_ignore_ascii_case("latest") {
            VersionRequest::Latest
        } else if !text
            .trim_start_matches('v')
            .starts_with(|c: char| c.is_ascii_digit())
        {
            VersionRequest::Channel(text.to_ascii_lowercase())
        } else {
            VersionRequest::Version(text.trim_start_matches('v').to_owned())
        }
    }

    /// Whether `version` satisfies the request. Channels are resolved by
    /// providers; here they accept any stable version.
    pub fn matches(&self, version: &ToolVersion) -> bool {
        match self {
            VersionRequest::Latest | VersionRequest::Channel(_) => !version.is_prerelease(),
            VersionRequest::Version(prefix) => version.starts_with_version(prefix),
        }
    }

    /// The highest of `versions` that satisfies the request.
    pub fn best<'a, I>(&self, versions: I) -> Option<&'a ToolVersion>
    where
        I: IntoIterator<Item = &'a ToolVersion>,
    {
        versions.into_iter().filter(|v| self.matches(v)).max()
    }
}

impl fmt::Display for VersionRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionRequest::Latest => f.write_str("latest"),
            VersionRequest::Channel(name) | VersionRequest::Version(name) => f.write_str(name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> ToolVersion {
        ToolVersion::new(text)
    }

    #[test]
    fn orders_numerically_with_prereleases_and_builds() {
        assert!(v("1.10.0") > v("1.9.3"));
        assert!(v("1.0.0") > v("1.0.0-rc1"));
        assert!(v("21.0.4+7") > v("21.0.4"));
        assert!(v("21.0.4+7") < v("21.0.10+1"));
        assert!(v("7.4.0rc1") < v("7.4.0"));
        assert!(v("2024-09-30") > v("2024-09-23"));
        assert_eq!(v("v1.2.3"), v("1.2.3"));
    }

    #[test]
    fn prefix_requests_match_on_segment_boundaries() {
        let request = VersionRequest::parse("21");
        assert!(request.matches(&v("21.0.4+7")));
        assert!(!request.matches(&v("210.1")));
        assert!(!request.matches(&v("17.0.12")));
        let versions = [v("21.0.3"), v("21.0.4+7"), v("17.0.12"), v("22.0.1")];
        assert_eq!(request.best(&versions), Some(&versions[1]));
        assert_eq!(VersionRequest::Latest.best(&versions), Some(&versions[3]));
    }

    #[test]
    fn parses_requests() {
        assert_eq!(VersionRequest::parse(""), VersionRequest::Latest);
        assert_eq!(
            VersionRequest::parse("LTS"),
            VersionRequest::Channel("lts".into())
        );
        assert_eq!(
            VersionRequest::parse("v20.11.1"),
            VersionRequest::Version("20.11.1".into())
        );
        assert!(!VersionRequest::Latest.matches(&v("2.0.0-beta.1")));
    }
}
