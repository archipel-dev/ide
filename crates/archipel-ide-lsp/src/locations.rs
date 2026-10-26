//! Locations returned by navigation requests (`textDocument/definition`,
//! `typeDefinition`, `implementation`, `references`).

use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use url::Url;

/// A range in a file. Lines and characters are LSP UTF-16 positions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspLocation {
    pub path: PathBuf,
    pub start_line: usize,
    pub start_character: usize,
    pub end_line: usize,
    pub end_character: usize,
}

/// Parses `Location | Location[] | LocationLink[] | null`. Links resolve to
/// their `targetSelectionRange` (the name), like editors do. Entries that are
/// not `file:` URIs are skipped. The result is sorted and deduplicated.
pub fn parse_locations(value: &Value) -> Vec<LspLocation> {
    let items: Vec<&Value> = match value {
        Value::Array(items) => items.iter().collect(),
        Value::Object(_) => vec![value],
        _ => Vec::new(),
    };
    let mut locations: Vec<LspLocation> = items.into_iter().filter_map(parse_location).collect();
    locations.sort();
    locations.dedup();
    locations
}

fn parse_location(value: &Value) -> Option<LspLocation> {
    let (uri, range) = match value.get("targetUri") {
        Some(uri) => (
            uri,
            value
                .get("targetSelectionRange")
                .or_else(|| value.get("targetRange"))?,
        ),
        None => (value.get("uri")?, value.get("range")?),
    };
    let url = Url::parse(uri.as_str()?).ok()?;
    if url.scheme() != "file" {
        return None;
    }
    let path = url.to_file_path().ok()?;
    let position = |name: &str, field: &str| -> Option<usize> {
        usize::try_from(range.get(name)?.get(field)?.as_u64()?).ok()
    };
    Some(LspLocation {
        path,
        start_line: position("start", "line")?,
        start_character: position("start", "character")?,
        end_line: position("end", "line")?,
        end_character: position("end", "character")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn range(line: u64) -> Value {
        json!({"start":{"line":line,"character":4},"end":{"line":line,"character":9}})
    }

    #[test]
    fn parses_every_location_shape() {
        let single = json!({"uri":"file:///p/a.rs","range":range(3)});
        assert_eq!(parse_locations(&single).len(), 1);
        assert_eq!(parse_locations(&single)[0].path, PathBuf::from("/p/a.rs"));
        let many = json!([
            {"uri":"file:///p/b.rs","range":range(1)},
            {"uri":"file:///p/a.rs","range":range(3)},
            {"uri":"file:///p/a.rs","range":range(3)},
            {"uri":"jar:/x.class","range":range(0)}
        ]);
        let parsed = parse_locations(&many);
        assert_eq!(parsed.len(), 2, "deduplicated, non-file URIs skipped");
        assert_eq!(parsed[0].path, PathBuf::from("/p/a.rs"));
        let links = json!([{"targetUri":"file:///p/c.rs","targetRange":range(10),"targetSelectionRange":range(11)}]);
        assert_eq!(parse_locations(&links)[0].start_line, 11);
        assert!(parse_locations(&Value::Null).is_empty());
        assert!(parse_locations(&json!([{"uri":"file:///p/a.rs"}])).is_empty());
    }
}
