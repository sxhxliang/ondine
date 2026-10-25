//! The root SVG document around a diagram's content. Port of Pele's `src/svg/root.ts`.

use std::borrow::Cow;

use crate::svg::builder::{esc, esc_text};
use crate::svg::theme::{
    ALIAS_DEFS, FONT, RADIUS, ROOT_STYLE, ROUNDED, font_style, static_font_style,
};
use crate::yaml::push_number;

/// The host's options that the shared SVG modules read. A subset of Pele's `RenderOptions`;
/// diagram modules add the fields they read as they are ported. Ondine has no font options, since
/// fonts come from `--ondine-font` and every width is monospace.
#[derive(Clone, Debug, Default)]
pub(crate) struct RenderOptions {
    /// Whether a diagram that runs across may be drawn running down when it does not fit
    /// `max_width`. On unless `Some(false)`.
    pub(crate) auto_direction: Option<bool>,
    /// The width under which a diagram may turn to run down. 640 when absent or not positive.
    pub(crate) direction_breakpoint: Option<f64>,
    /// Font size in pixels. 16 when absent.
    pub(crate) font_size: Option<f64>,
    /// The prefix of element IDs, so several diagrams can share a page. `pele` when absent.
    pub(crate) id_prefix: Option<String>,
    /// Schemes an image may use besides a relative address.
    pub(crate) image_schemes: Option<Vec<String>>,
    /// Whether images are drawn. On unless `Some(false)`.
    pub(crate) images: Option<bool>,
    /// Whether the SVG inlines its theme instead of leaving it in
    /// `--ondine-*` custom properties. Off unless `Some(true)`: the
    /// default output keeps variables so a host stylesheet can retheme
    /// a diagram without re-rendering it. Static output is for
    /// rasterisers that do not resolve custom properties (resvg), and
    /// honours the `theme_*` fields below when they are set.
    pub(crate) inline_theme: Option<bool>,
    /// The most UTF-16 units of source to accept. 50,000 when absent.
    pub(crate) limit: Option<f64>,
    /// The `rel` of links. `noopener` when absent; empty omits it.
    pub(crate) link_rel: Option<String>,
    /// Schemes a link may use besides a relative address.
    pub(crate) link_schemes: Option<Vec<String>>,
    /// Whether links are drawn. On unless `Some(false)`.
    pub(crate) links: Option<bool>,
    /// The most edges a flowchart may have. 5,000 when absent.
    pub(crate) max_edges: Option<f64>,
    /// The width the host has for the diagram, in pixels. A type that can draw itself narrower
    /// does, so its text is not shrunk with it.
    pub(crate) max_width: Option<f64>,
    /// The time a gantt chart schedules against, in milliseconds since the epoch. The epoch
    /// when absent, since the compiler reads no clock.
    pub(crate) now: Option<f64>,
    /// The most UTF-16 units of SVG to produce. 4,000,000 when absent.
    pub(crate) output_limit: Option<f64>,
    /// Space around a diagram. 8 when absent.
    pub(crate) padding: Option<f64>,
    /// Whether the document scales down to its container. On unless `Some(false)`.
    pub(crate) responsive: Option<bool>,
    /// Static-mode accent color. Ignored unless `inline_theme` is on.
    pub(crate) theme_accent: Option<String>,
    /// Static-mode background color. Ignored unless `inline_theme` is on.
    pub(crate) theme_background: Option<String>,
    /// Static-mode border color. Ignored unless `inline_theme` is on.
    pub(crate) theme_border: Option<String>,
    /// Static-mode font stack. Ignored unless `inline_theme` is on.
    pub(crate) theme_font: Option<String>,
    /// Static-mode foreground (text) color. Ignored unless `inline_theme` is on.
    pub(crate) theme_foreground: Option<String>,
    /// Static-mode line color. Ignored unless `inline_theme` is on.
    pub(crate) theme_line: Option<String>,
    /// Static-mode node surface color. Ignored unless `inline_theme` is on.
    pub(crate) theme_surface: Option<String>,
}

/// A diagram's accessible title and description.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Accessible<'a> {
    /// The accessible description.
    pub(crate) acc_descr: Option<&'a str>,
    /// The accessible title.
    pub(crate) acc_title: Option<&'a str>,
}

/// Moves the themed corner radius of each `<rect>` into its style. Browsers only lately take a
/// variable in the `rx` attribute, and all of them take one in a style. It goes first there, so a
/// radius the diagram's author set comes after it and wins. Also returns whether any moved.
fn rounded(content: &str) -> (Cow<'_, str>, bool) {
    let mut moved = false;
    let bytes = content.as_bytes();
    let mut out = String::with_capacity(content.len() + 64);
    let mut start = 0;
    let mut at = 0;
    while let Some(found) = content.get(at..).and_then(|rest| rest.find("<rect")) {
        let tag_start = at + found;
        let after = tag_start + 5;
        // `\b` after `rect`: the next character is not a word character.
        let bounded = !bytes
            .get(after)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_');
        let close = content.get(after..).and_then(|rest| rest.find('>'));
        let (true, Some(close)) = (bounded, close) else {
            at = tag_start + 1;
            continue;
        };
        let tag_end = after + close + 1;
        let tag = content.get(tag_start..tag_end).unwrap_or("");
        out.push_str(content.get(start..tag_start).unwrap_or(""));
        moved |= push_rounded(&mut out, tag);
        start = tag_end;
        at = tag_end;
    }
    out.push_str(content.get(start..).unwrap_or(""));
    (Cow::Owned(out), moved)
}

/// Appends one `<rect>` tag with its themed radius moved into its style, and returns whether it
/// had one.
fn push_rounded(out: &mut String, tag: &str) -> bool {
    let Some(at) = tag.find(ROUNDED) else {
        out.push_str(tag);
        return false;
    };

    let mut rest = String::with_capacity(tag.len());
    rest.push_str(tag.get(..at).unwrap_or(""));
    rest.push_str(tag.get(at + ROUNDED.len()..).unwrap_or(""));
    if let Some(style) = rest.find(" style=\"") {
        out.push_str(rest.get(..style + 8).unwrap_or(""));
        out.push_str("rx:var(--_r);");
        out.push_str(rest.get(style + 8..).unwrap_or(""));
        return true;
    }

    let end = if rest.ends_with("/>") {
        rest.len() - 2
    } else {
        rest.len().saturating_sub(1)
    };
    out.push_str(rest.get(..end).unwrap_or(""));
    out.push_str(" style=\"rx:var(--_r)\"");
    out.push_str(rest.get(end..).unwrap_or(""));
    true
}

/// The weights of bold text, each set only in a diagram that has such text: the alias and the
/// name in its public variable.
const WEIGHTS: [(&str, &str); 3] = [("--_tw", "title"), ("--_hw", "heading"), ("--_w", "bold")];

/// A theme resolved to concrete values, for static mode. Every field is
/// a color (or a font stack) with no `var()` left in it.
struct StaticTheme {
    /// Alternate surface: 20% foreground over the background.
    alt: String,
    /// Node surface color.
    surface: String,
    /// Accent color.
    accent: String,
    /// Background color.
    bg: String,
    /// Border color.
    border: String,
    /// Foreground (text) color.
    fg: String,
    /// Font stack, with double quotes single-quoted so it cannot end
    /// a double-quoted attribute early.
    font: String,
    /// Line color.
    line: String,
}

/// Pushes `font` with every double quote single-quoted. A font stack
/// from a host may quote family names (`"Segoe UI"`); left double-quoted
/// inside a double-quoted attribute, the first such quote would end the
/// attribute and truncate the whole root style.
fn push_sanitized_font(out: &mut String, font: &str) {
    for ch in font.chars() {
        if ch == '"' {
            out.push('\'');
        } else {
            out.push(ch);
        }
    }
}

/// Pushes two lowercase hex digits for one channel.
fn push_hex_byte(out: &mut String, value: u8) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out.push(char::from(HEX[(value >> 4) as usize]));
    out.push(char::from(HEX[(value & 0x0f) as usize]));
}

/// Parses `#rgb` or `#rrggbb` into channels. Anything else is not a
/// color static mode can mix, so it resolves to nothing.
fn parse_hex_color(text: &str) -> Option<(u8, u8, u8)> {
    fn nibble(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }
    let text = text.trim().trim_start_matches('#');
    let bytes = text.as_bytes();
    if bytes.len() == 3 {
        let r = nibble(bytes[0])?;
        let g = nibble(bytes[1])?;
        let b = nibble(bytes[2])?;
        Some((r * 17, g * 17, b * 17))
    } else if bytes.len() == 6 {
        let channel = |hi: u8, lo: u8| Some(nibble(hi)? * 16 + nibble(lo)?);
        Some((
            channel(bytes[0], bytes[1])?,
            channel(bytes[2], bytes[3])?,
            channel(bytes[4], bytes[5])?,
        ))
    } else {
        None
    }
}

/// Resolves the static theme from the host options. Unset colors fall
/// back exactly as `ROOT_STYLE` falls back: lines, borders, accents and
/// muted text to the foreground, surfaces to the background.
fn static_theme(options: &RenderOptions) -> StaticTheme {
    let bg = options
        .theme_background
        .as_deref()
        .unwrap_or("#fff")
        .to_string();
    let fg = options
        .theme_foreground
        .as_deref()
        .unwrap_or("#000")
        .to_string();
    let line = options.theme_line.as_deref().unwrap_or(&fg).to_string();
    let border = options.theme_border.as_deref().unwrap_or(&fg).to_string();
    let surface = options.theme_surface.as_deref().unwrap_or(&bg).to_string();
    let accent = options.theme_accent.as_deref().unwrap_or(&fg).to_string();
    let mut font = String::new();
    if let Some(host) = options.theme_font.as_deref() {
        push_sanitized_font(&mut font, host);
    } else {
        font.push_str(FONT);
    }
    let alt = match (parse_hex_color(&fg), parse_hex_color(&bg)) {
        (Some(fg_rgb), Some(bg_rgb)) => {
            let mut mixed = String::with_capacity(7);
            mixed.push('#');
            for channel in 0..3 {
                let a = u32::from([fg_rgb.0, fg_rgb.1, fg_rgb.2][channel]);
                let b = u32::from([bg_rgb.0, bg_rgb.1, bg_rgb.2][channel]);
                let value = (a * 20 + b * 80 + 50) / 100;
                push_hex_byte(&mut mixed, u8::try_from(value).unwrap_or(u8::MAX));
            }
            mixed
        }
        _ => bg.clone(),
    };
    StaticTheme {
        alt,
        surface,
        accent,
        bg,
        border,
        fg,
        font,
        line,
    }
}

/// The theme value for a public `--ondine-*` variable, when static
/// mode was given one. Other public variables keep their fallback.
fn theme_var<'a>(theme: &'a StaticTheme, name: &str) -> Option<&'a str> {
    match name {
        "--ondine-accent" => Some(&theme.accent),
        "--ondine-bg" => Some(&theme.bg),
        "--ondine-border" => Some(&theme.border),
        "--ondine-font" => Some(&theme.font),
        "--ondine-fg" => Some(&theme.fg),
        "--ondine-line" => Some(&theme.line),
        "--ondine-surface" => Some(&theme.surface),
        _ => None,
    }
}

/// Finds the index just past the `)` matching the `(` at `open`, or
/// nothing when the parentheses never balance.
fn matching_paren(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Splits a `var()` inner expression at its first top-level comma
/// into the variable name and its fallback, when there is one.
fn split_var_inner(inner: &str) -> (&str, Option<&str>) {
    let bytes = inner.as_bytes();
    let mut depth = 0i32;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b',' if depth == 0 => {
                return (
                    inner.get(..i).unwrap_or("").trim(),
                    Some(inner.get(i + 1..).unwrap_or("").trim()),
                );
            }
            _ => {}
        }
        i += 1;
    }
    (inner.trim(), None)
}

/// Appends the value of one `var()` inner expression with every
/// variable resolved. Unknown variables without a fallback are left
/// as they were: static mode resolves what the theme defines and
/// never invents a color.
fn push_var_value(out: &mut String, inner: &str, theme: &StaticTheme, depth: usize) {
    if depth > 8 {
        out.push_str("var(");
        out.push_str(inner);
        out.push(')');
        return;
    }
    let (name, fallback) = split_var_inner(inner);
    if let Some(definition) = ALIAS_DEFS
        .iter()
        .find(|(alias, _)| *alias == name)
        .map(|(_, definition)| *definition)
    {
        push_resolved_expr(out, definition, theme, depth + 1);
        return;
    }
    if let Some(value) = theme_var(theme, name) {
        out.push_str(value);
        return;
    }
    if let Some(fallback) = fallback {
        push_resolved_expr(out, fallback, theme, depth + 1);
        return;
    }
    out.push_str("var(");
    out.push_str(inner);
    out.push(')');
}

/// Appends `expr` with every `var()` in it resolved. A leading
/// `color-mix()` is computed; anything else is copied through with
/// only its variables replaced.
fn push_resolved_expr(out: &mut String, expr: &str, theme: &StaticTheme, depth: usize) {
    let expr = expr.trim();
    if expr.starts_with("color-mix(") {
        if push_color_mix(out, expr, theme, depth) {
            return;
        }
        out.push_str(expr);
        return;
    }
    let bytes = expr.as_bytes();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes.get(i..i + 4) == Some(b"var(") {
            let Some(end) = matching_paren(expr, i + 3) else {
                break;
            };
            out.push_str(expr.get(start..i).unwrap_or(""));
            let inner = expr.get(i + 4..end - 1).unwrap_or("");
            push_var_value(out, inner, theme, depth + 1);
            i = end;
            start = end;
        } else {
            i += 1;
        }
    }
    out.push_str(expr.get(start..).unwrap_or(""));
}

/// Resolves one color expression (a `var()`, a hex color, or a
/// fallback chain of them) to channels. Anything else is not a color
/// static mode can mix.
fn resolve_color(expr: &str, theme: &StaticTheme, depth: usize) -> Option<(u8, u8, u8)> {
    let expr = expr.trim();
    if expr.starts_with("var(") {
        let mut resolved = String::new();
        push_resolved_expr(&mut resolved, expr, theme, depth + 1);
        parse_hex_color(&resolved)
    } else {
        parse_hex_color(expr)
    }
}

/// Appends the computed color of a `color-mix(in srgb, A P%, B)` call.
/// Returns false when the call is not in that shape, so the caller
/// copies it through unchanged.
fn push_color_mix(out: &mut String, expr: &str, theme: &StaticTheme, depth: usize) -> bool {
    let Some(open) = expr.find('(') else {
        return false;
    };
    let Some(end) = matching_paren(expr, open) else {
        return false;
    };
    let inner = expr.get(open + 1..end - 1).unwrap_or("");
    let Some(inner) = inner.strip_prefix("in srgb,") else {
        return false;
    };
    let bytes = inner.as_bytes();
    let mut paren_depth = 0i32;
    let mut split = None;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => paren_depth += 1,
            b')' => paren_depth -= 1,
            b',' if paren_depth == 0 => {
                split = Some(i);
                break;
            }
            _ => {}
        }
        i += 1;
    }
    let Some(split) = split else {
        return false;
    };
    let first = inner.get(..split).unwrap_or("").trim();
    let second = inner.get(split + 1..).unwrap_or("").trim();
    let part = |text: &str| -> Option<((u8, u8, u8), Option<u32>)> {
        let (color, pct) = match text.rfind(' ') {
            Some(at) if text.get(at + 1..).unwrap_or("").trim_end().ends_with('%') => (
                text.get(..at).unwrap_or("").trim(),
                Some(
                    text.get(at + 1..)
                        .unwrap_or("")
                        .trim()
                        .trim_end_matches('%')
                        .parse::<u32>()
                        .ok()?,
                ),
            ),
            _ => (text, None),
        };
        Some((resolve_color(color, theme, depth + 1)?, pct))
    };
    let Some((first_rgb, first_pct)) = part(first) else {
        return false;
    };
    let Some((second_rgb, second_pct)) = part(second) else {
        return false;
    };
    let first_weight = first_pct.unwrap_or(100 - second_pct.unwrap_or(50));
    let second_weight = 100u32.saturating_sub(first_weight);
    let mix = |a: u8, b: u8| {
        let value = (u32::from(a) * first_weight + u32::from(b) * second_weight + 50) / 100;
        u8::try_from(value).unwrap_or(u8::MAX)
    };
    out.push('#');
    push_hex_byte(out, mix(first_rgb.0, second_rgb.0));
    push_hex_byte(out, mix(first_rgb.1, second_rgb.1));
    push_hex_byte(out, mix(first_rgb.2, second_rgb.2));
    true
}

/// Appends diagram `content` with every `var()` inside a tag resolved
/// to a concrete value. Text between tags is copied untouched, so a
/// label that happens to read `var(--_fg)` keeps its literal text.
fn push_static_content(out: &mut String, content: &str, theme: &StaticTheme) {
    let bytes = content.as_bytes();
    let mut start = 0;
    let mut i = 0;
    let mut in_tag = false;
    let mut in_quote = false;
    while i < bytes.len() {
        let byte = bytes[i];
        if in_tag {
            // Variables live inside quoted attribute values
            // (`fill="var(--_s)"`), so quote tracking only guards the
            // tag end: a `>` inside quotes does not close the tag, but
            // a `var(` inside quotes is still rewritten.
            if byte == b'"' {
                in_quote = !in_quote;
                i += 1;
                continue;
            }
            if !in_quote && byte == b'>' {
                in_tag = false;
                i += 1;
                continue;
            }
            if byte == b'v' && bytes.get(i..i + 4) == Some(b"var(") {
                let Some(end) = matching_paren(content, i + 3) else {
                    i += 1;
                    continue;
                };
                out.push_str(content.get(start..i).unwrap_or(""));
                let inner = content.get(i + 4..end - 1).unwrap_or("");
                push_var_value(out, inner, theme, 0);
                i = end;
                start = end;
                continue;
            }
            i += 1;
        } else {
            if byte == b'<' {
                in_tag = true;
            }
            i += 1;
        }
    }
    out.push_str(content.get(start..).unwrap_or(""));
}

/// Wraps a diagram's content in the root `<svg>` document of the given size, with its theme,
/// font at `font_size`, accessible title and description, and diagram `type` class.
pub(crate) fn svg_document(
    r#type: &str,
    width: f64,
    height: f64,
    font_size: f64,
    options: &RenderOptions,
    acc: Accessible<'_>,
    content: &str,
) -> String {
    let prefix = esc(options.id_prefix.as_deref().unwrap_or("ondine"));
    let mut head = String::new();
    let mut aria = String::new();
    if let Some(title) = acc.acc_title.filter(|title| !title.is_empty()) {
        head.push_str("<title id=\"");
        head.push_str(&prefix);
        head.push_str("-title\">");
        head.push_str(&esc_text(title));
        head.push_str("</title>");
        aria.push_str(" aria-labelledby=\"");
        aria.push_str(&prefix);
        aria.push_str("-title\"");
    }
    if let Some(descr) = acc.acc_descr.filter(|descr| !descr.is_empty()) {
        head.push_str("<desc id=\"");
        head.push_str(&prefix);
        head.push_str("-desc\">");
        head.push_str(&esc_text(descr));
        head.push_str("</desc>");
        aria.push_str(" aria-describedby=\"");
        aria.push_str(&prefix);
        aria.push_str("-desc\"");
    }

    // One scan finds the themed radius and every alias the content reads.
    let mut radius = false;
    let mut themed = false;
    let mut used = [false; WEIGHTS.len()];
    for (at, _) in content.match_indices("var(--") {
        let rest = content.get(at + 6..).unwrap_or("");
        let Some(name) = rest.strip_prefix("_") else {
            themed |= content
                .get(at.saturating_sub(ROUNDED.len() - RADIUS.len() - 1)..)
                .is_some_and(|tail| tail.starts_with(ROUNDED));
            continue;
        };
        radius |= name.starts_with("r)");
        for (used, (alias, _)) in used.iter_mut().zip(&WEIGHTS) {
            *used |= name
                .strip_prefix(alias.get(3..).unwrap_or(""))
                .is_some_and(|tail| tail.starts_with(')'));
        }
    }

    let content = if themed {
        let (content, moved) = rounded(content);
        radius |= moved;
        content
    } else {
        Cow::Borrowed(content)
    };

    let static_mode = options.inline_theme == Some(true);
    let theme = static_mode.then(|| static_theme(options));
    let mut weights = String::new();
    if radius {
        weights.push_str(";--_r:");
        if static_mode {
            weights.push('0');
        } else {
            weights.push_str(RADIUS);
        }
    }
    for ((alias, name), used) in WEIGHTS.iter().zip(used) {
        if used {
            weights.push(';');
            weights.push_str(alias);
            if static_mode {
                weights.push_str(":bold");
            } else {
                weights.push_str(":var(--ondine-");
                weights.push_str(name);
                weights.push_str("-weight,bold)");
            }
        }
    }

    let mut out = String::with_capacity(content.len() + 1024);
    out.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"ondine ondine-");
    out.push_str(r#type);
    out.push_str("\" viewBox=\"0 0 ");
    push_number(&mut out, width);
    out.push(' ');
    push_number(&mut out, height);
    out.push_str("\" width=\"");
    push_number(&mut out, width);
    out.push_str("\" height=\"");
    push_number(&mut out, height);
    out.push_str("\" style=\"");
    if options.responsive != Some(false) {
        out.push_str("max-width:100%;height:auto;");
    }
    if let Some(theme) = &theme {
        out.push_str("--_bg:");
        out.push_str(&theme.bg);
        out.push_str(";--_fg:");
        out.push_str(&theme.fg);
        out.push_str(";--_m:");
        out.push_str(&theme.fg);
        out.push_str(";--_l:");
        out.push_str(&theme.line);
        out.push_str(";--_s:");
        out.push_str(&theme.surface);
        out.push_str(";--_a:");
        out.push_str(&theme.alt);
        out.push_str(";--_b:");
        out.push_str(&theme.border);
        out.push_str(";--_c:");
        out.push_str(&theme.accent);
        out.push_str(";--_sw:2px;stroke-width:2px;color:");
        out.push_str(&theme.fg);
        out.push_str(&weights);
        out.push_str(&static_font_style(font_size));
        out.push_str("\" fill=\"");
        out.push_str(&theme.fg);
    } else {
        out.push_str(ROOT_STYLE);
        out.push_str(&weights);
        out.push_str(&font_style(font_size));
        out.push_str("\" fill=\"var(--_fg)");
    }
    out.push_str("\" role=\"graphics-document document\" aria-roledescription=\"");
    out.push_str(r#type);
    out.push('"');
    out.push_str(&aria);
    out.push('>');
    out.push_str(&head);
    if let Some(theme) = &theme {
        out.push_str("<g fill=\"");
        out.push_str(&theme.fg);
        out.push_str("\" font-size=\"");
        push_number(&mut out, font_size);
        out.push_str("px\" font-family=\"");
        out.push_str(&theme.font);
        out.push_str("\">");
        push_static_content(&mut out, &content, theme);
        out.push_str("</g>");
    } else {
        out.push_str(&content);
    }
    out.push_str("</svg>");
    out
}

#[cfg(test)]
mod tests {
    //! Expectations come from Pele's `svgDocument`, after the theme substitution: Pele's root
    //! style, font declarations, and `--pele-*` variables replaced by Ondine's.

    use super::{Accessible, RenderOptions, svg_document};

    #[expect(clippy::type_complexity, reason = "a table of cases")]
    const CASES: &[(
        &str,
        f64,
        f64,
        f64,
        Option<&str>,
        Option<bool>,
        Option<&str>,
        Option<&str>,
        &str,
        &str,
    )] = &[
        (
            "flowchart",
            120.0,
            80.0,
            16.0,
            None,
            None,
            None,
            None,
            "<rect x=\"0\" y=\"0\" width=\"5\" height=\"5\" rx=\"var(--ondine-radius,0)\" fill=\"red\"/>",
            "<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"ondine ondine-flowchart\" viewBox=\"0 0 120 80\" width=\"120\" height=\"80\" style=\"max-width:100%;height:auto;--_bg:var(--ondine-bg,#fff);--_fg:var(--ondine-fg,#000);--_m:var(--ondine-muted,var(--_fg));--_l:var(--ondine-line,var(--_fg));--_s:var(--ondine-surface,var(--_bg));--_a:var(--ondine-surface-alt,color-mix(in srgb,var(--_fg) 20%,var(--_bg)));--_b:var(--ondine-border,var(--_fg));--_c:var(--ondine-accent,var(--_fg));--_sw:var(--ondine-stroke,2px);stroke-width:var(--_sw);color:var(--_fg);--_r:var(--ondine-radius,0);--_fm:var(--ondine-font,ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', 'DejaVu Sans Mono', monospace);font:16px monospace;letter-spacing:normal;word-spacing:normal;text-transform:none;direction:ltr;font-family:var(--ondine-font,ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', 'DejaVu Sans Mono', monospace);font-kerning:none;font-variant-ligatures:none;text-rendering:geometricPrecision\" fill=\"var(--_fg)\" role=\"graphics-document document\" aria-roledescription=\"flowchart\"><rect x=\"0\" y=\"0\" width=\"5\" height=\"5\" fill=\"red\" style=\"rx:var(--_r)\"/></svg>",
        ),
        (
            "flowchart",
            120.5,
            80.0,
            14.0,
            Some("my\"id"),
            Some(false),
            Some("Title &amp; <x>"),
            Some("Desc"),
            "<rect rx=\"var(--ondine-radius,0)\" style=\"fill:red\"/><rectangle rx=\"var(--ondine-radius,0)\"/><text font-weight=\"var(--_w)\">b</text>",
            "<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"ondine ondine-flowchart\" viewBox=\"0 0 120.5 80\" width=\"120.5\" height=\"80\" style=\"--_bg:var(--ondine-bg,#fff);--_fg:var(--ondine-fg,#000);--_m:var(--ondine-muted,var(--_fg));--_l:var(--ondine-line,var(--_fg));--_s:var(--ondine-surface,var(--_bg));--_a:var(--ondine-surface-alt,color-mix(in srgb,var(--_fg) 20%,var(--_bg)));--_b:var(--ondine-border,var(--_fg));--_c:var(--ondine-accent,var(--_fg));--_sw:var(--ondine-stroke,2px);stroke-width:var(--_sw);color:var(--_fg);--_r:var(--ondine-radius,0);--_w:var(--ondine-bold-weight,bold);--_fm:var(--ondine-font,ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', 'DejaVu Sans Mono', monospace);font:14px monospace;letter-spacing:normal;word-spacing:normal;text-transform:none;direction:ltr;font-family:var(--ondine-font,ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', 'DejaVu Sans Mono', monospace);font-kerning:none;font-variant-ligatures:none;text-rendering:geometricPrecision\" fill=\"var(--_fg)\" role=\"graphics-document document\" aria-roledescription=\"flowchart\" aria-labelledby=\"my&quot;id-title\" aria-describedby=\"my&quot;id-desc\"><title id=\"my&quot;id-title\">Title &amp; &lt;x&gt;</title><desc id=\"my&quot;id-desc\">Desc</desc><rect style=\"rx:var(--_r);fill:red\"/><rectangle rx=\"var(--ondine-radius,0)\"/><text font-weight=\"var(--_w)\">b</text></svg>",
        ),
        (
            "pie",
            10.0,
            10.0,
            16.0,
            None,
            None,
            Some(""),
            Some("only desc"),
            "<g><text font-weight=\"var(--_tw)\">t</text><text font-weight=\"var(--_hw)\">h</text></g><rect width=\"1\"/>",
            "<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"ondine ondine-pie\" viewBox=\"0 0 10 10\" width=\"10\" height=\"10\" style=\"max-width:100%;height:auto;--_bg:var(--ondine-bg,#fff);--_fg:var(--ondine-fg,#000);--_m:var(--ondine-muted,var(--_fg));--_l:var(--ondine-line,var(--_fg));--_s:var(--ondine-surface,var(--_bg));--_a:var(--ondine-surface-alt,color-mix(in srgb,var(--_fg) 20%,var(--_bg)));--_b:var(--ondine-border,var(--_fg));--_c:var(--ondine-accent,var(--_fg));--_sw:var(--ondine-stroke,2px);stroke-width:var(--_sw);color:var(--_fg);--_tw:var(--ondine-title-weight,bold);--_hw:var(--ondine-heading-weight,bold);--_fm:var(--ondine-font,ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', 'DejaVu Sans Mono', monospace);font:16px monospace;letter-spacing:normal;word-spacing:normal;text-transform:none;direction:ltr;font-family:var(--ondine-font,ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', 'DejaVu Sans Mono', monospace);font-kerning:none;font-variant-ligatures:none;text-rendering:geometricPrecision\" fill=\"var(--_fg)\" role=\"graphics-document document\" aria-roledescription=\"pie\" aria-describedby=\"ondine-desc\"><desc id=\"ondine-desc\">only desc</desc><g><text font-weight=\"var(--_tw)\">t</text><text font-weight=\"var(--_hw)\">h</text></g><rect width=\"1\"/></svg>",
        ),
        (
            "sequence",
            0.0,
            0.0,
            12.5,
            Some(""),
            None,
            None,
            None,
            "",
            "<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"ondine ondine-sequence\" viewBox=\"0 0 0 0\" width=\"0\" height=\"0\" style=\"max-width:100%;height:auto;--_bg:var(--ondine-bg,#fff);--_fg:var(--ondine-fg,#000);--_m:var(--ondine-muted,var(--_fg));--_l:var(--ondine-line,var(--_fg));--_s:var(--ondine-surface,var(--_bg));--_a:var(--ondine-surface-alt,color-mix(in srgb,var(--_fg) 20%,var(--_bg)));--_b:var(--ondine-border,var(--_fg));--_c:var(--ondine-accent,var(--_fg));--_sw:var(--ondine-stroke,2px);stroke-width:var(--_sw);color:var(--_fg);--_fm:var(--ondine-font,ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', 'DejaVu Sans Mono', monospace);font:12.5px monospace;letter-spacing:normal;word-spacing:normal;text-transform:none;direction:ltr;font-family:var(--ondine-font,ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', 'DejaVu Sans Mono', monospace);font-kerning:none;font-variant-ligatures:none;text-rendering:geometricPrecision\" fill=\"var(--_fg)\" role=\"graphics-document document\" aria-roledescription=\"sequence\"></svg>",
        ),
        (
            "x",
            1.0,
            2.0,
            16.0,
            None,
            None,
            None,
            None,
            "<rect rx=\"var(--ondine-radius,0)\">",
            "<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"ondine ondine-x\" viewBox=\"0 0 1 2\" width=\"1\" height=\"2\" style=\"max-width:100%;height:auto;--_bg:var(--ondine-bg,#fff);--_fg:var(--ondine-fg,#000);--_m:var(--ondine-muted,var(--_fg));--_l:var(--ondine-line,var(--_fg));--_s:var(--ondine-surface,var(--_bg));--_a:var(--ondine-surface-alt,color-mix(in srgb,var(--_fg) 20%,var(--_bg)));--_b:var(--ondine-border,var(--_fg));--_c:var(--ondine-accent,var(--_fg));--_sw:var(--ondine-stroke,2px);stroke-width:var(--_sw);color:var(--_fg);--_r:var(--ondine-radius,0);--_fm:var(--ondine-font,ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', 'DejaVu Sans Mono', monospace);font:16px monospace;letter-spacing:normal;word-spacing:normal;text-transform:none;direction:ltr;font-family:var(--ondine-font,ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', 'DejaVu Sans Mono', monospace);font-kerning:none;font-variant-ligatures:none;text-rendering:geometricPrecision\" fill=\"var(--_fg)\" role=\"graphics-document document\" aria-roledescription=\"x\"><rect style=\"rx:var(--_r)\"></svg>",
        ),
    ];

    #[test]
    fn svg_document_matches_pele() {
        for &(
            r#type,
            width,
            height,
            size,
            id_prefix,
            responsive,
            acc_title,
            acc_descr,
            content,
            expected,
        ) in CASES
        {
            let options = RenderOptions {
                id_prefix: id_prefix.map(str::to_owned),
                responsive,
                ..RenderOptions::default()
            };
            let acc = Accessible {
                acc_descr,
                acc_title,
            };
            assert_eq!(
                svg_document(r#type, width, height, size, &options, acc, content),
                expected
            );
        }
    }
}
