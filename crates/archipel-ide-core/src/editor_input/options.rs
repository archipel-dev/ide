//! The editor settings the input layer reads (Settings | Editor | General, Smart Keys, Code
//! Style). One snapshot per action, so a setting changed in the dialog applies to the next key.

use crate::services::Settings;
use archipel_ide_settings::ids;

/// `Unindent on Backspace` (ED-TYPE-046).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnindentMode {
    Disabled,
    Nearest,
    Proper,
}

/// `Reformat on paste` (ED-TYPE-102).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteMode {
    None,
    IndentBlock,
    IndentLines,
    Reformat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorOptions {
    pub pair_brackets: bool,
    pub pair_quotes: bool,
    pub surround_selection: bool,
    pub pair_brace_on_enter: bool,
    pub tab_out: bool,
    pub smart_indent: bool,
    pub close_block_comment: bool,
    pub smart_home: bool,
    pub unindent: UnindentMode,
    pub paste: PasteMode,
    pub camel_humps: bool,
    /// Windows caret stop policy: Next Word stops at the next word start (ED-CARET-010).
    pub caret_stop_windows: bool,
    pub copy_line_without_selection: bool,
    pub undo_transparent_caret: bool,
    pub use_tab: bool,
    pub indent_size: usize,
    pub detect_indents: bool,
    pub line_comment_first_column: bool,
    pub line_comment_space: bool,
    pub block_comment_space: bool,
    pub comment_moves_caret: bool,
    pub tab_expands_templates: bool,
}

impl Default for EditorOptions {
    fn default() -> Self {
        Self {
            pair_brackets: true,
            pair_quotes: true,
            surround_selection: true,
            pair_brace_on_enter: true,
            tab_out: true,
            smart_indent: true,
            close_block_comment: true,
            smart_home: true,
            unindent: UnindentMode::Proper,
            paste: PasteMode::IndentLines,
            camel_humps: false,
            caret_stop_windows: false,
            copy_line_without_selection: true,
            undo_transparent_caret: false,
            use_tab: false,
            indent_size: 4,
            detect_indents: true,
            line_comment_first_column: true,
            line_comment_space: false,
            block_comment_space: false,
            comment_moves_caret: true,
            tab_expands_templates: true,
        }
    }
}

impl EditorOptions {
    pub fn from_settings(settings: &Settings) -> Self {
        let defaults = Self::default();
        let flag = |id: &str, default: bool| settings.bool(id).unwrap_or(default);
        let text = |id: &str| settings.text(id).unwrap_or_default();
        Self {
            pair_brackets: flag(ids::PAIR_BRACKETS, defaults.pair_brackets),
            pair_quotes: flag(ids::PAIR_QUOTES, defaults.pair_quotes),
            surround_selection: flag(ids::SURROUND_ON_TYPING, defaults.surround_selection),
            pair_brace_on_enter: flag(ids::PAIR_BRACE_ON_ENTER, defaults.pair_brace_on_enter),
            tab_out: flag(ids::TAB_OUT, defaults.tab_out),
            smart_indent: flag(ids::SMART_INDENT, defaults.smart_indent),
            close_block_comment: flag(ids::CLOSE_BLOCK_COMMENT, defaults.close_block_comment),
            smart_home: flag(ids::SMART_HOME, defaults.smart_home),
            unindent: match text(ids::UNINDENT_ON_BACKSPACE).as_str() {
                "none" => UnindentMode::Disabled,
                "nearest" => UnindentMode::Nearest,
                _ => UnindentMode::Proper,
            },
            paste: match text(ids::REFORMAT_ON_PASTE).as_str() {
                "none" => PasteMode::None,
                "block" => PasteMode::IndentBlock,
                "reformat" => PasteMode::Reformat,
                _ => PasteMode::IndentLines,
            },
            camel_humps: flag(ids::CAMEL_HUMPS, defaults.camel_humps),
            caret_stop_windows: text(ids::CARET_STOP) == "windows",
            copy_line_without_selection: !flag(ids::NO_COPY_LINE_WITHOUT_SELECTION, false),
            undo_transparent_caret: flag(
                ids::UNDO_TRANSPARENT_CARET,
                defaults.undo_transparent_caret,
            ),
            use_tab: flag(ids::USE_TAB, defaults.use_tab),
            indent_size: settings
                .int(ids::INDENT_SIZE)
                .map_or(defaults.indent_size, |n| n.clamp(1, 32) as usize),
            detect_indents: flag(ids::DETECT_INDENTS, defaults.detect_indents),
            line_comment_first_column: flag(
                ids::LINE_COMMENT_FIRST_COLUMN,
                defaults.line_comment_first_column,
            ),
            line_comment_space: flag(ids::LINE_COMMENT_SPACE, defaults.line_comment_space),
            block_comment_space: flag(ids::BLOCK_COMMENT_SPACE, defaults.block_comment_space),
            comment_moves_caret: flag(ids::COMMENT_MOVES_CARET, defaults.comment_moves_caret),
            tab_expands_templates: text(ids::TEMPLATE_EXPAND_KEY) != "enter"
                && text(ids::TEMPLATE_EXPAND_KEY) != "space",
        }
    }
}
