//! What choosing an item writes (JAVA-CMP-019..021, 035): keys that carry the row's meaning from
//! the list to the insertion, computed against the text as it is when the row is chosen.

use super::{CompletionInput, Insertion};
use crate::index::NotReady;

/// Starts the key of every row that is not a class (classes use their FQN).
const MARK: char = '\u{1}';

/// A row that inserts `text` as it is (a variable, a field, a keyword).
pub fn plain_key(text: &str) -> String {
    format!("{MARK}n{text}")
}

/// A method row: `name(` with the caret inside when some overload takes arguments, `name()`
/// otherwise (JAVA-CMP-035).
pub fn call_key(name: &str, with_args: bool) -> String {
    format!("{MARK}{}{name}", if with_args { 'c' } else { 'z' })
}

/// The identifier characters after the caret and what follows them.
fn after_identifier(input: &CompletionInput<'_>) -> bool {
    let rest = &input.text[input.offset.min(input.text.len())..];
    let tail: usize = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '$'))
        .map(char::len_utf8)
        .sum();
    rest[tail..]
        .trim_start_matches([' ', '\t'])
        .starts_with('(')
}

pub(super) fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('$', "\\$")
        .replace('}', "\\}")
}

pub fn insertion(input: &CompletionInput<'_>, key: &str) -> Result<Option<Insertion>, NotReady> {
    let Some(rest) = key.strip_prefix(MARK) else {
        return super::class_names::insertion(input, key);
    };
    let mut chars = rest.chars();
    let (Some(tag), text) = (chars.next(), chars.as_str()) else {
        return Ok(None);
    };
    Ok(Some(match tag {
        'n' => Insertion {
            text: text.to_string(),
            edits: Vec::new(),
            snippet: false,
        },
        // An existing call (`foo.ba|r(1)`) keeps its parentheses.
        'c' | 'z' if after_identifier(input) => Insertion {
            text: text.to_string(),
            edits: Vec::new(),
            snippet: false,
        },
        'c' => Insertion {
            text: format!("{}($0)", escape(text)),
            edits: Vec::new(),
            snippet: true,
        },
        'z' => Insertion {
            text: format!("{text}()"),
            edits: Vec::new(),
            snippet: false,
        },
        's' => return super::statics::insertion(input, text),
        _ => return Ok(None),
    }))
}
