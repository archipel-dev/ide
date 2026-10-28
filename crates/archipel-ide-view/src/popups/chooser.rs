//! The chooser of several navigation targets (Go to Declaration or Implementation with more
//! than one target, Show Usages): a title with the result count and one row per target with
//! the file, the code line with the symbol highlighted and the location (M17).

use archipel_ide_core::{NavigationItem, NavigationList};
use archipel_kit_model::IconKey;

/// The header of the popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChooserHeader {
    /// "Choose Declaration of foo".
    pub title: String,
    /// "3 results", "100+ results".
    pub count: String,
}

/// One row: the code line cut into the part before the symbol, the symbol and the rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceRow {
    /// The file icon.
    pub icon: IconKey,
    /// The code before the symbol (leading blanks dropped).
    pub before: String,
    /// The symbol (highlighted).
    pub symbol: String,
    /// The code after it.
    pub after: String,
    /// "File.java 12" at the right edge.
    pub location: String,
}

/// Plural text: `1 result`, `2 results`.
fn count_text(count: usize, truncated: bool) -> String {
    let noun = if count == 1 { "result" } else { "results" };
    format!("{count} {noun}{}", if truncated { "+" } else { "" })
}

/// The header of `list`.
pub fn header(list: &NavigationList) -> ChooserHeader {
    ChooserHeader {
        title: list.title.clone(),
        count: count_text(list.items.len(), list.truncated),
    }
}

/// The file name of a path.
fn file_name(item: &NavigationItem) -> String {
    item.path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| item.path.display().to_string())
}

/// The row of `item`.
pub fn row(item: &NavigationItem) -> ChoiceRow {
    let name = file_name(item);
    let code = item.line_text.trim_start();
    let trimmed = item.line_text.chars().count() - code.chars().count();
    let start = item.column.saturating_sub(trimmed);
    let before: String = code.chars().take(start).collect();
    let symbol: String = code
        .chars()
        .skip(start)
        .take(item.end_column.saturating_sub(item.column))
        .collect();
    let after: String = code.chars().skip(start + symbol.chars().count()).collect();
    ChoiceRow {
        icon: IconKey::File(name.clone()),
        before,
        symbol,
        after,
        location: format!("{name} {}", item.line + 1),
    }
}

/// The rows of `list`.
pub fn rows(list: &NavigationList) -> Vec<ChoiceRow> {
    list.items.iter().map(row).collect()
}

/// The command and parameters that open `item` (`navigate.goto`, 0-based line and column).
pub fn goto(item: &NavigationItem) -> (&'static str, serde_json::Value) {
    (
        "navigate.goto",
        serde_json::json!({"path": item.path, "line": item.line, "column": item.column}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_core::NavigationKind;
    use std::path::PathBuf;

    fn item() -> NavigationItem {
        NavigationItem {
            path: PathBuf::from("src/App.java"),
            external: false,
            line: 11,
            column: 11,
            end_column: 16,
            line_text: "    return greet(name);".into(),
        }
    }

    #[test]
    fn a_row_splits_the_code_around_the_symbol() {
        let row = row(&item());
        assert_eq!(row.before, "return ");
        assert_eq!(row.symbol, "greet");
        assert_eq!(row.after, "(name);");
        assert_eq!(row.location, "App.java 12");
        assert_eq!(row.icon, IconKey::File("App.java".into()));
    }

    #[test]
    fn the_header_counts_results() {
        let mut list = NavigationList {
            kind: NavigationKind::Implementation,
            title: "Choose Implementation of greet".into(),
            items: vec![item(), item()],
            truncated: false,
        };
        assert_eq!(header(&list).count, "2 results");
        list.truncated = true;
        assert_eq!(header(&list).count, "2 results+");
        list.items.truncate(1);
        assert_eq!(header(&list).count, "1 result+");
        assert_eq!(goto(&item()).0, "navigate.goto");
    }
}
