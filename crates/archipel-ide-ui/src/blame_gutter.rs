//! "Annotate with Git Blame" column painted at the left of the editor gutter:
//! date and author of the commit that last changed each line, shaded by age.

use crate::theme;
use archipel_ide_git::BlameLine;
use eframe::egui::{self, Color32, Pos2, pos2};

/// Width added to the gutter while annotations are shown.
pub const WIDTH: f32 = 140.0;

/// Text colour by age: the newest changes are brightest.
fn age_color(age_days: i64) -> Color32 {
    match age_days {
        d if d < 7 => *theme::TEXT,
        d if d < 60 => *theme::TEXT_SECONDARY,
        _ => *theme::TEXT_INFO,
    }
}

/// Paints the annotation of `line`; `origin` is the row's left top.
pub fn paint(
    painter: &egui::Painter,
    lines: &[BlameLine],
    line: usize,
    origin: Pos2,
    height: f32,
    now: i64,
) {
    let Some(blame) = lines.get(line) else {
        return;
    };
    if blame.is_uncommitted() {
        return;
    }
    let date = crate::git_log_ui::absolute_time(blame.time);
    let date = date.split(' ').next().unwrap_or_default();
    let author: String = blame.author.chars().take(8).collect();
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE - 1.0);
    let color = age_color((now - blame.time) / 86_400);
    let clip = egui::Rect::from_min_size(origin, egui::vec2(WIDTH - 8.0, height));
    let painter = painter.with_clip_rect(clip.intersect(painter.clip_rect()));
    painter.text(
        pos2(origin.x + 8.0, origin.y + height / 2.0),
        egui::Align2::LEFT_CENTER,
        date,
        font.clone(),
        color,
    );
    painter.text(
        pos2(origin.x + 82.0, origin.y + height / 2.0),
        egui::Align2::LEFT_CENTER,
        author,
        font,
        color,
    );
}

/// Tooltip text for `line`.
pub fn tooltip(lines: &[BlameLine], line: usize) -> Option<String> {
    let blame = lines.get(line)?;
    if blame.is_uncommitted() {
        return Some("Not committed yet".into());
    }
    Some(format!(
        "{} {} {}\n\n{}\n\nClick to show the commit in the Git log",
        blame.short(),
        blame.author,
        crate::git_log_ui::absolute_time(blame.time),
        blame.summary
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(hash: &str) -> BlameLine {
        BlameLine {
            hash: hash.repeat(40),
            author: "Ada".into(),
            time: 0,
            summary: "Initial".into(),
        }
    }

    #[test]
    fn tooltips_describe_the_commit_or_uncommitted_lines() {
        let lines = vec![line("a"), line("0")];
        let text = tooltip(&lines, 0).unwrap();
        assert!(text.starts_with("aaaaaaaa Ada 1970-01-01"), "{text}");
        assert!(text.contains("Initial"));
        assert_eq!(tooltip(&lines, 1).unwrap(), "Not committed yet");
        assert!(tooltip(&lines, 5).is_none());
    }

    #[test]
    fn recent_changes_are_brighter() {
        assert_eq!(age_color(1), *theme::TEXT);
        assert_eq!(age_color(400), *theme::TEXT_INFO);
    }
}
