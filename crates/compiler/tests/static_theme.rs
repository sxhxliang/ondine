//! Static (resvg-safe) mode: the default output keeps `--ondine-*`
//! custom properties, and `"inlineTheme": true` resolves them to
//! concrete values without changing anything else.

const FLOWCHART: &str = "flowchart LR\n A[开始] --> B{检查}\n B -->|成功| C[完成]";
const XYCHART: &str = "xychart-beta\n bar [10, 20, 30]";

#[test]
fn default_output_keeps_variables() {
    let rendered = compiler::render(FLOWCHART, "", 0.0).unwrap();
    assert!(
        rendered.svg.contains("--_bg:var(--ondine-bg,#fff)"),
        "default output must keep the variable theme"
    );
    assert!(rendered.svg.contains("var(--_fg)"));
    assert!(
        !rendered.svg.contains("<g fill=\"#000\""),
        "default output must not gain a static wrapper"
    );
}

#[test]
fn static_output_has_no_variables() {
    let rendered = compiler::render(FLOWCHART, "{\"inlineTheme\":true}", 0.0).unwrap();
    assert!(
        !rendered.svg.contains("var(--"),
        "static output must not leave any var() for resvg to drop: {}",
        rendered.svg
    );
    assert!(rendered.svg.contains("开始"));
    assert!(rendered.svg.contains("成功"));
    assert!(
        rendered.svg.contains("<g fill=\"#000\""),
        "static output wraps content in a <g> with explicit text fill"
    );
    assert!(rendered.svg.contains("ui-monospace"));
}

#[test]
fn static_theme_values_apply() {
    let options = "{\"inlineTheme\":true,\"themeBackground\":\"#171717\",\"themeForeground\":\"#eeeeee\",\"themeLine\":\"#64748B\",\"themeBorder\":\"#94A3B8\",\"themeSurface\":\"#F8FAFC\",\"themeAccent\":\"#F8FAFC\",\"themeFont\":\"Noto Sans, sans-serif\"}";
    let rendered = compiler::render(FLOWCHART, options, 0.0).unwrap();
    assert!(!rendered.svg.contains("var(--"));
    assert!(rendered.svg.contains("#171717"));
    assert!(rendered.svg.contains("#eeeeee"));
    assert!(rendered.svg.contains("<g fill=\"#eeeeee\""));
    assert!(rendered.svg.contains("Noto Sans, sans-serif"));
}

#[test]
fn theme_fields_are_ignored_without_the_flag() {
    let rendered = compiler::render(
        FLOWCHART,
        "{\"themeBackground\":\"#171717\",\"themeForeground\":\"#eeeeee\"}",
        0.0,
    )
    .unwrap();
    assert!(rendered.svg.contains("var(--_fg)"));
    assert!(!rendered.svg.contains("#171717"));
}

#[test]
fn static_mode_resolves_series_and_mix_colors() {
    let rendered = compiler::render(XYCHART, "{\"inlineTheme\":true}", 0.0).unwrap();
    assert!(
        !rendered.svg.contains("var(--") && !rendered.svg.contains("color-mix("),
        "series shades are color-mix fallbacks and must be computed: {}",
        rendered.svg
    );
}

#[test]
fn static_mode_keeps_dimensions() {
    let rendered = compiler::render(FLOWCHART, "{\"inlineTheme\":true}", 0.0).unwrap();
    assert!(rendered.width > 0.0 && rendered.height > 0.0);
}
