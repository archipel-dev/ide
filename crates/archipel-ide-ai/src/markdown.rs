//! Markdown answers reduced to a small block model the UI can paint.
//!
//! Parsing uses `pulldown-cmark`; partial Markdown (a streaming answer with an
//! unterminated code fence) parses as if the fence were closed.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpanStyle {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub link: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub style: SpanStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MdBlock {
    Heading {
        level: u8,
        spans: Vec<Span>,
    },
    Paragraph(Vec<Span>),
    /// `marker` is `•` or `N.`; `depth` starts at 0.
    ListItem {
        depth: usize,
        marker: String,
        spans: Vec<Span>,
    },
    Quote(Vec<Span>),
    Code(CodeBlock),
    Rule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    /// Info string language (`rust`, `bash`, …), empty when absent.
    pub language: String,
    pub code: String,
}

pub fn parse(markdown: &str) -> Vec<MdBlock> {
    let mut blocks = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut style = SpanStyle::default();
    // Stack of list counters: `None` for bullets, `Some(n)` for the next ordinal.
    let mut lists: Vec<Option<u64>> = Vec::new();
    let mut item_marker: Option<(usize, String)> = None;
    let mut heading: Option<u8> = None;
    let mut quote = 0usize;
    let mut code: Option<CodeBlock> = None;

    let flush = |spans: &mut Vec<Span>,
                 blocks: &mut Vec<MdBlock>,
                 heading: Option<u8>,
                 item: &mut Option<(usize, String)>,
                 quote: usize| {
        if spans.iter().all(|span| span.text.trim().is_empty()) && item.is_none() {
            spans.clear();
            return;
        }
        let taken = std::mem::take(spans);
        blocks.push(if let Some(level) = heading {
            MdBlock::Heading {
                level,
                spans: taken,
            }
        } else if let Some((depth, marker)) = item.take() {
            MdBlock::ListItem {
                depth,
                marker,
                spans: taken,
            }
        } else if quote > 0 {
            MdBlock::Quote(taken)
        } else {
            MdBlock::Paragraph(taken)
        });
    };

    for event in Parser::new_ext(
        markdown,
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES,
    ) {
        if let Some(block) = code.as_mut() {
            match event {
                Event::Text(text) => block.code.push_str(&text),
                Event::End(TagEnd::CodeBlock) => {
                    let mut block = code.take().unwrap_or_else(|| unreachable!());
                    if block.code.ends_with('\n') {
                        block.code.pop();
                    }
                    blocks.push(MdBlock::Code(block));
                }
                _ => {}
            }
            continue;
        }
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some(match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                });
            }
            Event::End(TagEnd::Heading(_)) => {
                flush(&mut spans, &mut blocks, heading, &mut item_marker, quote);
                heading = None;
            }
            Event::End(TagEnd::Paragraph)
            | Event::End(TagEnd::TableRow)
            | Event::End(TagEnd::TableHead) => {
                flush(&mut spans, &mut blocks, None, &mut item_marker, quote);
            }
            Event::Start(Tag::List(start)) => {
                flush(&mut spans, &mut blocks, None, &mut item_marker, quote);
                lists.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                lists.pop();
            }
            Event::Start(Tag::Item) => {
                flush(&mut spans, &mut blocks, None, &mut item_marker, quote);
                let depth = lists.len().saturating_sub(1);
                let marker = match lists.last_mut() {
                    Some(Some(next)) => {
                        let marker = format!("{next}.");
                        *next += 1;
                        marker
                    }
                    _ => "•".to_owned(),
                };
                item_marker = Some((depth, marker));
            }
            Event::End(TagEnd::Item) => {
                if item_marker.is_some() || !spans.is_empty() {
                    flush(&mut spans, &mut blocks, None, &mut item_marker, quote);
                }
            }
            Event::Start(Tag::BlockQuote(_)) => quote += 1,
            Event::End(TagEnd::BlockQuote(_)) => {
                flush(&mut spans, &mut blocks, None, &mut item_marker, quote);
                quote = quote.saturating_sub(1);
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush(&mut spans, &mut blocks, heading, &mut item_marker, quote);
                let language = match kind {
                    CodeBlockKind::Fenced(info) => info
                        .split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_owned(),
                    CodeBlockKind::Indented => String::new(),
                };
                code = Some(CodeBlock {
                    language,
                    code: String::new(),
                });
            }
            Event::Start(Tag::Strong) => style.bold = true,
            Event::End(TagEnd::Strong) => style.bold = false,
            Event::Start(Tag::Emphasis) => style.italic = true,
            Event::End(TagEnd::Emphasis) => style.italic = false,
            Event::Start(Tag::Link { .. }) => style.link = true,
            Event::End(TagEnd::Link) => style.link = false,
            Event::Text(text) => spans.push(Span {
                text: text.into_string(),
                style,
            }),
            Event::Code(text) => spans.push(Span {
                text: text.into_string(),
                style: SpanStyle {
                    code: true,
                    ..style
                },
            }),
            Event::SoftBreak => spans.push(Span {
                text: " ".into(),
                style,
            }),
            Event::HardBreak => spans.push(Span {
                text: "\n".into(),
                style,
            }),
            Event::End(TagEnd::TableCell) => spans.push(Span {
                text: "  │  ".into(),
                style: SpanStyle::default(),
            }),
            Event::Rule => {
                flush(&mut spans, &mut blocks, None, &mut item_marker, quote);
                blocks.push(MdBlock::Rule);
            }
            _ => {}
        }
    }
    if let Some(block) = code.take() {
        blocks.push(MdBlock::Code(block));
    }
    flush(&mut spans, &mut blocks, heading, &mut item_marker, quote);
    blocks
}

/// Fenced code blocks of an answer, in order.
pub fn code_blocks(markdown: &str) -> Vec<CodeBlock> {
    parse(markdown)
        .into_iter()
        .filter_map(|block| match block {
            MdBlock::Code(code) => Some(code),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(spans: &[Span]) -> String {
        spans.iter().map(|span| span.text.as_str()).collect()
    }

    #[test]
    fn parses_headings_lists_inline_styles_and_code() {
        let blocks = parse(
            "## Fix\nUse `Vec::new` **now**.\n\n1. one\n2. two\n   - nested\n\n```rust\nfn main() {}\n```\n---\n",
        );
        assert!(
            matches!(&blocks[0], MdBlock::Heading { level: 2, spans } if plain(spans) == "Fix")
        );
        let MdBlock::Paragraph(spans) = &blocks[1] else {
            panic!("{blocks:?}")
        };
        assert_eq!(plain(spans), "Use Vec::new now.");
        assert!(spans[1].style.code && spans[3].style.bold);
        assert!(
            matches!(&blocks[2], MdBlock::ListItem { depth: 0, marker, spans } if marker == "1." && plain(spans) == "one")
        );
        assert!(matches!(&blocks[3], MdBlock::ListItem { marker, .. } if marker == "2."));
        assert!(matches!(&blocks[4], MdBlock::ListItem { depth: 1, marker, .. } if marker == "•"));
        assert_eq!(
            blocks[5],
            MdBlock::Code(CodeBlock {
                language: "rust".into(),
                code: "fn main() {}".into()
            })
        );
        assert_eq!(blocks[6], MdBlock::Rule);
    }

    #[test]
    fn unterminated_fence_while_streaming_is_code() {
        let blocks = code_blocks("Here:\n```py\nprint(1)\nprint(");
        assert_eq!(blocks[0].language, "py");
        assert_eq!(blocks[0].code, "print(1)\nprint(");
    }
}
