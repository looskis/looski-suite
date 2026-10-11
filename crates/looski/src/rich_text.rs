// SPDX-License-Identifier: AGPL-3.0-only
//! Rich text is plain source in a small Markdown subset, parsed into typed blocks.
//! No HTML passes through: any markup in the source is literal text, and renderers
//! receive only these structures. Links are allow-listed here, not by the renderer.
//! This is a pure parser with no application state: CMS, Projects and the knowledge
//! base each use it whether or not the others are enabled.
use crate::application::OperationError as Error;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RichInline {
    pub text: String,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub code: bool,
    /// Always `http:`, `https:`, `mailto:` or a same-site path.
    pub href: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RichListItem {
    pub inlines: Vec<RichInline>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RichBlock {
    Heading { level: u8, inlines: Vec<RichInline> },
    Paragraph { inlines: Vec<RichInline> },
    Quote { inlines: Vec<RichInline> },
    BulletList { items: Vec<RichListItem> },
    OrderedList { items: Vec<RichListItem> },
}
/// Allow-list rather than deny-list: anything a browser might normalize into
/// another scheme (`java\tscript:`, `//host`, `/\host`) fails every branch.
pub(crate) fn safe_href(href: &str) -> bool {
    if href.is_empty()
        || href.len() > 2000
        || href
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '\\')
    {
        return false;
    }
    let lower = href.to_ascii_lowercase();
    (lower.starts_with("https://") && lower.len() > 8)
        || (lower.starts_with("http://") && lower.len() > 7)
        || (lower.starts_with("mailto:") && lower.len() > 7)
        || (href.starts_with('/') && !href.starts_with("//"))
}
#[derive(Default)]
struct Inlines {
    out: Vec<RichInline>,
    text: String,
    bold: bool,
    italic: bool,
    unsafe_links: usize,
}
impl Inlines {
    fn flush(&mut self) {
        if !self.text.is_empty() {
            self.out.push(RichInline {
                text: std::mem::take(&mut self.text),
                bold: self.bold,
                italic: self.italic,
                ..Default::default()
            });
        }
    }
}
/// `absent_from` remembers a failed search: nothing later can match either, so a long
/// run of unmatched markers stays linear rather than rescanning the text for each one.
fn find(chars: &[char], from: usize, needle: &[char], absent_from: &mut usize) -> Option<usize> {
    if from >= *absent_from {
        return None;
    }
    let found = (from..chars.len().saturating_sub(needle.len() - 1))
        .find(|&i| chars[i..].starts_with(needle));
    if found.is_none() {
        *absent_from = from;
    }
    found
}
/// A marker opens before text with a closer ahead, and closes after text; a `*`
/// surrounded by spaces, as in `2 * 3`, is an ordinary character.
fn emphasis_toggles(
    chars: &[char],
    at: usize,
    width: usize,
    open: bool,
    no_closer_from: &mut usize,
) -> bool {
    let marker = |i: usize| chars[i] == '*';
    let closes = |i: usize| {
        i > 0
            && !chars[i - 1].is_whitespace()
            && (0..width).all(|n| chars.get(i + n) == Some(&'*'))
            && (width == 2 || (!marker(i - 1) && chars.get(i + 1) != Some(&'*')))
    };
    if open {
        return closes(at);
    }
    if !chars
        .get(at + width)
        .is_some_and(|next| !next.is_whitespace() && *next != '*')
    {
        return false;
    }
    let from = at + width + 1;
    if from >= *no_closer_from {
        return false;
    }
    let found = (from..chars.len()).any(closes);
    if !found {
        *no_closer_from = from;
    }
    found
}
fn inlines(source: &str, unsafe_links: &mut usize) -> Vec<RichInline> {
    let chars: Vec<char> = source.chars().collect();
    let mut state = Inlines::default();
    let mut i = 0;
    let [mut no_tick, mut no_link_middle, mut no_link_end] = [usize::MAX; 3];
    let [mut no_bold_closer, mut no_italic_closer] = [usize::MAX; 2];
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && chars.get(i + 1).is_some_and(|n| n.is_ascii_punctuation()) {
            state.text.push(chars[i + 1]);
            i += 2;
        } else if c == '`'
            && let Some(end) = find(&chars, i + 1, &['`'], &mut no_tick)
            && end > i + 1
        {
            state.flush();
            state.out.push(RichInline {
                text: chars[i + 1..end].iter().collect(),
                code: true,
                ..Default::default()
            });
            i = end + 1;
        } else if chars[i..].starts_with(&['*', '*'])
            && emphasis_toggles(&chars, i, 2, state.bold, &mut no_bold_closer)
        {
            state.flush();
            state.bold = !state.bold;
            i += 2;
        } else if c == '*' && emphasis_toggles(&chars, i, 1, state.italic, &mut no_italic_closer) {
            state.flush();
            state.italic = !state.italic;
            i += 1;
        } else if c == '['
            && let Some(middle) = find(&chars, i + 1, &[']', '('], &mut no_link_middle)
            && let Some(end) = find(&chars, middle + 2, &[')'], &mut no_link_end)
            && middle > i + 1
        {
            state.flush();
            let text: String = chars[i + 1..middle].iter().collect();
            let href: String = chars[middle + 2..end].iter().collect();
            let safe = safe_href(&href);
            if !safe {
                state.unsafe_links += 1;
            }
            state.out.push(RichInline {
                text,
                bold: state.bold,
                italic: state.italic,
                code: false,
                href: safe.then_some(href),
            });
            i = end + 1;
        } else {
            state.text.push(c);
            i += 1;
        }
    }
    state.flush();
    *unsafe_links += state.unsafe_links;
    state.out
}
fn ordered_item(line: &str) -> Option<&str> {
    let (number, rest) = line.split_once(". ")?;
    (!number.is_empty() && number.len() <= 9 && number.bytes().all(|b| b.is_ascii_digit()))
        .then_some(rest)
}
fn parse_counting(source: &str, unsafe_links: &mut usize) -> Vec<RichBlock> {
    let mut blocks = Vec::new();
    let mut lines = source.lines().map(str::trim_end).peekable();
    while let Some(line) = lines.next() {
        if line.trim().is_empty() {
            continue;
        }
        let heading = line.chars().take_while(|c| *c == '#').count();
        if (1..=3).contains(&heading) && line[heading..].starts_with(' ') {
            blocks.push(RichBlock::Heading {
                level: heading as u8,
                inlines: inlines(line[heading..].trim(), unsafe_links),
            });
        } else if let Some(first) = line.strip_prefix("- ") {
            let mut items = vec![RichListItem {
                inlines: inlines(first.trim(), unsafe_links),
            }];
            while let Some(next) = lines.peek().and_then(|l| l.strip_prefix("- ")) {
                items.push(RichListItem {
                    inlines: inlines(next.trim(), unsafe_links),
                });
                lines.next();
            }
            blocks.push(RichBlock::BulletList { items });
        } else if let Some(first) = ordered_item(line) {
            let mut items = vec![RichListItem {
                inlines: inlines(first.trim(), unsafe_links),
            }];
            while let Some(next) = lines.peek().and_then(|l| ordered_item(l)) {
                items.push(RichListItem {
                    inlines: inlines(next.trim(), unsafe_links),
                });
                lines.next();
            }
            blocks.push(RichBlock::OrderedList { items });
        } else if let Some(first) = line.strip_prefix("> ") {
            let mut quote = first.trim().to_string();
            while let Some(next) = lines.peek().and_then(|l| l.strip_prefix("> ")) {
                quote.push(' ');
                quote.push_str(next.trim());
                lines.next();
            }
            blocks.push(RichBlock::Quote {
                inlines: inlines(&quote, unsafe_links),
            });
        } else {
            let mut paragraph = line.trim().to_string();
            while let Some(next) = lines.peek().copied() {
                let block_start = next.trim().is_empty()
                    || next.starts_with("- ")
                    || next.starts_with("> ")
                    || next.starts_with('#')
                    || ordered_item(next).is_some();
                if block_start {
                    break;
                }
                paragraph.push(' ');
                paragraph.push_str(next.trim());
                lines.next();
            }
            blocks.push(RichBlock::Paragraph {
                inlines: inlines(&paragraph, unsafe_links),
            });
        }
    }
    blocks
}
/// Blocks for rendering. A link that fails the allow-list keeps its text only.
pub fn parse(source: &str) -> Vec<RichBlock> {
    parse_counting(source, &mut 0)
}
fn inline_words(items: &[RichInline], out: &mut String) {
    for item in items {
        out.push_str(&item.text);
    }
    out.push('\n');
}
/// The words a reader sees, for search and excerpts: no markup and no link targets.
pub(crate) fn plain_text(blocks: &[RichBlock]) -> String {
    let mut out = String::new();
    for block in blocks {
        match block {
            RichBlock::Heading { inlines, .. }
            | RichBlock::Paragraph { inlines }
            | RichBlock::Quote { inlines } => inline_words(inlines, &mut out),
            RichBlock::BulletList { items } | RichBlock::OrderedList { items } => {
                for item in items {
                    inline_words(&item.inlines, &mut out);
                }
            }
        }
    }
    out
}
/// Every link target that passed the allow-list, in reading order.
pub(crate) fn hrefs(blocks: &[RichBlock]) -> Vec<&str> {
    fn from<'a>(items: &'a [RichInline], out: &mut Vec<&'a str>) {
        out.extend(items.iter().filter_map(|i| i.href.as_deref()));
    }
    let mut out = Vec::new();
    for block in blocks {
        match block {
            RichBlock::Heading { inlines, .. }
            | RichBlock::Paragraph { inlines }
            | RichBlock::Quote { inlines } => from(inlines, &mut out),
            RichBlock::BulletList { items } | RichBlock::OrderedList { items } => {
                for item in items {
                    from(&item.inlines, &mut out);
                }
            }
        }
    }
    out
}
/// Refuse unsafe links when text is saved, so authors see why instead of a dead link.
pub(crate) fn validate(source: &str) -> Result<(), Error> {
    let mut unsafe_links = 0;
    parse_counting(source, &mut unsafe_links);
    if unsafe_links > 0 {
        return Err(Error::Invalid(
            "Links must start with https://, http://, mailto: or a single /",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plain_text_and_hrefs_leave_out_each_other() {
        let blocks = parse(
            "# Head\n\nSee [the guide](/a/b) and **bold** `<b>`\n\n- [x](https://e.example)\n",
        );
        let plain = plain_text(&blocks);
        assert_eq!(plain, "Head\nSee the guide and bold <b>\nx\n");
        assert_eq!(hrefs(&blocks), vec!["/a/b", "https://e.example"]);
    }
    #[test]
    fn pathological_source_parses_quickly() {
        for unit in ["*a ", "**a ", "[a ", "`", "[a](", "\\*"] {
            let source = unit.repeat(100_000 / unit.len());
            let started = std::time::Instant::now();
            parse(&source);
            assert!(
                started.elapsed().as_secs() < 5,
                "{unit} took {:?}",
                started.elapsed()
            );
        }
    }
    fn text(block: &RichBlock) -> String {
        match block {
            RichBlock::Heading { inlines, .. }
            | RichBlock::Paragraph { inlines }
            | RichBlock::Quote { inlines } => inlines.iter().map(|i| i.text.as_str()).collect(),
            _ => panic!("list"),
        }
    }
    #[test]
    fn parses_the_supported_blocks() {
        let blocks =
            parse("# One\n\nSome **bold** and *it*\nwrapped\n\n- a\n- b\n\n1. x\n2. y\n\n> q");
        assert!(matches!(blocks[0], RichBlock::Heading { level: 1, .. }));
        assert_eq!(text(&blocks[1]), "Some bold and it wrapped");
        let RichBlock::Paragraph { inlines } = &blocks[1] else {
            panic!()
        };
        assert!(inlines[1].bold && !inlines[0].bold && inlines[3].italic);
        assert!(matches!(&blocks[2], RichBlock::BulletList { items } if items.len() == 2));
        assert!(matches!(&blocks[3], RichBlock::OrderedList { items } if items.len() == 2));
        assert!(matches!(&blocks[4], RichBlock::Quote { .. }));
    }
    #[test]
    fn markup_stays_literal_text() {
        for payload in [
            "<script>alert(1)</script>",
            "<img src=x onerror=alert(1)>",
            "<a href=\"javascript:alert(1)\">x</a>",
        ] {
            let blocks = parse(payload);
            assert_eq!(text(&blocks[0]), payload);
            let RichBlock::Paragraph { inlines } = &blocks[0] else {
                panic!()
            };
            assert!(inlines.iter().all(|i| i.href.is_none()));
        }
    }
    #[test]
    fn links_are_allow_listed() {
        for good in [
            "https://example.com/a?b=c",
            "http://example.com",
            "HTTPS://example.com",
            "mailto:a@example.com",
            "/c/page",
        ] {
            assert!(safe_href(good), "{good}");
            assert!(validate(&format!("[x]({good})")).is_ok());
        }
        for bad in [
            "javascript:alert(1)",
            "JaVaScRiPt:alert(1)",
            "java\tscript:alert(1)",
            " javascript:alert(1)",
            "data:text/html,<script>alert(1)</script>",
            "vbscript:x",
            "//evil.example",
            "/\\evil.example",
            "https://",
            "page",
            "",
        ] {
            assert!(!safe_href(bad), "{bad:?}");
        }
        for bad in ["[x](javascript:alert(1))", "[x](data:text/html,y)"] {
            assert!(validate(bad).is_err());
            let RichBlock::Paragraph { inlines } = &parse(bad)[0] else {
                panic!()
            };
            assert!(inlines.iter().all(|i| i.href.is_none()));
        }
    }
    #[test]
    fn unmatched_markers_and_escapes_are_text() {
        assert_eq!(text(&parse("2 * 3 and ** and `")[0]), "2 * 3 and ** and `");
        assert_eq!(text(&parse("\\*not\\* \\[x\\](y)")[0]), "*not* [x](y)");
        assert!(parse(" \n\n ").is_empty());
    }
    #[test]
    fn non_ascii_text_does_not_split_characters() {
        assert_eq!(text(&parse("# 雪 **é**")[0]), "雪 é");
        assert_eq!(text(&parse("雪 [é](/a) `雪`")[0]), "雪 é 雪");
    }
}
