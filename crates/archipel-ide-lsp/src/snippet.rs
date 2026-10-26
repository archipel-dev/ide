//! LSP snippet syntax (`insertTextFormat: 2`).
//!
//! Supports tab stops (`$1`, `${1}`), placeholders (`${1:default}`, nested),
//! choices (`${1|a,b|}`, the first choice is inserted), variables (`$TM_FILENAME`,
//! `${CLIPBOARD:default}`) and `\` escapes. Unknown or malformed syntax is
//! kept as literal text rather than rejected.

use std::collections::BTreeMap;

/// Text with tab stops expanded.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Snippet {
    pub text: String,
    /// Tab stops in visiting order (1, 2, …, then 0). Each stop lists the
    /// char ranges (relative to `text`) of all its occurrences; mirrors of the
    /// same number are edited together.
    pub stops: Vec<Vec<(usize, usize)>>,
    /// Whether the snippet declared `$0`; otherwise the caret ends after the text.
    pub has_final_stop: bool,
}

impl Snippet {
    /// A snippet with tab stops other than `$0` needs an interactive session.
    pub fn is_interactive(&self) -> bool {
        self.stops.len() > usize::from(self.has_final_stop)
    }

    /// Caret offset (chars) after insertion when no session is started.
    pub fn final_caret(&self) -> usize {
        if self.has_final_stop
            && let Some(&(start, _)) = self.stops.last().and_then(|stop| stop.first())
        {
            return start;
        }
        self.text.chars().count()
    }
}

/// Resolves `$NAME` variables.
pub trait SnippetVariables {
    fn resolve(&self, name: &str) -> Option<String>;
}

impl SnippetVariables for () {
    fn resolve(&self, _: &str) -> Option<String> {
        None
    }
}

impl<F: Fn(&str) -> Option<String>> SnippetVariables for F {
    fn resolve(&self, name: &str) -> Option<String> {
        self(name)
    }
}

/// Parses `snippet`; never fails.
pub fn parse(snippet: &str, variables: &dyn SnippetVariables) -> Snippet {
    let chars: Vec<char> = snippet.chars().collect();
    // First pass: learn each stop's placeholder text so that bare mirrors
    // (`$1` after `${1:name}`) repeat it.
    let mut first = Parser::new(&chars, variables, BTreeMap::new());
    first.parse_until(None);
    let mut parser = Parser::new(&chars, variables, first.defaults);
    parser.parse_until(None);
    let has_final_stop = parser.stops.contains_key(&0);
    let mut stops: Vec<Vec<(usize, usize)>> = parser
        .stops
        .iter()
        .filter(|(number, _)| **number != 0)
        .map(|(_, ranges)| ranges.clone())
        .collect();
    if let Some(final_stop) = parser.stops.get(&0) {
        stops.push(final_stop.clone());
    }
    Snippet {
        text: parser.out,
        stops,
        has_final_stop,
    }
}

/// Plain text of a snippet (for display or when snippets are not wanted).
pub fn to_plain_text(snippet: &str) -> String {
    parse(snippet, &()).text
}

struct Parser<'a> {
    chars: &'a [char],
    index: usize,
    out: String,
    /// `out` length in chars.
    out_len: usize,
    stops: BTreeMap<u32, Vec<(usize, usize)>>,
    variables: &'a dyn SnippetVariables,
    /// Placeholder text per stop number.
    defaults: BTreeMap<u32, String>,
}

impl<'a> Parser<'a> {
    fn new(
        chars: &'a [char],
        variables: &'a dyn SnippetVariables,
        defaults: BTreeMap<u32, String>,
    ) -> Self {
        Self {
            chars,
            index: 0,
            out: String::new(),
            out_len: 0,
            stops: BTreeMap::new(),
            variables,
            defaults,
        }
    }

    /// Inserts the placeholder text of `number` at a bare mirror.
    fn mirror(&mut self, number: u32) {
        let start = self.out_len;
        if let Some(text) = self.defaults.get(&number).cloned() {
            self.push_str(&text);
        }
        self.add_stop(number, start);
    }

    /// Records the text a placeholder produced (first definition wins).
    fn define(&mut self, number: u32, start: usize) {
        let text: String = self.out.chars().skip(start).collect();
        self.defaults.entry(number).or_insert(text);
        self.add_stop(number, start);
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn push(&mut self, c: char) {
        self.out.push(c);
        self.out_len += 1;
    }

    fn push_str(&mut self, text: &str) {
        self.out.push_str(text);
        self.out_len += text.chars().count();
    }

    /// Parses until `terminator` (unescaped) or the end; consumes the terminator.
    fn parse_until(&mut self, terminator: Option<char>) {
        while let Some(c) = self.peek() {
            if Some(c) == terminator {
                self.index += 1;
                return;
            }
            match c {
                '\\' => {
                    let next = self.chars.get(self.index + 1).copied();
                    match next {
                        Some(escaped @ ('$' | '}' | '\\' | ',' | '|')) => {
                            self.push(escaped);
                            self.index += 2;
                        }
                        _ => {
                            self.push('\\');
                            self.index += 1;
                        }
                    }
                }
                '$' => self.parse_dollar(),
                _ => {
                    self.push(c);
                    self.index += 1;
                }
            }
        }
    }

    fn read_number(&mut self) -> Option<u32> {
        let start = self.index;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.index += 1;
        }
        if start == self.index {
            return None;
        }
        self.chars[start..self.index]
            .iter()
            .collect::<String>()
            .parse()
            .ok()
    }

    fn read_name(&mut self) -> Option<String> {
        let start = self.index;
        if !self
            .peek()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        {
            return None;
        }
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.index += 1;
        }
        Some(self.chars[start..self.index].iter().collect())
    }

    fn add_stop(&mut self, number: u32, start: usize) {
        self.stops
            .entry(number)
            .or_default()
            .push((start, self.out_len));
    }

    fn parse_dollar(&mut self) {
        let dollar = self.index;
        self.index += 1;
        // $1
        if let Some(number) = self.read_number() {
            self.mirror(number);
            return;
        }
        // $NAME
        if let Some(name) = self.read_name() {
            let value = self.variables.resolve(&name).unwrap_or_default();
            self.push_str(&value);
            return;
        }
        if self.peek() != Some('{') {
            self.push('$');
            return;
        }
        self.index += 1;
        if let Some(number) = self.read_number() {
            let start = self.out_len;
            match self.peek() {
                Some('}') => {
                    self.index += 1;
                    self.mirror(number);
                }
                Some(':') => {
                    self.index += 1;
                    self.parse_until(Some('}'));
                    self.define(number, start);
                }
                Some('|') => {
                    self.index += 1;
                    let mut choice = String::new();
                    let mut first = true;
                    while let Some(c) = self.peek() {
                        self.index += 1;
                        match c {
                            '\\' => {
                                if let Some(escaped) = self.peek() {
                                    self.index += 1;
                                    if first {
                                        choice.push(escaped);
                                    }
                                }
                            }
                            ',' => first = false,
                            '|' => break,
                            _ if first => choice.push(c),
                            _ => {}
                        }
                    }
                    if self.peek() == Some('}') {
                        self.index += 1;
                    }
                    self.push_str(&choice);
                    self.define(number, start);
                }
                _ => self.literal_from(dollar),
            }
            return;
        }
        if let Some(name) = self.read_name() {
            match self.peek() {
                Some('}') => {
                    self.index += 1;
                    let value = self.variables.resolve(&name).unwrap_or_default();
                    self.push_str(&value);
                }
                Some(':') => {
                    self.index += 1;
                    match self.variables.resolve(&name) {
                        Some(value) => {
                            self.push_str(&value);
                            // Skip the default.
                            let mut depth = 1;
                            while let Some(c) = self.peek() {
                                self.index += 1;
                                match c {
                                    '\\' => self.index += 1,
                                    '{' => depth += 1,
                                    '}' => {
                                        depth -= 1;
                                        if depth == 0 {
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        None => self.parse_until(Some('}')),
                    }
                }
                // Transforms (`${NAME/regex/format/}`) are not supported; keep text.
                _ => self.literal_from(dollar),
            }
            return;
        }
        self.literal_from(dollar);
    }

    /// Emits `chars[from..]` up to the current index as literal text.
    fn literal_from(&mut self, from: usize) {
        let text: String = self.chars[from..self.index].iter().collect();
        self.push_str(&text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_tab_stops_placeholders_and_final_caret() {
        let snippet = parse("greeting(${1:name})$0", &());
        assert_eq!(snippet.text, "greeting(name)");
        assert_eq!(snippet.stops, [vec![(9, 13)], vec![(14, 14)]]);
        assert!(snippet.has_final_stop);
        assert!(snippet.is_interactive());

        let call = parse("push_str($0)", &());
        assert_eq!(call.text, "push_str()");
        assert_eq!(call.final_caret(), 9);
        assert!(!call.is_interactive());

        assert_eq!(parse("plain", &()).final_caret(), 5);
    }

    #[test]
    fn orders_stops_numerically_with_zero_last_and_keeps_mirrors() {
        let snippet = parse("for ${2:i} in ${1:iter} { $2 }$0", &());
        assert_eq!(snippet.text, "for i in iter { i }");
        assert_eq!(
            snippet.stops,
            [vec![(9, 13)], vec![(4, 5), (16, 17)], vec![(19, 19)]]
        );
        // A mirror before its definition also repeats the placeholder.
        assert_eq!(parse("$1 = ${1:x}", &()).text, "x = x");
    }

    #[test]
    fn supports_nested_placeholders_choices_variables_and_escapes() {
        let nested = parse("${1:outer ${2:inner}}", &());
        assert_eq!(nested.text, "outer inner");
        assert_eq!(nested.stops, [vec![(0, 11)], vec![(6, 11)]]);

        let choice = parse("${1|public,private|} x", &());
        assert_eq!(choice.text, "public x");
        assert_eq!(choice.stops, [vec![(0, 6)]]);

        let vars = |name: &str| (name == "TM_FILENAME").then(|| "App.java".to_string());
        assert_eq!(
            parse("// $TM_FILENAME ${UNKNOWN:fallback}", &vars).text,
            "// App.java fallback"
        );
        assert_eq!(parse("${TM_FILENAME:x}", &vars).text, "App.java");
        assert_eq!(parse(r"cost: \$5 \} \\", &()).text, r"cost: $5 } \");
    }

    #[test]
    fn malformed_snippets_degrade_to_text() {
        assert_eq!(parse("$", &()).text, "$");
        assert_eq!(parse("${", &()).text, "${");
        assert_eq!(parse("${1:unterminated", &()).text, "unterminated");
        assert_eq!(parse("${x/a/b/}", &()).text, "${x/a/b/}");
        assert_eq!(parse("a$ b", &()).text, "a$ b");
        assert_eq!(to_plain_text("é${1:ü}"), "éü");
        assert_eq!(parse("é${1:ü}", &()).stops, [vec![(1, 2)]]);
    }
}
