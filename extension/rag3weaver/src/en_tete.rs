//! **L'en-tête YAML d'un fichier d'entité** (lot M2 de la mémoire, 18 octobre
//! 2026 ; `docs/18-octobre-2026-memoire-longue/04-les-fiches-dans-des-fichiers.md`).
//!
//! Un fichier Markdown commence par un en-tête entre deux lignes `---`, comme
//! dans Obsidian. Deux fonctions seulement, derrière lesquelles se cache
//! l'analyseur (`serde_norway`) :
//!
//! - [`ecrire_en_tete`] écrit **toujours la même forme** : les clés dans l'ordre
//!   donné, les chaînes entre guillemets doubles (aucune n'est jamais devinée
//!   comme un booléen, un nombre ou une date), les listes en tirets, les accents
//!   et les guillemets typographiques tels quels. Un en-tête écrit par elle se
//!   relit et se réécrit à l'identique, octet pour octet.
//! - [`lire_en_tete`] lit un en-tête **guidé par le schéma JSON de l'entité** : le
//!   type d'un champ est celui que le schéma déclare, jamais celui que YAML
//!   devine. `no`, `yes`, `on`, `off`, `y`, `n`, `1.0`, `0123` restent des
//!   chaînes là où le schéma attend une chaîne — c'est ce qu'Obsidian montre, et
//!   c'est ce que la personne a écrit. Un champ que le schéma ne connaît pas est
//!   refusé quand il interdit les propriétés en plus.
//!
//! Le corps (tout ce qui suit la seconde ligne `---`) n'est pas touché.

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};
use std::fmt;

/// La ligne qui ouvre et ferme l'en-tête.
pub const DELIMITEUR: &str = "---";

/// **Sépare un fichier** en (en-tête YAML, corps). `None` si le fichier ne
/// commence pas par un en-tête fermé.
pub fn separer(texte: &str) -> Option<(&str, &str)> {
    let reste = texte.strip_prefix("---\n").or_else(|| texte.strip_prefix("---\r\n"))?;
    let mut debut = 0;
    for ligne in reste.split_inclusive('\n') {
        if ligne.trim_end_matches(['\r', '\n']) == DELIMITEUR {
            let yaml = &reste[..debut];
            let corps = &reste[debut + ligne.len()..];
            return Some((yaml, corps));
        }
        debut += ligne.len();
    }
    None
}

/// **Assemble un fichier** : l'en-tête écrit par [`ecrire_en_tete`] entre ses
/// deux lignes, puis le corps.
pub fn assembler(en_tete: &str, corps: &str) -> String {
    format!("{DELIMITEUR}\n{en_tete}{DELIMITEUR}\n{corps}")
}

/// **Écrit l'en-tête**, dans la forme canonique, sans les lignes `---`. `ordre`
/// donne l'ordre des clés ; une clé de `champs` absente d'`ordre` vient
/// ensuite, par ordre alphabétique ; une valeur nulle n'est pas écrite.
pub fn ecrire_en_tete(champs: &Map<String, Value>, ordre: &[&str]) -> String {
    let mut cles: Vec<&str> = ordre.iter().copied().filter(|c| champs.contains_key(*c)).collect();
    let mut autres: Vec<&str> = champs.keys().map(String::as_str).filter(|c| !ordre.contains(c)).collect();
    autres.sort_unstable();
    cles.extend(autres);
    let mut out = String::new();
    for cle in cles {
        let v = &champs[cle];
        if v.is_null() {
            continue;
        }
        ecrire_valeur(&mut out, cle, v, 0);
    }
    out
}

fn retrait(n: usize) -> String {
    " ".repeat(n)
}

/// Une chaîne entre guillemets doubles : `\` et `"` échappés, les caractères de
/// contrôle en `\u`, tout le reste (accents, « », ’) tel quel.
fn citer(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Une clé nue si elle est un identifiant simple, citée sinon.
fn cle(c: &str) -> String {
    let simple = !c.is_empty() && c.chars().all(|x| x.is_alphanumeric() || x == '_' || x == '-');
    if simple { c.to_string() } else { citer(c) }
}

fn scalaire(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(citer(s)),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::Null => Some("null".into()),
        _ => None,
    }
}

fn ecrire_valeur(out: &mut String, c: &str, v: &Value, n: usize) {
    let r = retrait(n);
    match v {
        Value::Array(a) if a.is_empty() => out.push_str(&format!("{r}{}: []\n", cle(c))),
        Value::Object(o) if o.is_empty() => out.push_str(&format!("{r}{}: {{}}\n", cle(c))),
        Value::Array(a) => {
            out.push_str(&format!("{r}{}:\n", cle(c)));
            for item in a {
                ecrire_element(out, item, n);
            }
        }
        Value::Object(o) => {
            out.push_str(&format!("{r}{}:\n", cle(c)));
            for (k, x) in o {
                if !x.is_null() {
                    ecrire_valeur(out, k, x, n + 2);
                }
            }
        }
        _ => out.push_str(&format!("{r}{}: {}\n", cle(c), scalaire(v).unwrap())),
    }
}

/// Un élément de liste, en tiret au même retrait que sa clé (la forme
/// d'Obsidian) ; un objet s'écrit `- clé: valeur` puis ses autres clés alignées.
fn ecrire_element(out: &mut String, v: &Value, n: usize) {
    let r = retrait(n);
    match v {
        Value::Object(o) if !o.is_empty() => {
            let mut premiere = true;
            for (k, x) in o.iter().filter(|(_, x)| !x.is_null()) {
                let mut ligne = String::new();
                ecrire_valeur(&mut ligne, k, x, n + 2);
                if premiere {
                    // `  clé: …` devient `- clé: …` sur la première ligne.
                    out.push_str(&format!("{r}- {}", &ligne[n + 2..]));
                    premiere = false;
                } else {
                    out.push_str(&ligne);
                }
            }
            if premiere {
                out.push_str(&format!("{r}- {{}}\n"));
            }
        }
        Value::Array(_) | Value::Object(_) => {
            // Une liste dans une liste, ou un objet vide : en JSON, qui est du YAML.
            out.push_str(&format!("{r}- {}\n", v));
        }
        _ => out.push_str(&format!("{r}- {}\n", scalaire(v).unwrap())),
    }
}

/// **Lit l'en-tête** (sans ses lignes `---`), guidé par `schema` (le schéma JSON
/// de l'entité, un objet). Rend les champs, typés comme le schéma les déclare.
pub fn lire_en_tete(yaml: &str, schema: &Value) -> Result<Map<String, Value>, String> {
    if yaml.trim().is_empty() {
        return Ok(Map::new());
    }
    let de = serde_norway::Deserializer::from_str(yaml);
    match (Graine { schema, chemin: String::new() }).deserialize(de) {
        Ok(Value::Object(m)) => Ok(m),
        Ok(autre) => Err(format!("l'en-tête n'est pas un objet : {autre}")),
        Err(e) => Err(format!("en-tête illisible : {e}")),
    }
}

/// La graine de lecture d'une valeur : son schéma et son chemin (pour nommer
/// le champ dans un refus).
struct Graine<'a> {
    schema: &'a Value,
    chemin: String,
}

fn type_declare(schema: &Value) -> Option<&str> {
    match &schema["type"] {
        Value::String(s) => Some(s.as_str()),
        // `["string", "null"]` : le premier type qui n'est pas null.
        Value::Array(a) => a.iter().filter_map(Value::as_str).find(|t| *t != "null"),
        _ => None,
    }
}

impl<'de> DeserializeSeed<'de> for Graine<'_> {
    type Value = Value;
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
        match type_declare(self.schema) {
            // **La règle qui compte** : une chaîne attendue se lit comme le texte
            // écrit, quel que soit le type que YAML devinerait (`no`, `1.0`, `0123`).
            Some("string") => d.deserialize_str(Texte { chemin: self.chemin }),
            Some("integer") => d.deserialize_i64(Entier { chemin: self.chemin }),
            Some("number") => d.deserialize_f64(Nombre { chemin: self.chemin }),
            Some("boolean") => d.deserialize_bool(Booleen { chemin: self.chemin }),
            Some("array") => d.deserialize_seq(Liste { schema: self.schema, chemin: self.chemin }),
            Some("object") => d.deserialize_map(Objet { schema: self.schema, chemin: self.chemin }),
            _ => <Value as serde::Deserialize>::deserialize(d),
        }
    }
}

fn nomme(chemin: &str) -> String {
    if chemin.is_empty() { "l'en-tête".into() } else { format!("« {chemin} »") }
}

struct Texte {
    chemin: String,
}
impl<'de> Visitor<'de> for Texte {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "une chaîne pour {}", nomme(&self.chemin))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Value, E> {
        Ok(Value::String(v.to_string()))
    }
    fn visit_string<E: de::Error>(self, v: String) -> Result<Value, E> {
        Ok(Value::String(v))
    }
    // Un analyseur qui aurait résolu le scalaire avant de le rendre : on refuse
    // plutôt que de reconstruire un texte qu'on ne connaît plus.
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Value, E> {
        Err(E::custom(format!("{} : une chaîne était attendue, YAML a lu le booléen {v} — citez la valeur", nomme(&self.chemin))))
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Value, E> {
        Err(E::custom(format!("{} : une chaîne était attendue, YAML a lu le nombre {v} — citez la valeur", nomme(&self.chemin))))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
        Err(E::custom(format!("{} : une chaîne était attendue, YAML a lu le nombre {v} — citez la valeur", nomme(&self.chemin))))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Value, E> {
        Err(E::custom(format!("{} : une chaîne était attendue, YAML a lu le nombre {v} — citez la valeur", nomme(&self.chemin))))
    }
}

struct Entier {
    chemin: String,
}
impl<'de> Visitor<'de> for Entier {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "un entier pour {}", nomme(&self.chemin))
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
        Ok(Value::from(v))
    }
}

struct Nombre {
    chemin: String,
}
impl<'de> Visitor<'de> for Nombre {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "un nombre pour {}", nomme(&self.chemin))
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Value, E> {
        serde_json::Number::from_f64(v)
            .map(Value::Number)
            .ok_or_else(|| E::custom(format!("{} : nombre non fini", nomme(&self.chemin))))
    }
}

struct Booleen {
    chemin: String,
}
impl<'de> Visitor<'de> for Booleen {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "true ou false pour {}", nomme(&self.chemin))
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }
}

struct Liste<'a> {
    schema: &'a Value,
    chemin: String,
}
impl<'de> Visitor<'de> for Liste<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "une liste pour {}", nomme(&self.chemin))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let items = &self.schema["items"];
        let mut out = Vec::new();
        let mut i = 0;
        while let Some(v) = seq.next_element_seed(Graine { schema: items, chemin: format!("{}[{i}]", self.chemin) })? {
            out.push(v);
            i += 1;
        }
        Ok(Value::Array(out))
    }
}

struct Objet<'a> {
    schema: &'a Value,
    chemin: String,
}
impl<'de> Visitor<'de> for Objet<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "un objet pour {}", nomme(&self.chemin))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let props = &self.schema["properties"];
        let fermes = self.schema["additionalProperties"] == Value::Bool(false);
        let rien = Value::Null;
        let mut out = Map::new();
        while let Some(k) = map.next_key::<String>()? {
            let chemin = if self.chemin.is_empty() { k.clone() } else { format!("{}.{k}", self.chemin) };
            let sous = match props.get(&k) {
                Some(s) => s,
                None if fermes => {
                    return Err(de::Error::custom(format!(
                        "« {chemin} » : champ inconnu du schéma (connus : {})",
                        props.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>().join(", ")).unwrap_or_default()
                    )))
                }
                None => &rien,
            };
            let v = map.next_value_seed(Graine { schema: sous, chemin })?;
            out.insert(k, v);
        }
        Ok(Value::Object(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema() -> Value {
        json!({
            "type": "object", "additionalProperties": false,
            "properties": {
                "name": {"type": "string"}, "claim": {"type": "string"},
                "subjects": {"type": "array", "items": {"type": "string"}},
                "revision": {"type": "integer"}, "score": {"type": "number"}, "ok": {"type": "boolean"},
                "dit_le": {"type": "string"},
                "refs": {"type": "array", "items": {"type": "object", "additionalProperties": false,
                          "properties": {"genre": {"type": "string"}, "valeur": {"type": "string"}}}}
            }
        })
    }

    fn lire(yaml: &str) -> Map<String, Value> {
        lire_en_tete(yaml, &schema()).unwrap_or_else(|e| panic!("{yaml} : {e}"))
    }

    /// Les mots que YAML 1.1 prend pour des booléens restent des chaînes quand
    /// le schéma attend une chaîne — écrits nus, comme dans Obsidian.
    #[test]
    fn no_yes_on_off_y_n_restent_des_chaines() {
        for mot in ["no", "yes", "on", "off", "y", "n", "No", "TRUE", "false"] {
            let m = lire(&format!("name: {mot}\nsubjects:\n- {mot}\n"));
            assert_eq!(m["name"], json!(mot), "name: {mot}");
            assert_eq!(m["subjects"], json!([mot]), "subjects: [{mot}]");
        }
    }

    /// `1.0` et `0123` gardent leur texte (pas 1 ni 123) là où le schéma attend une chaîne.
    #[test]
    fn un_nombre_ecrit_nu_reste_son_texte_dans_une_chaine() {
        let m = lire("claim: 1.0\nsubjects:\n- 0123\n- 1e3\n- .5\n");
        assert_eq!(m["claim"], json!("1.0"));
        assert_eq!(m["subjects"], json!(["0123", "1e3", ".5"]));
    }

    /// Une date ISO avec fuseau se relit à l'identique, dans un champ chaîne.
    #[test]
    fn une_date_iso_avec_fuseau_se_relit_a_l_identique() {
        for d in ["2026-10-18T21:37:00+02:00", "2026-10-18T19:37:00Z", "2026-10-18"] {
            let m = lire(&format!("dit_le: {d}\n"));
            assert_eq!(m["dit_le"], json!(d));
            let ecrit = ecrire_en_tete(&m, &["dit_le"]);
            assert_eq!(lire(&ecrit)["dit_le"], json!(d), "aller-retour de {d}");
        }
    }

    /// Un `:` dans une valeur est cité par l'écrivain, et se relit.
    #[test]
    fn un_deux_points_dans_une_valeur_est_cite() {
        let mut m = Map::new();
        m.insert("claim".into(), json!("le moteur : la reprise du WAL"));
        let ecrit = ecrire_en_tete(&m, &["claim"]);
        assert_eq!(ecrit, "claim: \"le moteur : la reprise du WAL\"\n");
        assert_eq!(lire(&ecrit)["claim"], json!("le moteur : la reprise du WAL"));
    }

    /// Les accents et les guillemets typographiques passent tels quels, sans échappement.
    #[test]
    fn les_accents_et_les_guillemets_typographiques_passent_tels_quels() {
        let texte = "« Où l’été s’arrête » — déjà vu, “cité”";
        let mut m = Map::new();
        m.insert("claim".into(), json!(texte));
        let ecrit = ecrire_en_tete(&m, &["claim"]);
        assert!(ecrit.contains(texte), "écrit tel quel : {ecrit}");
        assert_eq!(lire(&ecrit)["claim"], json!(texte));
        // Écrit nu à la main, il se lit aussi.
        assert_eq!(lire(&format!("claim: {texte}\n"))["claim"], json!(texte));
    }

    /// La forme canonique se relit et se réécrit à l'identique, octet pour octet.
    #[test]
    fn la_forme_canonique_fait_un_aller_retour_exact() {
        let m = json!({
            "name": "le moteur de reprise du WAL", "revision": 3, "score": 0.5, "ok": true,
            "subjects": ["no", "1.0", "a: b"],
            "refs": [{"genre": "path", "valeur": "git:github.com/x/y:src/a.rs"}, {"genre": "doc", "valeur": "sources/x.md#Reprise"}]
        });
        let m = m.as_object().unwrap().clone();
        let ordre = ["name", "revision", "subjects", "refs", "score", "ok"];
        let une = ecrire_en_tete(&m, &ordre);
        let relu = lire(&une);
        assert_eq!(Value::Object(relu.clone()), Value::Object(m));
        assert_eq!(ecrire_en_tete(&relu, &ordre), une, "réécrit à l'identique");
        assert_eq!(
            une,
            "name: \"le moteur de reprise du WAL\"\nrevision: 3\nsubjects:\n- \"no\"\n- \"1.0\"\n- \"a: b\"\nrefs:\n- genre: \"path\"\n  valeur: \"git:github.com/x/y:src/a.rs\"\n- genre: \"doc\"\n  valeur: \"sources/x.md#Reprise\"\nscore: 0.5\nok: true\n"
        );
    }

    /// Le schéma tranche : un champ inconnu est refusé en le nommant, un entier
    /// attendu qui n'en est pas un aussi.
    #[test]
    fn le_schema_tranche_et_le_refus_nomme_le_champ() {
        let e = lire_en_tete("nom: x\n", &schema()).unwrap_err();
        assert!(e.contains("« nom »"), "{e}");
        let e = lire_en_tete("revision: trois\n", &schema()).unwrap_err();
        assert!(e.contains("revision"), "{e}");
        let e = lire_en_tete("refs:\n- genre: path\n  valuer: x\n", &schema()).unwrap_err();
        assert!(e.contains("refs[0].valuer"), "{e}");
    }

    #[test]
    fn separer_et_assembler_gardent_le_corps() {
        let f = assembler("name: \"x\"\n", "## État\n\ntexte\n---\nune règle horizontale dans le corps\n");
        let (yaml, corps) = separer(&f).expect("un en-tête");
        assert_eq!(yaml, "name: \"x\"\n");
        assert!(corps.starts_with("## État") && corps.contains("---\nune règle"));
        assert!(separer("pas d'en-tête\n").is_none());
        assert!(separer("---\nname: x\nsans fin\n").is_none());
    }
}
