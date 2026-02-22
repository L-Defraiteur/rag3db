/// Compile a regex literal once per call site, return `&regex::Regex`.
/// Each invocation expands to its own `OnceLock` — zero HashMap lookup.
///
/// ```rust
/// let re = cached_regex!(r"\d+");
/// if re.is_match("abc123") { ... }
/// let caps = cached_regex!(r"(\w+)=(\w+)").captures(text);
/// ```
#[macro_export]
macro_rules! cached_regex {
    ($pattern:expr) => {{
        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        RE.get_or_init(|| regex::Regex::new($pattern).unwrap())
    }};
}
