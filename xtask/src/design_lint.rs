//! `cargo xtask design-lint`: keeps ad-hoc painting out of app code.
//!
//! Every color, radius and island must come from archipel-kit. The lint counts
//! hand-rolled colors and shapes per file and rule, and fails when a count
//! rises above `xtask/design-lint-baseline.txt`. Existing hits are grandfathered
//! there and shrink as areas migrate (`docs/ui-migration.md`); run with
//! `--update` after a migration to lower the baseline. Test modules
//! (`#[cfg(test)]` to the end of the file) and `tests/` directories are skipped.

use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Roots holding UI code. Engine crates have no egui and are not scanned.
const ROOTS: &[&str] = &[
    "crates/archipel-ide-ui/src",
    "crates/archipel-ide-app/src",
    "crates/archipel-ide-plugin-ui/src",
    "plugins",
];
const BASELINE: &str = "xtask/design-lint-baseline.txt";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Rule {
    /// `Color32::from_rgb*` / `from_rgba*`: a color that is not a kit token.
    RgbColor,
    /// `0xRRGGBB` / `0xRRGGBBAA` literals or `from_hex`: a hand-written palette.
    HexPalette,
    /// `rect_filled` with a non-zero radius: a hand-painted island or button.
    RoundedFill,
    /// `CornerRadius::same`: a radius that is not a kit token.
    CornerRadius,
}

impl Rule {
    const ALL: [Rule; 4] = [
        Rule::RgbColor,
        Rule::HexPalette,
        Rule::RoundedFill,
        Rule::CornerRadius,
    ];

    fn name(self) -> &'static str {
        match self {
            Rule::RgbColor => "rgb-color",
            Rule::HexPalette => "hex-palette",
            Rule::RoundedFill => "rounded-fill",
            Rule::CornerRadius => "corner-radius",
        }
    }

    fn parse(name: &str) -> Option<Rule> {
        Rule::ALL.into_iter().find(|rule| rule.name() == name)
    }
}

type Counts = BTreeMap<(String, Rule), usize>;

pub fn run(args: impl Iterator<Item = String>) -> Result<()> {
    let mut update = false;
    for arg in args {
        match arg.as_str() {
            "--update" => update = true,
            other => bail!("unknown argument `{other}`; usage: cargo xtask design-lint [--update]"),
        }
    }
    let root = crate::workspace_root();
    let counts = scan(&root)?;
    let baseline_path = root.join(BASELINE);
    if update {
        fs::write(&baseline_path, render(&counts))
            .with_context(|| format!("write {}", baseline_path.display()))?;
        println!("wrote {} ({} entries)", BASELINE, counts.len());
        return Ok(());
    }
    let baseline = parse_baseline(&fs::read_to_string(&baseline_path).unwrap_or_default())?;
    let mut failures = Vec::new();
    let mut improved = 0;
    for (key, &count) in &counts {
        let allowed = baseline.get(key).copied().unwrap_or(0);
        if count > allowed {
            failures.push(format!(
                "{}: {} {} (baseline {allowed})",
                key.0,
                count,
                key.1.name()
            ));
        }
    }
    for (key, &allowed) in &baseline {
        if counts.get(key).copied().unwrap_or(0) < allowed {
            improved += 1;
        }
    }
    if !failures.is_empty() {
        bail!(
            "design lint: new ad-hoc painting outside archipel-kit:\n  {}\n\
             Use kit components, parts and tokens instead (docs/ui-migration.md). \
             If the kit lacks something, request it in archipel-dev/kit.",
            failures.join("\n  ")
        );
    }
    let total: usize = counts.values().sum();
    println!(
        "design lint: ok ({total} grandfathered hits in {} entries)",
        counts.len()
    );
    if improved > 0 {
        println!(
            "design lint: {improved} entries dropped below the baseline; \
             run `cargo xtask design-lint --update` to lock that in"
        );
    }
    Ok(())
}

fn scan(root: &Path) -> Result<Counts> {
    let mut files = Vec::new();
    for dir in ROOTS {
        collect(&root.join(dir), &mut files)?;
    }
    files.sort();
    let mut counts = Counts::new();
    for file in files {
        let text = fs::read_to_string(&file).with_context(|| format!("read {}", file.display()))?;
        let rel = file
            .strip_prefix(root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        for (rule, count) in count_hits(&text) {
            counts.insert((rel.clone(), rule), count);
        }
    }
    Ok(counts)
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries {
        let path = entry?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if !matches!(name, "target" | "tests" | "benches") && !name.starts_with('.') {
                collect(&path, files)?;
            }
        } else if name.ends_with(".rs") {
            files.push(path);
        }
    }
    Ok(())
}

/// Hits per rule in one source file, ignoring comments and the trailing test module.
fn count_hits(text: &str) -> BTreeMap<Rule, usize> {
    let code = strip_tests(text);
    let mut hits = BTreeMap::new();
    let mut bump = |rule| *hits.entry(rule).or_insert(0) += 1;
    for line in code.lines() {
        let line = line.split("//").next().unwrap_or("");
        for _ in line.matches("Color32::from_rgb") {
            bump(Rule::RgbColor);
        }
        for _ in line.matches("from_hex") {
            bump(Rule::HexPalette);
        }
        for _ in 0..hex_colors(line) {
            bump(Rule::HexPalette);
        }
        for _ in line.matches("CornerRadius::same") {
            bump(Rule::CornerRadius);
        }
    }
    for (index, _) in code.match_indices("rect_filled(") {
        if rounded(&code[index + "rect_filled(".len()..]) {
            bump(Rule::RoundedFill);
        }
    }
    hits
}

/// The file up to its `#[cfg(test)]` module (by convention the last item).
fn strip_tests(text: &str) -> &str {
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line.trim() == "#[cfg(test)]" {
            let rest = &text[offset + line.len()..];
            if rest.trim_start().starts_with("mod ") {
                return &text[..offset];
            }
        }
        offset += line.len();
    }
    text
}

/// Counts `0x` literals with exactly 6 or 8 hex digits (`0xRRGGBB`, `0xRRGGBBAA`).
fn hex_colors(line: &str) -> usize {
    let bytes = line.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i + 1 < bytes.len() {
        let boundary = i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
        if boundary && bytes[i] == b'0' && bytes[i + 1] == b'x' {
            let digits = bytes[i + 2..]
                .iter()
                .take_while(|b| b.is_ascii_hexdigit() || **b == b'_')
                .filter(|b| **b != b'_')
                .count();
            let len = bytes[i + 2..]
                .iter()
                .take_while(|b| b.is_ascii_hexdigit() || **b == b'_')
                .count();
            if matches!(digits, 6 | 8) {
                count += 1;
            }
            i += 2 + len;
        } else {
            i += 1;
        }
    }
    count
}

/// Whether the radius (second argument) of a `rect_filled(` call is non-zero.
fn rounded(args: &str) -> bool {
    let mut depth = 0usize;
    let mut start = None;
    for (i, c) in args.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' if depth == 0 => return false,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => match start {
                None => start = Some(i + 1),
                Some(s) => return !zero_radius(args[s..i].trim()),
            },
            _ => {}
        }
    }
    false
}

fn zero_radius(arg: &str) -> bool {
    matches!(
        arg,
        "0" | "0.0" | "0.0_f32" | "0.0f32" | "egui::CornerRadius::ZERO" | "CornerRadius::ZERO"
    )
}

fn render(counts: &Counts) -> String {
    let mut out = String::from(
        "# Grandfathered design-lint hits: <file> <rule> <count>.\n\
         # Regenerate with `cargo xtask design-lint --update`; counts may only go down.\n",
    );
    for ((file, rule), count) in counts {
        out.push_str(&format!("{file} {} {count}\n", rule.name()));
    }
    out
}

fn parse_baseline(text: &str) -> Result<Counts> {
    let mut counts = Counts::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        let [file, rule, count] = parts.as_slice() else {
            bail!(
                "{BASELINE}:{}: expected `<file> <rule> <count>`",
                number + 1
            );
        };
        let rule = Rule::parse(rule)
            .with_context(|| format!("{BASELINE}:{}: unknown rule `{rule}`", number + 1))?;
        let count = count
            .parse()
            .with_context(|| format!("{BASELINE}:{}: bad count", number + 1))?;
        counts.insert(((*file).to_owned(), rule), count);
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_each_rule_and_skips_comments_zero_radius_and_tests() {
        let source = r#"
fn paint(ui: &mut egui::Ui) {
    let a = Color32::from_rgb(1, 2, 3);
    let b = Color32::from_rgba_unmultiplied(1, 2, 3, 4); // Color32::from_rgb in a comment
    let palette = [0x1E1F22, 0xFFFFFF29, 0x10, 0x1234_5678_9ABC];
    ui.painter().rect_filled(rect, 6.0, a);
    ui.painter().rect_filled(rect, 0.0, a);
    ui.painter().rect_filled(
        rect.shrink(f(1.0, 2.0)),
        egui::CornerRadius::same(4),
        a,
    );
}

#[cfg(test)]
mod tests {
    const X: u32 = 0xABCDEF;
}
"#;
        let hits = count_hits(source);
        assert_eq!(hits.get(&Rule::RgbColor), Some(&2));
        assert_eq!(hits.get(&Rule::HexPalette), Some(&2));
        assert_eq!(hits.get(&Rule::RoundedFill), Some(&2));
        assert_eq!(hits.get(&Rule::CornerRadius), Some(&1));
    }

    #[test]
    fn baseline_round_trips() {
        let mut counts = Counts::new();
        counts.insert(("crates/a.rs".into(), Rule::RgbColor), 3);
        counts.insert(("crates/a.rs".into(), Rule::RoundedFill), 1);
        assert_eq!(parse_baseline(&render(&counts)).unwrap(), counts);
        assert!(parse_baseline("crates/a.rs nope 1").is_err());
    }
}
