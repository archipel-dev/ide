//! Minimal `java.util.Properties` text-format reader (`local.properties`,
//! `source.properties`). Handles comments, `=`/`:`/whitespace separators, line
//! continuations and backslash escapes, which Android Studio writes for Windows
//! paths (`sdk.dir=C\:\\Users\\me\\AppData\\Local\\Android\\Sdk`).

use std::collections::BTreeMap;

pub fn parse(input: &str) -> BTreeMap<String, String> {
    let mut entries = BTreeMap::new();
    let mut logical = String::new();
    for raw in input.lines() {
        let line = raw.trim_start();
        if logical.is_empty() && (line.is_empty() || line.starts_with('#') || line.starts_with('!'))
        {
            continue;
        }
        if ends_with_continuation(line) {
            logical.push_str(&line[..line.len() - 1]);
            continue;
        }
        logical.push_str(line);
        let (key, value) = split(&logical);
        entries.insert(unescape(key), unescape(value));
        logical.clear();
    }
    if !logical.is_empty() {
        let (key, value) = split(&logical);
        entries.insert(unescape(key), unescape(value));
    }
    entries
}

/// Escapes a properties value, e.g. `C:\Sdk` becomes `C\:\\Sdk`.
pub fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' | ':' | '=' | '#' | '!' => {
                out.push('\\');
                out.push(ch);
            }
            '\n' => out.push_str("\\n"),
            _ => out.push(ch),
        }
    }
    out
}

/// Replaces (or appends) the `key` entry in properties `text`, keeping every
/// other line untouched.
pub fn set(text: &str, key: &str, value: &str) -> String {
    let entry = format!("{}={}", key, escape(value));
    let mut replaced = false;
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let is_key = !trimmed.starts_with(['#', '!'])
            && unescape(split(trimmed).0) == key
            && !ends_with_continuation(trimmed);
        if is_key {
            if !replaced {
                lines.push(entry.clone());
                replaced = true;
            }
        } else {
            lines.push(line.to_owned());
        }
    }
    if !replaced {
        lines.push(entry);
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// A line continues when it ends with an odd number of backslashes.
fn ends_with_continuation(line: &str) -> bool {
    line.bytes().rev().take_while(|b| *b == b'\\').count() % 2 == 1
}

fn split(line: &str) -> (&str, &str) {
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '=' | ':' | ' ' | '\t' => {
                let key = &line[..index];
                let mut rest = line[index..].trim_start_matches([' ', '\t']);
                if rest.starts_with(['=', ':']) {
                    rest = &rest[1..];
                }
                return (key, rest.trim_start_matches([' ', '\t']));
            }
            _ => {}
        }
    }
    (line, "")
}

fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('f') => out.push('\u{c}'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    Some(decoded) if hex.len() == 4 => out.push(decoded),
                    _ => {
                        out.push_str("\\u");
                        out.push_str(&hex);
                    }
                }
            }
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_studio_written_local_properties() {
        let parsed = parse(
            "## This file must *NOT* be checked into Version Control Systems,\n\
             # as it contains information specific to your local configuration.\n\
             sdk.dir=C\\:\\\\Users\\\\me\\\\AppData\\\\Local\\\\Android\\\\Sdk\n\
             ndk.dir = /opt/ndk\n\
             key\\ with\\ space : value\n\
             multi=one, \\\n    two\n\
             unicode=\\u00e9t\\u00e9\n",
        );
        assert_eq!(
            parsed["sdk.dir"],
            "C:\\Users\\me\\AppData\\Local\\Android\\Sdk"
        );
        assert_eq!(parsed["ndk.dir"], "/opt/ndk");
        assert_eq!(parsed["key with space"], "value");
        assert_eq!(parsed["multi"], "one, two");
        assert_eq!(parsed["unicode"], "été");
    }

    #[test]
    fn set_round_trips_windows_paths() {
        let original = "# comment\nsdk.dir=/old\nndk.dir=/ndk\n";
        let updated = set(original, "sdk.dir", "C:\\Android\\Sdk");
        assert_eq!(
            updated,
            "# comment\nsdk.dir=C\\:\\\\Android\\\\Sdk\nndk.dir=/ndk\n"
        );
        assert_eq!(parse(&updated)["sdk.dir"], "C:\\Android\\Sdk");
        assert_eq!(set("", "sdk.dir", "/a"), "sdk.dir=/a\n");
    }

    #[test]
    fn tolerates_malformed_input() {
        let parsed = parse("novalue\n=\nbad=\\u12\ntrailing=\\");
        assert_eq!(parsed["novalue"], "");
        assert_eq!(parsed["bad"], "\\u12");
        assert_eq!(parsed["trailing"], "");
    }
}
