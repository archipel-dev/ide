//! Parity inventory parser and weighted dashboard generator.
//!
//! Every `docs/parity/*.md` file (except the generated `dashboard.md`) is an *area*.
//! Each area contains one or more Markdown tables whose header row names at least an
//! `ID` and a `Status` column. Columns are located by header name, so the classic
//! 7-column format (`ID | Reference | Feature | Status | Commands | Tests |
//! Limitations / evidence`), the 8-column format that adds `Weight`, and older
//! variants without a `Reference` column are all accepted.
//!
//! Scoring:
//! - a missing `Weight` column or empty weight cell counts as weight 1;
//! - `% implemented` = weight of `Implemented` + `Verified` rows, plus half the weight
//!   of `Partial` rows, divided by the total weight;
//! - `% verified` = weight of `Verified` rows divided by the total weight;
//! - rows tagged `[Ultimate]` (in any cell) are IntelliJ IDEA Ultimate-only features
//!   and are additionally reported in a separate edition breakdown.

use std::{
    collections::BTreeMap,
    fmt, fs,
    path::{Path, PathBuf},
};

pub const DASHBOARD_FILE: &str = "dashboard.md";
/// Files in `docs/parity/` that are not areas: the generated dashboard and the format guide.
pub const NON_AREA_FILES: [&str; 2] = [DASHBOARD_FILE, "README.md"];
pub const README_BEGIN: &str = "<!-- parity:begin -->";
pub const README_END: &str = "<!-- parity:end -->";
pub const ULTIMATE_TAG: &str = "[Ultimate]";
const MAX_WEIGHT: u32 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Missing,
    Planned,
    Partial,
    Implemented,
    Verified,
}

impl Status {
    pub const ALL: [Status; 5] = [
        Status::Missing,
        Status::Planned,
        Status::Partial,
        Status::Implemented,
        Status::Verified,
    ];

    pub fn parse(cell: &str) -> Option<Status> {
        let cell = cell.trim().trim_matches('*').trim();
        Status::ALL.into_iter().find(|s| s.name() == cell)
    }

    pub fn name(self) -> &'static str {
        match self {
            Status::Missing => "Missing",
            Status::Planned => "Planned",
            Status::Partial => "Partial",
            Status::Implemented => "Implemented",
            Status::Verified => "Verified",
        }
    }

    fn index(self) -> usize {
        self as usize
    }

    /// Credit in half-points: Implemented/Verified = 2, Partial = 1, otherwise 0.
    fn half_credit(self) -> u64 {
        match self {
            Status::Implemented | Status::Verified => 2,
            Status::Partial => 1,
            Status::Missing | Status::Planned => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub status: Status,
    pub weight: u32,
    pub ultimate: bool,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Area {
    /// File name, e.g. `intellij-editor.md`.
    pub file: String,
    /// First `# ` heading, or the file stem when there is none.
    pub title: String,
    pub rows: Vec<Row>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParityError {
    pub file: String,
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}: {}", self.file, self.message)
        } else {
            write!(f, "{}:{}: {}", self.file, self.line, self.message)
        }
    }
}

impl std::error::Error for ParityError {}

/// Splits a Markdown table row into trimmed cells, honouring `\|` escapes.
fn split_cells(line: &str) -> Vec<String> {
    let inner = line.trim();
    let inner = inner.strip_prefix('|').unwrap_or(inner);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                current.push('|');
                chars.next();
            }
            '|' => cells.push(std::mem::take(&mut current).trim().to_string()),
            _ => current.push(c),
        }
    }
    cells.push(current.trim().to_string());
    cells
}

fn is_separator(line: &str) -> bool {
    let line = line.trim();
    line.starts_with('|')
        && split_cells(line).iter().all(|cell| {
            let c = cell.trim_matches(':');
            !c.is_empty() && c.chars().all(|ch| ch == '-')
        })
}

struct Columns {
    count: usize,
    id: usize,
    status: usize,
    weight: Option<usize>,
}

fn columns(header: &[String]) -> Option<Columns> {
    let find = |name: &str| {
        header
            .iter()
            .position(|h| h.trim_matches('*').trim().eq_ignore_ascii_case(name))
    };
    Some(Columns {
        count: header.len(),
        id: find("ID")?,
        status: find("Status")?,
        weight: find("Weight"),
    })
}

fn parse_weight(cell: &str) -> Result<u32, String> {
    let cell = cell.trim();
    if cell.is_empty() || cell == "—" || cell == "-" {
        return Ok(1);
    }
    match cell.parse::<u32>() {
        Ok(w) if (1..=MAX_WEIGHT).contains(&w) => Ok(w),
        _ => Err(format!(
            "invalid weight `{cell}`; expected an integer from 1 to {MAX_WEIGHT}"
        )),
    }
}

/// Parses one area document.
pub fn parse_area(file: &str, text: &str) -> Result<Area, ParityError> {
    let err = |line: usize, message: String| ParityError {
        file: file.to_string(),
        line,
        message,
    };
    let title = text
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .map(|t| t.trim().to_string())
        .unwrap_or_else(|| file.trim_end_matches(".md").to_string());
    let lines: Vec<&str> = text.lines().collect();
    let mut rows = Vec::new();
    let mut table: Option<Columns> = None;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let line_no = i + 1;
        if !line.trim_start().starts_with('|') {
            table = None;
            i += 1;
            continue;
        }
        if table.is_none() {
            // A table starts with a header row followed by a separator row.
            if lines.get(i + 1).is_some_and(|next| is_separator(next)) {
                table = columns(&split_cells(line));
                if table.is_none() {
                    // Not a parity table: skip it entirely.
                    while i < lines.len() && lines[i].trim_start().starts_with('|') {
                        i += 1;
                    }
                    continue;
                }
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        let Some(cols) = table.as_ref() else {
            i += 1;
            continue;
        };
        let cells = split_cells(line);
        if cells.len() != cols.count {
            return Err(err(
                line_no,
                format!(
                    "row has {} cells but the table header has {}",
                    cells.len(),
                    cols.count
                ),
            ));
        }
        let id = cells[cols.id].trim_matches('`').to_string();
        if id.is_empty() {
            return Err(err(line_no, "row has an empty ID".into()));
        }
        let status = Status::parse(&cells[cols.status]).ok_or_else(|| {
            err(
                line_no,
                format!(
                    "unknown status `{}` for {id}; expected one of Missing, Planned, Partial, Implemented, Verified",
                    cells[cols.status]
                ),
            )
        })?;
        let weight = match cols.weight {
            Some(w) => parse_weight(&cells[w]).map_err(|m| err(line_no, format!("{id}: {m}")))?,
            None => 1,
        };
        let ultimate = cells.iter().any(|c| c.contains(ULTIMATE_TAG));
        rows.push(Row {
            id,
            status,
            weight,
            ultimate,
            line: line_no,
        });
        i += 1;
    }
    Ok(Area {
        file: file.to_string(),
        title,
        rows,
    })
}

/// Rejects IDs used more than once across all areas.
pub fn check_unique_ids(areas: &[Area]) -> Result<(), ParityError> {
    let mut seen: BTreeMap<&str, (&str, usize)> = BTreeMap::new();
    for area in areas {
        for row in &area.rows {
            if let Some((file, line)) = seen.insert(&row.id, (&area.file, row.line)) {
                return Err(ParityError {
                    file: area.file.clone(),
                    line: row.line,
                    message: format!("duplicate ID {} (first defined at {file}:{line})", row.id),
                });
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub rows: [u64; 5],
    pub weight: [u64; 5],
}

impl Stats {
    pub fn add(&mut self, row: &Row) {
        self.rows[row.status.index()] += 1;
        self.weight[row.status.index()] += u64::from(row.weight);
    }

    pub fn of<'a>(rows: impl IntoIterator<Item = &'a Row>) -> Stats {
        let mut stats = Stats::default();
        for row in rows {
            stats.add(row);
        }
        stats
    }

    pub fn total_rows(&self) -> u64 {
        self.rows.iter().sum()
    }

    pub fn total_weight(&self) -> u64 {
        self.weight.iter().sum()
    }

    fn half_points(&self) -> u64 {
        Status::ALL
            .iter()
            .map(|s| self.weight[s.index()] * s.half_credit())
            .sum()
    }

    /// Weighted percent implemented, in tenths of a percent (rounded half up).
    pub fn implemented_permille(&self) -> Option<u64> {
        permille(self.half_points(), 2 * self.total_weight())
    }

    /// Weighted percent verified, in tenths of a percent (rounded half up).
    pub fn verified_permille(&self) -> Option<u64> {
        permille(self.weight[Status::Verified.index()], self.total_weight())
    }
}

fn permille(numerator: u64, denominator: u64) -> Option<u64> {
    (denominator > 0).then(|| (numerator * 1000 + denominator / 2) / denominator)
}

pub fn format_pct(permille: Option<u64>) -> String {
    match permille {
        Some(p) => format!("{}.{}%", p / 10, p % 10),
        None => "—".to_string(),
    }
}

fn status_cells(stats: &Stats) -> String {
    Status::ALL
        .iter()
        .map(|s| stats.rows[s.index()].to_string())
        .collect::<Vec<_>>()
        .join(" | ")
}

pub fn render_dashboard(areas: &[Area]) -> String {
    let all = Stats::of(areas.iter().flat_map(|a| &a.rows));
    let community = Stats::of(areas.iter().flat_map(|a| &a.rows).filter(|r| !r.ultimate));
    let ultimate = Stats::of(areas.iter().flat_map(|a| &a.rows).filter(|r| r.ultimate));
    let mut out = String::new();
    out.push_str("# Parity dashboard\n\n");
    out.push_str(
        "Generated by `cargo run -p xtask -- parity` from `docs/parity/*.md`; do not edit by hand.\n\
         Regenerate with `cargo run -p xtask -- parity`; `--check` validates the tables.\n\n\
         Scoring: each row has a weight (default 1). **% implemented** counts Implemented and\n\
         Verified rows fully and Partial rows at half weight. **% verified** counts only Verified\n\
         rows. Rows tagged `[Ultimate]` are IntelliJ IDEA Ultimate-only features.\n\n",
    );
    out.push_str("## Overall\n\n| Metric | Value |\n|---|---:|\n");
    out.push_str(&format!("| Areas | {} |\n", areas.len()));
    out.push_str(&format!("| Rows | {} |\n", all.total_rows()));
    out.push_str(&format!("| Total weight | {} |\n", all.total_weight()));
    out.push_str(&format!(
        "| **Weighted % implemented** | **{}** |\n",
        format_pct(all.implemented_permille())
    ));
    out.push_str(&format!(
        "| **Weighted % verified** | **{}** |\n\n",
        format_pct(all.verified_permille())
    ));
    out.push_str("## Status counts\n\n| Status | Rows | Weight |\n|---|---:|---:|\n");
    for s in Status::ALL {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            s.name(),
            all.rows[s.index()],
            all.weight[s.index()]
        ));
    }
    out.push_str(&format!(
        "| **Total** | **{}** | **{}** |\n\n",
        all.total_rows(),
        all.total_weight()
    ));
    out.push_str(
        "## By area\n\n| Area | File | Rows | Weight | Missing | Planned | Partial | Implemented | Verified | % implemented | % verified |\n\
         |---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n",
    );
    for area in areas {
        let s = Stats::of(&area.rows);
        out.push_str(&format!(
            "| {} | [{}]({}) | {} | {} | {} | {} | {} |\n",
            area.title.replace('|', "\\|"),
            area.file,
            area.file,
            s.total_rows(),
            s.total_weight(),
            status_cells(&s),
            format_pct(s.implemented_permille()),
            format_pct(s.verified_permille()),
        ));
    }
    out.push_str(
        "\n## By edition\n\n| Edition scope | Rows | Weight | % implemented | % verified |\n|---|---:|---:|---:|---:|\n",
    );
    for (label, s) in [
        ("Community / platform (untagged)", community),
        ("Ultimate-only (`[Ultimate]`)", ultimate),
    ] {
        out.push_str(&format!(
            "| {label} | {} | {} | {} | {} |\n",
            s.total_rows(),
            s.total_weight(),
            format_pct(s.implemented_permille()),
            format_pct(s.verified_permille()),
        ));
    }
    out
}

/// The "Behavior scenarios" section: the baseline of `cargo xtask scenario`, committed as
/// `docs/parity/scenarios/REPORT.json`. Empty when there is no report yet.
fn scenario_section(dir: &Path) -> String {
    let Ok(text) = fs::read_to_string(dir.join("scenarios/REPORT.json")) else {
        return String::new();
    };
    let Ok(report) = serde_json::from_str::<serde_json::Value>(&text) else {
        return String::new();
    };
    let count = |key: &str| report["totals"][key].as_u64().unwrap_or(0);
    let (pass, partial, fail, skipped) = (count("pass"), count("partial"), count("fail"), count("skipped"));
    let ran = pass + partial + fail;
    let score = if ran == 0 {
        "-".to_owned()
    } else {
        format!("{:.0}%", (pass as f64 + partial as f64 / 2.0) / ran as f64 * 100.0)
    };
    format!(
        "\n## Behavior scenarios\n\n\
         Recorded IntelliJ behavior replayed against the real IDE (`cargo run -p xtask -- scenario`,\n\
         corpus and baseline in [`scenarios/`](scenarios/README.md), per scenario in\n\
         [`scenarios/REPORT.md`](scenarios/REPORT.md)). Pass counts 1, partial one half, over the\n\
         scenarios that ran.\n\n\
         | Scenarios | Pass | Partial | Fail | Skipped | **Score** |\n|---:|---:|---:|---:|---:|---:|\n\
         | {} | {pass} | {partial} | {fail} | {skipped} | **{score}** |\n",
        pass + partial + fail + skipped
    )
}

pub fn render_readme_block(areas: &[Area]) -> String {
    let all = Stats::of(areas.iter().flat_map(|a| &a.rows));
    format!(
        "{README_BEGIN}\n**Parity progress:** {} implemented (weighted), {} verified, across {} tracked\nfeatures in {} areas. See [`docs/parity/dashboard.md`](docs/parity/dashboard.md).\n{README_END}",
        format_pct(all.implemented_permille()),
        format_pct(all.verified_permille()),
        all.total_rows(),
        areas.len(),
    )
}

/// Replaces the marked parity block in a README, appending one when absent.
pub fn update_readme(readme: &str, block: &str) -> Result<String, String> {
    match (readme.find(README_BEGIN), readme.find(README_END)) {
        (Some(begin), Some(end)) if begin < end => {
            let end = end + README_END.len();
            Ok(format!("{}{}{}", &readme[..begin], block, &readme[end..]))
        }
        (None, None) => {
            let mut out = readme.trim_end().to_string();
            out.push_str("\n\n");
            out.push_str(block);
            out.push('\n');
            Ok(out)
        }
        _ => Err(format!(
            "README parity markers are malformed; expected `{README_BEGIN}` before `{README_END}`"
        )),
    }
}

/// Loads every area file in `dir`, sorted by file name.
pub fn load_areas(dir: &Path) -> Result<Vec<Area>, Box<dyn std::error::Error>> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| format!("read {}: {e}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().is_some_and(|e| e == "md")
                && p.file_name()
                    .is_some_and(|n| !NON_AREA_FILES.iter().any(|x| n == *x))
        })
        .collect();
    files.sort();
    let mut areas = Vec::new();
    for path in files {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let text =
            fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        areas.push(parse_area(&name, &text)?);
    }
    check_unique_ids(&areas)?;
    Ok(areas)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// `--check`: every table parsed and IDs are unique.
    Valid {
        rows: usize,
    },
    Written(Vec<PathBuf>),
    UpToDate,
}

/// Validates the parity tables and, unless `check` is set, regenerates the
/// dashboard and README summary.
///
/// `--check` only validates rows (known status, weights, unique IDs) and does not
/// compare the generated files, so parallel PRs that each add rows do not conflict
/// on them; they are regenerated on main.
pub fn run(root: &Path, check: bool) -> Result<Outcome, Box<dyn std::error::Error>> {
    let dir = root.join("docs/parity");
    let areas = load_areas(&dir)?;
    if check {
        let rows = areas.iter().map(|a| a.rows.len()).sum();
        return Ok(Outcome::Valid { rows });
    }
    let dashboard_path = dir.join(DASHBOARD_FILE);
    let readme_path = root.join("README.md");
    let mut dashboard = render_dashboard(&areas);
    dashboard.push_str(&scenario_section(&dir));
    let readme_current = fs::read_to_string(&readme_path).unwrap_or_default();
    let readme = update_readme(&readme_current, &render_readme_block(&areas))?;
    let dashboard_current = fs::read_to_string(&dashboard_path).unwrap_or_default();
    let mut written = Vec::new();
    for (path, current, content) in [
        (dashboard_path, dashboard_current, dashboard),
        (readme_path, readme_current, readme),
    ] {
        if current != content {
            fs::write(&path, content).map_err(|e| format!("write {}: {e}", path.display()))?;
            written.push(path);
        }
    }
    if written.is_empty() {
        Ok(Outcome::UpToDate)
    } else {
        Ok(Outcome::Written(written))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEVEN: &str = "# Platform parity\n\n\
| ID | Reference | Feature | Status | Commands | Tests | Limitations / evidence |\n\
|---|---|---|---|---|---|---|\n\
| P-1 | IntelliJ Platform | Window | Implemented | `app.quit` | manual | ok |\n\
| P-2 | IntelliJ Platform | Tree | Partial | — | — | — |\n\
| P-3 | IntelliJ Platform | Docks | Missing | — | — | — |\n";

    const EIGHT: &str = "# Editor\n\n\
| ID | Reference | Feature | Status | Weight | Commands | Tests | Limitations / evidence |\n\
|---|---|---|---|---:|---|---|---|\n\
| E-1 | ref | Undo | Implemented | 3 | `editor.undo` | t | — |\n\
| E-2 | ref | Folding | Partial | 2 | — | — | — |\n\
| E-3 | ref | [Ultimate] Spring | Missing | 5 | — | — | — |\n\
| E-4 | ref | Pipe \\| escaped | Verified |  | — | — | — |\n";

    #[test]
    fn parses_seven_column_tables_with_default_weight() {
        let area = parse_area("platform.md", SEVEN).unwrap();
        assert_eq!(area.title, "Platform parity");
        assert_eq!(area.rows.len(), 3);
        assert!(area.rows.iter().all(|r| r.weight == 1 && !r.ultimate));
        let s = Stats::of(&area.rows);
        // (1 + 0.5) / 3 = 50.0%
        assert_eq!(format_pct(s.implemented_permille()), "50.0%");
        assert_eq!(format_pct(s.verified_permille()), "0.0%");
    }

    #[test]
    fn parses_eight_column_tables_with_weights_and_edition_tag() {
        let area = parse_area("editor.md", EIGHT).unwrap();
        let weights: Vec<u32> = area.rows.iter().map(|r| r.weight).collect();
        assert_eq!(weights, [3, 2, 5, 1]);
        assert!(area.rows[2].ultimate);
        assert_eq!(area.rows[3].status, Status::Verified);
        let s = Stats::of(&area.rows);
        assert_eq!(s.total_weight(), 11);
        // (3 + 1 + 0 + 1) / 11 = 45.45%
        assert_eq!(format_pct(s.implemented_permille()), "45.5%");
        // 1 / 11 = 9.09%
        assert_eq!(format_pct(s.verified_permille()), "9.1%");
    }

    #[test]
    fn accepts_tables_without_reference_column() {
        let text = "| ID | Feature | Status | Commands | Tests | Known limitations |\n\
|---|---|---|---|---|---|\n| A-1 | SDK | Planned | — | — | — |\n";
        let area = parse_area("android.md", text).unwrap();
        assert_eq!(area.title, "android");
        assert_eq!(area.rows[0].status, Status::Planned);
    }

    #[test]
    fn ignores_non_parity_tables_and_prose() {
        let text = "# X\n\n| Key | Value |\n|---|---|\n| a | Missing |\n\nSome | prose\n\n\
| ID | Status |\n|---|---|\n| X-1 | Implemented |\n";
        let area = parse_area("x.md", text).unwrap();
        assert_eq!(area.rows.len(), 1);
        assert_eq!(area.rows[0].id, "X-1");
        assert_eq!(area.rows[0].line, 11);
    }

    #[test]
    fn rejects_unknown_status_with_location() {
        let text = "| ID | Status |\n|---|---|\n| X-1 | Done |\n";
        let e = parse_area("x.md", text).unwrap_err();
        assert_eq!(e.line, 3);
        assert!(e.to_string().starts_with("x.md:3: unknown status `Done`"));
    }

    #[test]
    fn rejects_invalid_weights() {
        for bad in ["0", "abc", "1.5", "101", "-2"] {
            let text =
                format!("| ID | Status | Weight |\n|---|---|---|\n| X-1 | Missing | {bad} |\n");
            let e = parse_area("x.md", &text).unwrap_err();
            assert!(e.message.contains("invalid weight"), "{bad}: {e}");
        }
    }

    #[test]
    fn rejects_rows_with_wrong_cell_count() {
        let text = "| ID | Status | Weight |\n|---|---|---|\n| X-1 | Missing |\n";
        let e = parse_area("x.md", text).unwrap_err();
        assert!(e.message.contains("2 cells but the table header has 3"));
    }

    #[test]
    fn rejects_duplicate_ids_across_areas() {
        let a = parse_area("a.md", "| ID | Status |\n|---|---|\n| D-1 | Missing |\n").unwrap();
        let b = parse_area("b.md", "| ID | Status |\n|---|---|\n| D-1 | Partial |\n").unwrap();
        let e = check_unique_ids(&[a, b]).unwrap_err();
        assert_eq!(e.file, "b.md");
        assert!(e.message.contains("first defined at a.md:3"));
    }

    #[test]
    fn empty_area_reports_dash() {
        assert_eq!(format_pct(Stats::default().implemented_permille()), "—");
    }

    #[test]
    fn readme_block_is_replaced_or_appended() {
        let block = format!("{README_BEGIN}\nnew\n{README_END}");
        let replaced = update_readme(
            &format!("a\n{README_BEGIN}\nold\n{README_END}\nb\n"),
            &block,
        )
        .unwrap();
        assert_eq!(replaced, format!("a\n{block}\nb\n"));
        assert_eq!(
            update_readme("a\n", &block).unwrap(),
            format!("a\n\n{block}\n")
        );
        assert!(update_readme(&format!("{README_END}{README_BEGIN}"), &block).is_err());
    }

    #[test]
    fn dashboard_includes_areas_and_editions() {
        let areas = vec![
            parse_area("editor.md", EIGHT).unwrap(),
            parse_area("platform.md", SEVEN).unwrap(),
        ];
        let d = render_dashboard(&areas);
        assert!(d.contains("| Rows | 7 |"));
        assert!(d.contains("| Total weight | 14 |"));
        // (5 half-units*2=10 + platform 3) / 28 -> (10 + 3) / 28 = 46.4%
        assert!(
            d.contains("| **Weighted % implemented** | **46.4%** |"),
            "{d}"
        );
        assert!(d.contains(
            "| Editor | [editor.md](editor.md) | 4 | 11 | 1 | 0 | 1 | 1 | 1 | 45.5% | 9.1% |"
        ));
        assert!(d.contains("| Ultimate-only (`[Ultimate]`) | 1 | 5 | 0.0% | 0.0% |"));
    }

    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("xtask-parity-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("docs/parity")).unwrap();
        dir
    }

    #[test]
    fn check_validates_and_run_writes_generated_files() {
        let root = temp_root("run");
        fs::write(root.join("docs/parity/platform.md"), SEVEN).unwrap();
        fs::write(root.join("README.md"), "# Demo\n").unwrap();
        // The format guide is not an area, so its example rows are never parsed.
        fs::write(
            root.join("docs/parity/README.md"),
            "| ID | Status |\n|---|---|\n| P-1 | Example |\n",
        )
        .unwrap();
        assert_eq!(run(&root, true).unwrap(), Outcome::Valid { rows: 3 });
        assert!(!root.join("docs/parity/dashboard.md").exists());
        assert!(matches!(run(&root, false).unwrap(), Outcome::Written(p) if p.len() == 2));
        assert_eq!(run(&root, false).unwrap(), Outcome::UpToDate);
        let readme = fs::read_to_string(root.join("README.md")).unwrap();
        assert!(readme.contains("**Parity progress:** 50.0% implemented (weighted)"));
        fs::write(
            root.join("docs/parity/editor.md"),
            "| ID | Reference | Feature | Status | Commands | Tests | Limitations / evidence |\n|---|---|---|---|---|---|---|\n| P-1 | r | Dup | Bogus | — | — | — |\n",
        )
        .unwrap();
        assert!(run(&root, true).is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn dashboard_summarizes_the_scenario_baseline_when_there_is_one() {
        let root = temp_root("scenarios");
        let dir = root.join("docs/parity");
        assert_eq!(scenario_section(&dir), "");
        fs::create_dir_all(dir.join("scenarios")).unwrap();
        fs::write(
            dir.join("scenarios/REPORT.json"),
            r#"{"totals": {"pass": 2, "partial": 2, "fail": 4, "skipped": 1}}"#,
        )
        .unwrap();
        let section = scenario_section(&dir);
        assert!(section.contains("## Behavior scenarios"));
        assert!(section.contains("| 9 | 2 | 2 | 4 | 1 | **38%** |"), "{section}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn repository_inventory_is_valid() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let areas = load_areas(&root.join("docs/parity")).unwrap();
        assert!(areas.iter().map(|a| a.rows.len()).sum::<usize>() > 0);
    }
}
