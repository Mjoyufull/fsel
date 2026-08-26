//! Lightweight HTML-to-text conversion for clipboard display.

const HIDDEN_ELEMENTS: &[&str] = &["head", "noscript", "script", "style", "svg", "template"];
const BOUNDARY_ELEMENTS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "br",
    "dd",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "li",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "table",
    "td",
    "th",
    "tr",
    "ul",
];

pub(crate) fn is_html_mime(mime_type: &str) -> bool {
    let essence = mime_type.split(';').next().unwrap_or(mime_type).trim();
    essence.eq_ignore_ascii_case("text/html")
        || essence.eq_ignore_ascii_case("application/xhtml+xml")
}

pub(crate) fn text_for_display(mime_type: &str, content: &str) -> String {
    if is_html_mime(mime_type) {
        to_plain_text(content)
    } else {
        content.to_string()
    }
}

fn to_plain_text(html: &str) -> String {
    let mut renderer = TextRenderer::default();
    let mut cursor = 0;

    while cursor < html.len() {
        let remaining = &html[cursor..];
        if remaining.starts_with("<!--") {
            cursor += comment_len(remaining);
        } else if remaining.starts_with('<') && looks_like_tag(remaining) {
            if let Some(tag_len) = tag_len(remaining) {
                renderer.push_tag(&remaining[1..tag_len - 1]);
                cursor += tag_len;
            } else {
                renderer.push_text("<");
                cursor += 1;
            }
        } else {
            let text_len = remaining.find('<').unwrap_or(remaining.len()).max(1);
            renderer.push_text(&remaining[..text_len]);
            cursor += text_len;
        }
    }

    renderer.finish()
}

#[derive(Default)]
struct TextRenderer {
    output: String,
    hidden_depth: usize,
    pending_space: bool,
}

impl TextRenderer {
    fn push_tag(&mut self, raw_tag: &str) {
        let tag = Tag::parse(raw_tag);
        if tag.name.is_empty() {
            return;
        }

        if tag.closing && is_hidden_element(tag.name) {
            self.hidden_depth = self.hidden_depth.saturating_sub(1);
        }

        if self.hidden_depth == 0 && is_boundary_element(tag.name) {
            self.pending_space = !self.output.is_empty();
        }

        if !tag.closing && !tag.self_closing && is_hidden_element(tag.name) {
            self.hidden_depth += 1;
        }
    }

    fn push_text(&mut self, text: &str) {
        if self.hidden_depth > 0 {
            return;
        }

        let mut cursor = 0;
        while cursor < text.len() {
            let remaining = &text[cursor..];
            if remaining.starts_with('&')
                && let Some((decoded, consumed)) = decode_entity(remaining)
            {
                self.push_decoded(&decoded);
                cursor += consumed;
                continue;
            }

            let ch = remaining
                .chars()
                .next()
                .expect("remaining text is not empty");
            self.push_char(ch);
            cursor += ch.len_utf8();
        }
    }

    fn push_decoded(&mut self, decoded: &str) {
        for ch in decoded.chars() {
            self.push_char(ch);
        }
    }

    fn push_char(&mut self, ch: char) {
        if ch.is_whitespace() {
            self.pending_space = !self.output.is_empty();
            return;
        }

        if self.pending_space {
            self.output.push(' ');
            self.pending_space = false;
        }
        self.output.push(ch);
    }

    fn finish(self) -> String {
        self.output
    }
}

struct Tag<'a> {
    name: &'a str,
    closing: bool,
    self_closing: bool,
}

impl<'a> Tag<'a> {
    fn parse(raw_tag: &'a str) -> Self {
        let trimmed = raw_tag.trim();
        let closing = trimmed.starts_with('/');
        let body = trimmed.trim_start_matches('/').trim_start();
        let name_len = body
            .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != ':')
            .unwrap_or(body.len());

        Self {
            name: &body[..name_len],
            closing,
            self_closing: body.trim_end().ends_with('/'),
        }
    }
}

fn looks_like_tag(remaining: &str) -> bool {
    matches!(
        remaining.as_bytes().get(1),
        Some(b'!' | b'/' | b'?' | b'A'..=b'Z' | b'a'..=b'z')
    )
}

fn comment_len(remaining: &str) -> usize {
    remaining
        .find("-->")
        .map_or(remaining.len(), |end| end + "-->".len())
}

fn tag_len(remaining: &str) -> Option<usize> {
    let mut quote = None;
    for (offset, ch) in remaining.char_indices().skip(1) {
        match (quote, ch) {
            (Some(opening), closing) if opening == closing => quote = None,
            (None, '"' | '\'') => quote = Some(ch),
            (None, '>') => return Some(offset + ch.len_utf8()),
            _ => {}
        }
    }
    None
}

fn is_hidden_element(name: &str) -> bool {
    HIDDEN_ELEMENTS
        .iter()
        .any(|element| name.eq_ignore_ascii_case(element))
}

fn is_boundary_element(name: &str) -> bool {
    BOUNDARY_ELEMENTS
        .iter()
        .any(|element| name.eq_ignore_ascii_case(element))
}

fn decode_entity(remaining: &str) -> Option<(String, usize)> {
    let semicolon = remaining.find(';')?;
    if semicolon > 12 {
        return None;
    }

    let entity = &remaining[1..semicolon];
    let decoded = match entity {
        "amp" => "&".to_string(),
        "apos" => "'".to_string(),
        "gt" => ">".to_string(),
        "lt" => "<".to_string(),
        "nbsp" => " ".to_string(),
        "quot" => "\"".to_string(),
        numeric if numeric.starts_with("#x") || numeric.starts_with("#X") => {
            decode_numeric_entity(&numeric[2..], 16)?
        }
        numeric if numeric.starts_with('#') => decode_numeric_entity(&numeric[1..], 10)?,
        _ => return None,
    };

    Some((decoded, semicolon + 1))
}

fn decode_numeric_entity(digits: &str, radix: u32) -> Option<String> {
    let codepoint = u32::from_str_radix(digits, radix).ok()?;
    char::from_u32(codepoint).map(|ch| ch.to_string())
}

#[cfg(test)]
mod tests {
    use super::{text_for_display, to_plain_text};

    #[test]
    fn renders_visible_text_without_tags_or_metadata() {
        let html = concat!(
            r#"<meta http-equiv="content-type" content="text/html; charset=utf-8">"#,
            r#"<div class="message"><strong>Hello</strong> world</div>"#
        );

        assert_eq!(to_plain_text(html), "Hello world");
    }

    #[test]
    fn decodes_named_and_numeric_entities() {
        let html = "<p>Tom &amp; Jerry&nbsp;&#x1F63A; &#62; Spike</p>";

        assert_eq!(to_plain_text(html), "Tom & Jerry 😺 > Spike");
    }

    #[test]
    fn omits_non_visible_element_content() {
        let html = concat!(
            "<style>.secret { color: red; }</style>",
            "<p>Visible</p>",
            "<script>alert('hidden')</script>",
            "<svg><title>Icon</title></svg>"
        );

        assert_eq!(to_plain_text(html), "Visible");
    }

    #[test]
    fn preserves_boundaries_between_block_elements() {
        let html = "<div>first</div><div>second<br>third</div>";

        assert_eq!(to_plain_text(html), "first second third");
    }

    #[test]
    fn leaves_non_html_content_unchanged() {
        let text = "2 < 3 & plain";

        assert_eq!(text_for_display("text/plain;charset=utf-8", text), text);
    }

    #[test]
    fn treats_unclosed_tags_as_text_without_panicking() {
        let html = "<strong title=\"unfinished 😺";

        assert_eq!(to_plain_text(html), "<strong title=\"unfinished 😺");
    }
}
