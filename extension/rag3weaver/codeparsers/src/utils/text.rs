//! Troncatures sûres pour l'UTF-8. `&s[..77]` panique si l'octet 77 tombe
//! au milieu d'un caractère multi-octets — un `─` de séparateur de section
//! suffisait à faire échouer le parsing d'un fichier entier (25 août 2026).

/// Les `max_bytes` premiers octets de `s`, reculés jusqu'à une frontière de
/// caractère. Rend `s` entier s'il est plus court.
pub fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// `s` si ≤ `max_bytes`, sinon un préfixe sûr suivi de `...`.
pub fn ellipsize(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        s.to_string()
    } else {
        format!("{}...", truncate_at_char_boundary(s, max_bytes.saturating_sub(3)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_cuts_inside_a_char() {
        let s = "// ─── section ───────────────────────────────────────────────────";
        for n in 0..=s.len() {
            let t = truncate_at_char_boundary(s, n);
            assert!(t.len() <= n);
            assert!(s.starts_with(t));
        }
        assert_eq!(ellipsize("abc", 80), "abc");
        assert!(ellipsize(s, 80).ends_with("..."));
        assert!(ellipsize(s, 80).len() <= 80);
    }
}
