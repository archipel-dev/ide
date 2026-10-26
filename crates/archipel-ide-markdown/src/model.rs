//! Markdown source → block model (CommonMark + GFM tables, task lists, strikethrough).

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    None,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text(String),
    Code(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Link {
        url: String,
        title: String,
        children: Vec<Inline>,
    },
    Image {
        url: String,
        alt: String,
    },
    SoftBreak,
    HardBreak,
    Html(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    /// `Some(checked)` for GFM task list items.
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockKind {
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    CodeBlock {
        lang: Option<String>,
        code: String,
    },
    BlockQuote(Vec<Block>),
    List {
        /// First number of an ordered list.
        start: Option<u64>,
        items: Vec<ListItem>,
    },
    Table {
        align: Vec<Align>,
        header: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    Rule,
    Html(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub kind: BlockKind,
    /// Zero-based source lines covered: `start..end` (end exclusive).
    pub lines: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document {
    pub blocks: Vec<Block>,
}

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
}

/// Renders `source` to a standalone-body HTML fragment (HTML export).
pub fn to_html(source: &str) -> String {
    let mut out = String::new();
    pulldown_cmark::html::push_html(&mut out, Parser::new_ext(source, options()));
    out
}

struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    fn new(source: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(source.match_indices('\n').map(|(i, _)| i + 1));
        Self { starts }
    }

    fn line(&self, byte: usize) -> usize {
        self.starts
            .partition_point(|&s| s <= byte)
            .saturating_sub(1)
    }

    /// Lines of a byte span; a span usually ends right after its final newline.
    fn range(&self, bytes: &Range<usize>) -> Range<usize> {
        let start = self.line(bytes.start);
        let end = self.line(bytes.end.saturating_sub(1).max(bytes.start)) + 1;
        start..end.max(start + 1)
    }
}

type Ev<'a> = (Event<'a>, Range<usize>);

struct Builder<'a> {
    events: Vec<Ev<'a>>,
    pos: usize,
    index: LineIndex,
}

/// Parses `source` into the preview block model. Never panics on any input.
pub fn parse(source: &str) -> Document {
    let mut builder = Builder {
        events: Parser::new_ext(source, options())
            .into_offset_iter()
            .collect(),
        pos: 0,
        index: LineIndex::new(source),
    };
    Document {
        blocks: builder.blocks(),
    }
}

fn is_inline_start(tag: &Tag) -> bool {
    matches!(
        tag,
        Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. } | Tag::Image { .. }
    )
}

impl<'a> Builder<'a> {
    fn peek(&self) -> Option<&Ev<'a>> {
        self.events.get(self.pos)
    }

    /// Blocks up to (and consuming) the container's end event, or to the end of input.
    fn blocks(&mut self) -> Vec<Block> {
        let mut out = Vec::new();
        while let Some((event, span)) = self.peek().cloned() {
            match event {
                Event::End(_) => {
                    self.pos += 1;
                    break;
                }
                Event::Start(tag) if !is_inline_start(&tag) => {
                    self.pos += 1;
                    if let Some(kind) = self.block(tag) {
                        out.push(Block {
                            kind,
                            lines: self.index.range(&span),
                        });
                    }
                }
                Event::Rule => {
                    self.pos += 1;
                    out.push(Block {
                        kind: BlockKind::Rule,
                        lines: self.index.range(&span),
                    });
                }
                Event::Html(html) => {
                    self.pos += 1;
                    out.push(Block {
                        kind: BlockKind::Html(html.into()),
                        lines: self.index.range(&span),
                    });
                }
                Event::TaskListMarker(_) => self.pos += 1,
                _ => {
                    // Inline content without a paragraph: tight list items.
                    let inlines = self.inlines(true);
                    if !inlines.is_empty() {
                        out.push(Block {
                            kind: BlockKind::Paragraph(inlines),
                            lines: self.index.range(&span),
                        });
                    }
                }
            }
        }
        out
    }

    /// The start event is consumed; consumes through the matching end.
    fn block(&mut self, tag: Tag<'a>) -> Option<BlockKind> {
        Some(match tag {
            Tag::Paragraph => BlockKind::Paragraph(self.inlines(false)),
            Tag::Heading { level, .. } => BlockKind::Heading {
                level: match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                },
                inlines: self.inlines(false),
            },
            Tag::BlockQuote(_) => BlockKind::BlockQuote(self.blocks()),
            Tag::CodeBlock(kind) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => info
                        .split_whitespace()
                        .next()
                        .filter(|lang| !lang.is_empty())
                        .map(str::to_owned),
                    CodeBlockKind::Indented => None,
                };
                let mut code = String::new();
                while let Some((event, _)) = self.peek().cloned() {
                    self.pos += 1;
                    match event {
                        Event::End(_) => break,
                        Event::Text(text) => code.push_str(&text),
                        _ => {}
                    }
                }
                BlockKind::CodeBlock { lang, code }
            }
            Tag::HtmlBlock => {
                let mut html = String::new();
                while let Some((event, _)) = self.peek().cloned() {
                    self.pos += 1;
                    match event {
                        Event::End(_) => break,
                        Event::Html(text) | Event::Text(text) => html.push_str(&text),
                        _ => {}
                    }
                }
                BlockKind::Html(html)
            }
            Tag::List(start) => {
                let mut items = Vec::new();
                while let Some((event, _)) = self.peek().cloned() {
                    self.pos += 1;
                    match event {
                        Event::End(_) => break,
                        Event::Start(Tag::Item) => {
                            let task = match self.peek() {
                                Some((Event::TaskListMarker(checked), _)) => Some(*checked),
                                _ => None,
                            };
                            items.push(ListItem {
                                task,
                                blocks: self.blocks(),
                            });
                        }
                        _ => {}
                    }
                }
                BlockKind::List { start, items }
            }
            Tag::Table(aligns) => self.table(&aligns),
            _ => {
                // Unsupported container (footnote definitions, metadata, ...): keep its content.
                let inner = self.blocks();
                return inner.into_iter().next().map(|block| block.kind);
            }
        })
    }

    fn table(&mut self, aligns: &[pulldown_cmark::Alignment]) -> BlockKind {
        let align = aligns
            .iter()
            .map(|a| match a {
                pulldown_cmark::Alignment::None => Align::None,
                pulldown_cmark::Alignment::Left => Align::Left,
                pulldown_cmark::Alignment::Center => Align::Center,
                pulldown_cmark::Alignment::Right => Align::Right,
            })
            .collect();
        let mut header = Vec::new();
        let mut rows = Vec::new();
        let mut current: Vec<Vec<Inline>> = Vec::new();
        while let Some((event, _)) = self.peek().cloned() {
            self.pos += 1;
            match event {
                Event::End(TagEnd::Table) => break,
                Event::End(TagEnd::TableHead) => header = std::mem::take(&mut current),
                Event::End(TagEnd::TableRow) => rows.push(std::mem::take(&mut current)),
                Event::Start(Tag::TableCell) => current.push(self.inlines(false)),
                _ => {}
            }
        }
        BlockKind::Table {
            align,
            header,
            rows,
        }
    }

    /// Inline content up to the container's end (consumed). With `stop_at_block`, stops
    /// before (not consuming) the next block-level start or the end of the parent.
    fn inlines(&mut self, stop_at_block: bool) -> Vec<Inline> {
        let mut out = Vec::new();
        while let Some((event, _)) = self.peek().cloned() {
            match event {
                Event::End(_) => {
                    if !stop_at_block {
                        self.pos += 1;
                    }
                    break;
                }
                Event::Start(ref tag) if !is_inline_start(tag) => break,
                Event::Rule | Event::Html(_) if stop_at_block => break,
                _ => {}
            }
            self.pos += 1;
            match event {
                Event::Text(text) => out.push(Inline::Text(text.into())),
                Event::Code(text) => out.push(Inline::Code(text.into())),
                Event::InlineHtml(html) | Event::Html(html) => out.push(Inline::Html(html.into())),
                Event::SoftBreak => out.push(Inline::SoftBreak),
                Event::HardBreak => out.push(Inline::HardBreak),
                Event::FootnoteReference(label) => out.push(Inline::Text(format!("[{label}]"))),
                Event::Start(Tag::Emphasis) => out.push(Inline::Emphasis(self.inlines(false))),
                Event::Start(Tag::Strong) => out.push(Inline::Strong(self.inlines(false))),
                Event::Start(Tag::Strikethrough) => {
                    out.push(Inline::Strikethrough(self.inlines(false)));
                }
                Event::Start(Tag::Link {
                    dest_url, title, ..
                }) => out.push(Inline::Link {
                    url: dest_url.into(),
                    title: title.into(),
                    children: self.inlines(false),
                }),
                Event::Start(Tag::Image { dest_url, .. }) => {
                    let alt = plain_text(&self.inlines(false));
                    out.push(Inline::Image {
                        url: dest_url.into(),
                        alt,
                    });
                }
                _ => {}
            }
        }
        out
    }
}

/// Concatenated text of inline nodes (image alt text, heading titles).
pub fn plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(t) | Inline::Code(t) => out.push_str(t),
            Inline::Emphasis(c) | Inline::Strong(c) | Inline::Strikethrough(c) => {
                out.push_str(&plain_text(c));
            }
            Inline::Link { children, .. } => out.push_str(&plain_text(children)),
            Inline::Image { alt, .. } => out.push_str(alt),
            Inline::SoftBreak | Inline::HardBreak => out.push(' '),
            Inline::Html(_) => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<BlockKind> {
        parse(source).blocks.into_iter().map(|b| b.kind).collect()
    }

    #[test]
    fn headings_paragraphs_and_inline_styles() {
        let doc = parse("# Title *x*\n\nHello **bold** ~~gone~~ `code` [l](http://a \"t\")\n");
        assert_eq!(doc.blocks.len(), 2);
        assert_eq!(doc.blocks[0].lines, 0..1);
        assert_eq!(doc.blocks[1].lines, 2..3);
        let BlockKind::Heading { level, inlines } = &doc.blocks[0].kind else {
            panic!("heading expected");
        };
        assert_eq!(*level, 1);
        assert_eq!(
            inlines,
            &[
                Inline::Text("Title ".into()),
                Inline::Emphasis(vec![Inline::Text("x".into())])
            ]
        );
        let BlockKind::Paragraph(inlines) = &doc.blocks[1].kind else {
            panic!("paragraph expected");
        };
        assert!(inlines.contains(&Inline::Strong(vec![Inline::Text("bold".into())])));
        assert!(inlines.contains(&Inline::Strikethrough(vec![Inline::Text("gone".into())])));
        assert!(inlines.contains(&Inline::Code("code".into())));
        assert!(inlines.iter().any(|i| matches!(
            i,
            Inline::Link { url, title, .. } if url == "http://a" && title == "t"
        )));
    }

    #[test]
    fn tight_and_task_lists() {
        let doc = parse("- [x] done\n- [ ] todo\n\n3. three\n4. four\n");
        let BlockKind::List { start, items } = &doc.blocks[0].kind else {
            panic!("list expected");
        };
        assert_eq!(*start, None);
        assert_eq!(
            items.iter().map(|i| i.task).collect::<Vec<_>>(),
            [Some(true), Some(false)]
        );
        assert_eq!(
            items[0].blocks[0].kind,
            BlockKind::Paragraph(vec![Inline::Text("done".into())])
        );
        let BlockKind::List { start, items } = &doc.blocks[1].kind else {
            panic!("ordered list expected");
        };
        assert_eq!((*start, items.len()), (Some(3), 2));
    }

    #[test]
    fn code_blocks_keep_language_and_text() {
        let k = kinds("```rust title=x\nfn main() {}\n```\n\n    indented\n");
        assert_eq!(
            k[0],
            BlockKind::CodeBlock {
                lang: Some("rust".into()),
                code: "fn main() {}\n".into()
            }
        );
        assert_eq!(
            k[1],
            BlockKind::CodeBlock {
                lang: None,
                code: "indented\n".into()
            }
        );
    }

    #[test]
    fn tables_with_alignment() {
        let k = kinds("| a | b |\n|:--|--:|\n| 1 | 2 |\n| 3 | 4 |\n");
        let BlockKind::Table {
            align,
            header,
            rows,
        } = &k[0]
        else {
            panic!("table expected");
        };
        assert_eq!(align, &[Align::Left, Align::Right]);
        assert_eq!(header.len(), 2);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1][1], vec![Inline::Text("4".into())]);
    }

    #[test]
    fn blockquotes_images_and_rules() {
        let k = kinds("> quote\n> more\n\n---\n\n![alt text](img/a.png)\n");
        let BlockKind::BlockQuote(inner) = &k[0] else {
            panic!("quote expected");
        };
        assert!(matches!(inner[0].kind, BlockKind::Paragraph(_)));
        assert_eq!(k[1], BlockKind::Rule);
        let BlockKind::Paragraph(inlines) = &k[2] else {
            panic!("paragraph expected");
        };
        assert_eq!(
            inlines,
            &[Inline::Image {
                url: "img/a.png".into(),
                alt: "alt text".into()
            }]
        );
    }

    #[test]
    fn unicode_and_garbage_input_do_not_panic() {
        for source in [
            "",
            "\n\n",
            "# ünï 🎉\n\n日本語 *テスト*",
            "[[[",
            "| a |\n|-",
            "```",
            "<div>\n",
        ] {
            let _ = parse(source);
        }
        let doc = parse("# ünï 🎉\n\n日本語\n");
        assert_eq!(doc.blocks[1].lines, 2..3);
    }

    #[test]
    fn html_export_renders_gfm() {
        let html = to_html("~~x~~\n\n- [x] t\n");
        assert!(html.contains("<del>x</del>"));
        assert!(html.contains("checked"));
    }
}
