//! **Un intervalle de version npm, jugé** (19 octobre 2026) : `libs.json` garde
//! les `dependencies` telles qu'écrites dans `package.json`, et une copie rangée
//! les satisfait par les règles de npm. Un nombre nu reste exact (`19.2.0` ne
//! vaut que `19.2.0`), comme les `libs.json` d'avant, qui retiraient le `^`.
//!
//! Le sous-ensemble de npm qu'on lit : `*`, `x`, ``(vide) ; `1.2.3`, `=1.2.3` ;
//! `^`, `~` ; `>`, `>=`, `<`, `<=` ; les jokers `1.x`, `1.2.*`, `1` ; un intervalle
//! `a - b` ; plusieurs comparateurs séparés par des espaces (tous vrais) ; des
//! alternatives `||`. Une préversion (`1.2.3-beta.1`) ne satisfait qu'un
//! comparateur qui en nomme une du même triplet, comme dans npm.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Triplet(u64, u64, u64);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Version {
    t: Triplet,
    pre: Option<String>,
}

fn version(s: &str) -> Option<Version> {
    let s = s.trim().trim_start_matches('v');
    let (corps, pre) = match s.split_once('-') {
        Some((c, p)) => (c, Some(p.split('+').next().unwrap_or(p).to_string())),
        None => (s.split('+').next().unwrap_or(s), None),
    };
    let mut n = corps.split('.').map(|x| x.parse::<u64>().ok());
    let t = Triplet(n.next()??, n.next()??, n.next()??);
    if n.next().is_some() {
        return None;
    }
    Some(Version { t, pre })
}

/// Un triplet partiel : `1`, `1.2`, `1.x`, `1.2.*` → les composants donnés.
fn partiel(s: &str) -> Option<(Option<u64>, Option<u64>, Option<u64>, Option<String>)> {
    let s = s.trim().trim_start_matches('v');
    if s.is_empty() || s == "*" || s.eq_ignore_ascii_case("x") {
        return Some((None, None, None, None));
    }
    let (corps, pre) = match s.split_once('-') {
        Some((c, p)) => (c, Some(p.to_string())),
        None => (s, None),
    };
    let mut parts = corps.split('.');
    let lire = |p: Option<&str>| -> Option<Option<u64>> {
        match p {
            None => Some(None),
            Some(x) if x == "*" || x.eq_ignore_ascii_case("x") => Some(None),
            Some(x) => x.parse().ok().map(Some),
        }
    };
    let a = lire(parts.next())?;
    let b = lire(parts.next())?;
    let c = lire(parts.next())?;
    if parts.next().is_some() {
        return None;
    }
    Some((a, b, c, pre))
}

/// Un comparateur élémentaire : `(op, borne)`.
#[derive(Debug, Clone)]
enum Cmp {
    Ge(Version),
    Gt(Version),
    Lt(Version),
    Le(Version),
    Eq(Version),
}

fn v(t: Triplet, pre: Option<String>) -> Version {
    Version { t, pre }
}

/// Les comparateurs d'un terme (`^1.2.3`, `>=1`, `1.x`…).
fn termes(s: &str) -> Option<Vec<Cmp>> {
    let s = s.trim();
    let (op, reste) = ["<=", ">=", "<", ">", "=", "^", "~"]
        .iter()
        .find_map(|op| s.strip_prefix(op).map(|r| (*op, r.trim())))
        .unwrap_or(("", s));
    let (a, b, c, pre) = partiel(reste)?;
    let bas = Triplet(a.unwrap_or(0), b.unwrap_or(0), c.unwrap_or(0));
    let complet = a.is_some() && b.is_some() && c.is_some();
    Some(match op {
        "" | "=" if complet => vec![Cmp::Eq(v(bas, pre))],
        "" | "=" => match (a, b) {
            (None, _) => vec![],
            (Some(a), None) => vec![Cmp::Ge(v(Triplet(a, 0, 0), None)), Cmp::Lt(v(Triplet(a + 1, 0, 0), None))],
            (Some(a), Some(b)) => vec![Cmp::Ge(v(Triplet(a, b, 0), None)), Cmp::Lt(v(Triplet(a, b + 1, 0), None))],
        },
        ">=" => vec![Cmp::Ge(v(bas, pre))],
        ">" if complet => vec![Cmp::Gt(v(bas, pre))],
        ">" => match (a, b) {
            (None, _) => vec![Cmp::Lt(v(Triplet(0, 0, 0), None))],
            (Some(a), None) => vec![Cmp::Ge(v(Triplet(a + 1, 0, 0), None))],
            (Some(a), Some(b)) => vec![Cmp::Ge(v(Triplet(a, b + 1, 0), None))],
        },
        "<" => vec![Cmp::Lt(v(bas, pre))],
        "<=" if complet => vec![Cmp::Le(v(bas, pre))],
        "<=" => match (a, b) {
            (None, _) => vec![],
            (Some(a), None) => vec![Cmp::Lt(v(Triplet(a + 1, 0, 0), None))],
            (Some(a), Some(b)) => vec![Cmp::Lt(v(Triplet(a, b + 1, 0), None))],
        },
        "~" => {
            let haut = match (a, b) {
                (Some(a), Some(b)) => Triplet(a, b + 1, 0),
                (Some(a), None) => Triplet(a + 1, 0, 0),
                (None, _) => return Some(vec![]),
            };
            vec![Cmp::Ge(v(bas, pre)), Cmp::Lt(v(haut, None))]
        }
        "^" => {
            let haut = match (a, b, c) {
                (None, ..) => return Some(vec![]),
                (Some(0), Some(0), Some(c)) => Triplet(0, 0, c + 1),
                (Some(0), Some(b), _) => Triplet(0, b + 1, 0),
                (Some(a), ..) => Triplet(a + 1, 0, 0),
            };
            vec![Cmp::Ge(v(bas, pre)), Cmp::Lt(v(haut, None))]
        }
        _ => return None,
    })
}

fn vrai(c: &Cmp, x: &Version) -> bool {
    let ordre = |b: &Version| -> std::cmp::Ordering {
        x.t.cmp(&b.t).then_with(|| match (&x.pre, &b.pre) {
            (None, None) => std::cmp::Ordering::Equal,
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(p), Some(q)) => p.cmp(q),
        })
    };
    match c {
        Cmp::Ge(b) => ordre(b).is_ge(),
        Cmp::Gt(b) => ordre(b).is_gt(),
        Cmp::Lt(b) => ordre(b).is_lt(),
        Cmp::Le(b) => ordre(b).is_le(),
        Cmp::Eq(b) => ordre(b).is_eq(),
    }
}

/// **`version` satisfait-elle `intervalle`** (règles de npm, voir le module) ?
/// Un intervalle illisible est une erreur qui le nomme.
pub fn satisfies(range: &str, version_text: &str) -> Result<bool, String> {
    let x = version(version_text).ok_or_else(|| format!("version illisible : « {version_text} »"))?;
    for alternative in range.split("||") {
        let alternative = alternative.trim();
        // `a - b` : de a à b, bornes comprises.
        let cmps: Vec<Cmp> = if let Some((a, b)) = alternative.split_once(" - ") {
            let mut c = termes(&format!(">={}", a.trim())).ok_or_else(|| format!("intervalle illisible : « {range} »"))?;
            c.extend(termes(&format!("<={}", b.trim())).ok_or_else(|| format!("intervalle illisible : « {range} »"))?);
            c
        } else {
            let mut c = Vec::new();
            // `>= 1.2` s'écrit aussi avec un espace après l'opérateur.
            let mut mots = alternative.split_whitespace().peekable();
            while let Some(m) = mots.next() {
                let terme = if ["<", "<=", ">", ">=", "=", "^", "~"].contains(&m) {
                    format!("{m}{}", mots.next().unwrap_or(""))
                } else {
                    m.to_string()
                };
                c.extend(termes(&terme).ok_or_else(|| format!("intervalle illisible : « {range} »"))?);
            }
            c
        };
        // Une préversion ne passe que si un comparateur nomme une préversion du même triplet.
        if x.pre.is_some() && !cmps.iter().any(|c| matches!(c, Cmp::Ge(b) | Cmp::Gt(b) | Cmp::Lt(b) | Cmp::Le(b) | Cmp::Eq(b) if b.pre.is_some() && b.t == x.t)) {
            continue;
        }
        if cmps.iter().all(|c| vrai(c, &x)) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::satisfies;

    #[test]
    fn npm_ranges_are_judged_like_npm() {
        let oui = |r: &str, v: &str| assert!(satisfies(r, v).unwrap(), "{v} devrait satisfaire « {r} »");
        let non = |r: &str, v: &str| assert!(!satisfies(r, v).unwrap(), "{v} ne devrait pas satisfaire « {r} »");
        // Un nombre nu reste exact : les libs.json d'avant gardent leur sens.
        oui("19.2.0", "19.2.0");
        non("19.2.0", "19.2.1");
        oui("=1.2.3", "1.2.3");
        // ^ et ~ comme npm, 0.x compris.
        oui("^6.7.0", "6.7.6");
        oui("^6.0.0", "6.43.14");
        non("^6.7.0", "7.0.0");
        non("^6.7.0", "6.6.9");
        oui("^0.27.0", "0.27.3");
        non("^0.27.0", "0.28.0");
        non("^0.0.3", "0.0.4");
        oui("~1.2.3", "1.2.9");
        non("~1.2.3", "1.3.0");
        // Jokers, bornes, intervalles, alternatives.
        oui("*", "3.1.4");
        oui("", "3.1.4");
        oui("1.x", "1.9.0");
        non("1.x", "2.0.0");
        oui("1.2.*", "1.2.7");
        oui(">=1.0.0 <2.0.0", "1.5.0");
        non(">=1.0.0 <2.0.0", "2.0.0");
        oui(">= 1.2", "1.3.0");
        oui("1.0.0 - 1.4.0", "1.4.0");
        non("1.0.0 - 1.4.0", "1.4.1");
        oui("^16.8.0 || ^17.0.0 || ^18.0.0 || ^19.0.0", "19.2.0");
        non("^16.8.0 || ^17.0.0", "19.2.0");
        // Une préversion ne satisfait qu'un comparateur de préversion du même triplet.
        non("^1.0.0", "1.1.0-beta.1");
        oui(">=1.1.0-beta.0 <1.2.0", "1.1.0-beta.1");
        assert!(satisfies("^?!", "1.0.0").is_err());
    }
}
