//! **The index tools say what they are for that `outline` and `read_file` are not** (the measure of 24 Nov 2026: two
//! first turns with 0 `search_code` and 0 `usages`, a report whose names came from `outline` and memory). The shipped
//! descriptions say: check a name by `usages` before citing it; `outline` does not tell who calls nor where else a name
//! lives. A module of its own.

fn description(tool: &str) -> String {
    let f = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("templates/tools/atelier_{tool}.mmd"));
    std::fs::read_to_string(&f).unwrap().lines().find_map(|l| l.strip_prefix("%% description: ").map(str::to_string)).unwrap_or_default()
}

#[test]
fn usages_says_it_checks_a_name_before_it_is_cited_and_outline_points_to_it() {
    let usages = description("usages");
    assert!(usages.contains("avant de le citer"), "usages says it checks a name before it is cited: {usages}");
    let outline = description("outline");
    assert!(outline.contains("usages") && outline.contains("search_code"), "outline says what it does not tell, and which tool does: {outline}");
}
