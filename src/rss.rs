//! Small dependency-free RSS/Atom reader shared by every NSH target.
//! The feeds we consume are XML with HTML inside their description fields,
//! so parsing here is deliberately tolerant instead of strict.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    V2ex,
    LinuxDo,
}

impl Source {
    pub const ALL: [Source; 2] = [Source::V2ex, Source::LinuxDo];

    pub fn label(self) -> &'static str {
        match self {
            Source::V2ex => "V2EX",
            Source::LinuxDo => "LINUX DO",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Source::V2ex => "blog.v2ex.com/rss.xml",
            Source::LinuxDo => "linux.do · 最新话题",
        }
    }

    /// Primary feed URL used by native builds. The web build asks its own
    /// Cloudflare Pages Function instead, to avoid the browser's CORS limit.
    pub fn feed_url(self) -> &'static str {
        match self {
            // blog.v2ex.com/rss/ is an HTML article, not a feed; /rss.xml is.
            Source::V2ex => "https://blog.v2ex.com/rss.xml",
            // Official Linux.do feed; may be behind a Cloudflare challenge.
            Source::LinuxDo => "https://linux.do/latest.rss",
        }
    }

    /// Fallback feed URL when the primary is blocked (e.g. Linux.do's
    /// Cloudflare managed challenge). `None` for sources without a mirror.
    pub fn feed_fallback_url(self) -> Option<&'static str> {
        match self {
            Source::V2ex => None,
            // Same Linux.do topics, re-published as challenge-free RSS.
            Source::LinuxDo => Some("https://linuxdorss.longpink.com/latest.xml"),
        }
    }

    pub fn query_value(self) -> &'static str {
        match self {
            Source::V2ex => "v2ex",
            Source::LinuxDo => "linuxdo",
        }
    }

    pub fn from_query(value: &str) -> Option<Self> {
        match value {
            "v2ex" => Some(Source::V2ex),
            "linuxdo" => Some(Source::LinuxDo),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct FeedItem {
    pub id: String,
    pub source: Source,
    pub title: String,
    pub link: String,
    pub date: String,
    pub author: String,
    pub summary: String,
    pub content: String,
}

pub fn parse_feed(xml: &str, source: Source) -> Vec<FeedItem> {
    let mut items = Vec::new();
    let mut rest = xml;

    while let Some(start) = find_tag_start(rest, "item") {
        let after_open = match rest[start..].find('>') {
            Some(i) => start + i + 1,
            None => break,
        };
        let end = match rest[after_open..].find("</item>") {
            Some(i) => after_open + i,
            None => break,
        };
        let block = &rest[after_open..end];
        rest = &rest[end + "</item>".len()..];

        let title = plain(&tag(block, &["title"]));
        if title.is_empty() {
            continue;
        }

        let link = link_of(block);
        let guid = plain(&tag(block, &["guid", "id"]));
        let id = if !guid.is_empty() {
            guid
        } else if !link.is_empty() {
            link.clone()
        } else {
            title.clone()
        };

        let raw_date = plain(&tag(block, &["pubDate", "published", "updated", "dc:date"]));
        let raw_description = tag(block, &["description", "summary"]);
        let raw_content = tag(block, &["content:encoded", "content"]);
        let body = if raw_content.trim().is_empty() {
            raw_description.clone()
        } else {
            raw_content
        };
        let content = plain(&body);
        let summary = shorten(&content, 110);

        items.push(FeedItem {
            id,
            source,
            title,
            link,
            date: format_date(&raw_date),
            author: plain(&tag(block, &["dc:creator", "author", "creator"])),
            summary,
            content,
        });
    }

    items
}

fn link_of(block: &str) -> String {
    let direct = plain(&tag(block, &["link"]));
    if !direct.is_empty() {
        return direct;
    }
    // Atom-style <link rel="alternate" href="https://…" />
    if let Some(pos) = block.find("<link") {
        let tail = &block[pos..];
        if let Some(end) = tail.find('>') {
            let attrs = &tail[..end];
            let href = attribute(attrs, "href");
            if !href.is_empty() {
                return decode_entities(&href);
            }
        }
    }
    String::new()
}

fn find_tag_start(haystack: &str, tag: &str) -> Option<usize> {
    let needle = format!("<{tag}");
    let mut from = 0;
    while let Some(rel) = haystack[from..].find(&needle) {
        let pos = from + rel;
        let after = pos + needle.len();
        if haystack[after..]
            .chars()
            .next()
            .is_some_and(|c| c == '>' || c.is_whitespace())
        {
            return Some(pos);
        }
        from = after;
    }
    None
}

/// Returns the inner text of the first matching element, CDATA included.
fn tag(block: &str, names: &[&str]) -> String {
    for name in names {
        let Some(start) = find_tag_start(block, name) else {
            continue;
        };
        let Some(gt) = block[start..].find('>') else {
            continue;
        };
        let from = start + gt + 1;
        let closing = format!("</{name}>");
        let Some(rel_end) = block[from..].find(&closing) else {
            continue;
        };
        let raw = &block[from..from + rel_end];
        let raw = raw.trim();
        if raw.starts_with("<![CDATA[") {
            let inner = raw
                .strip_prefix("<![CDATA[")
                .and_then(|s| s.strip_suffix("]]>"))
                .unwrap_or(raw);
            return inner.to_owned();
        }
        return raw.to_owned();
    }
    String::new()
}

fn attribute(tag_text: &str, name: &str) -> String {
    for quote in ['"', '\''] {
        let needle = format!("{name}={quote}");
        if let Some(pos) = tag_text.find(&needle) {
            let from = pos + needle.len();
            if let Some(end) = tag_text[from..].find(quote) {
                return tag_text[from..from + end].to_owned();
            }
        }
    }
    String::new()
}

fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '&' {
            out.push(c);
            continue;
        }
        let mut entity = String::new();
        let mut terminated = false;
        while let Some(&next) = chars.peek() {
            chars.next();
            if next == ';' {
                terminated = true;
                break;
            }
            entity.push(next);
            if entity.len() > 12 {
                break;
            }
        }
        if !terminated {
            out.push('&');
            out.push_str(&entity);
            continue;
        }
        match entity.as_str() {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" | "#39" => out.push('\''),
            "nbsp" => out.push(' '),
            "hellip" => out.push('…'),
            "mdash" => out.push('—'),
            "ndash" => out.push('–'),
            other if other.starts_with("#x") || other.starts_with("#X") => {
                if let Ok(code) = u32::from_str_radix(&other[2..], 16) {
                    if let Some(ch) = char::from_u32(code) {
                        out.push(ch);
                    }
                } else {
                    out.push('&');
                    out.push_str(other);
                    out.push(';');
                }
            }
            other if other.starts_with('#') => {
                if let Ok(code) = other[1..].parse::<u32>() {
                    if let Some(ch) = char::from_u32(code) {
                        out.push(ch);
                    }
                } else {
                    out.push('&');
                    out.push_str(other);
                    out.push(';');
                }
            }
            other => {
                out.push('&');
                out.push_str(other);
                out.push(';');
            }
        }
    }
    out
}

/// HTML -> readable plain text, keeping paragraph breaks.
pub fn plain(html: &str) -> String {
    let mut s = html.to_owned();
    for tag_name in ["script", "style", "noscript"] {
        loop {
            let open = format!("<{tag_name}");
            let Some(start) = s.to_ascii_lowercase().find(&open) else {
                break;
            };
            let Some(gt) = s[start..].find('>') else {
                break;
            };
            let close = format!("</{tag_name}>");
            let Some(end) = s[start + gt + 1..].to_ascii_lowercase().find(&close) else {
                break;
            };
            let finish = start + gt + 1 + end + close.len();
            s.replace_range(start..finish, " ");
        }
    }

    for block_tag in [
        "br",
        "br/",
        "br />",
        "/p",
        "/div",
        "/li",
        "/h1",
        "/h2",
        "/h3",
        "/blockquote",
        "/tr",
    ] {
        let needle = format!("<{block_tag}");
        s = s.replace(&needle, "\n");
    }

    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }

    let decoded = decode_entities(&out);
    let mut clean = String::with_capacity(decoded.len());
    let mut space = false;
    for ch in decoded.chars() {
        if ch == '\n' {
            while clean.ends_with(' ') {
                clean.pop();
            }
            if !clean.ends_with('\n') {
                clean.push('\n');
            }
            space = false;
        } else if ch.is_whitespace() {
            space = true;
        } else {
            if space && !clean.is_empty() && !clean.ends_with('\n') {
                clean.push(' ');
            }
            space = false;
            clean.push(ch);
        }
    }
    clean.trim().to_owned()
}

fn shorten(s: &str, max: usize) -> String {
    let mut out: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        out.push('…');
    }
    out
}

/// Turns the RSS date into `YYYY-MM-DD HH:MM`; leaves unknown formats intact.
fn format_date(raw: &str) -> String {
    let raw = raw.trim();
    if raw.len() >= 16 && raw.as_bytes().get(4) == Some(&b'-') {
        let date = raw[..16].replace('T', " ");
        return date;
    }
    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.len() >= 5 {
        let months = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        if let Some(month) = months.iter().position(|m| *m == parts[2]) {
            let day = parts[1].parse::<u32>().unwrap_or(0);
            let year = parts[3];
            let time = parts[4].chars().take(5).collect::<String>();
            return format!("{year}-{:02}-{:02} {time}", month + 1, day);
        }
    }
    raw.chars().take(24).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_simple_item() {
        let xml = r#"<?xml version="1.0"?><rss><channel>
            <item><title>Hello &amp; world</title><link>https://example.com/a</link>
            <pubDate>Mon, 16 Nov 2026 17:04:00 +0800</pubDate>
            <description><![CDATA[<p>First <b>paragraph</b>.</p><p>Second.</p>]]></description>
            </item></channel></rss>"#;
        let items = parse_feed(xml, Source::V2ex);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Hello & world");
        assert_eq!(items[0].date, "2026-11-16 17:04");
        assert_eq!(items[0].content, "First paragraph.\nSecond.");
    }
}
