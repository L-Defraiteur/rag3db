//! **Les appels d'outil qu'on lit** (lot tty-11) : chaque appel numéroté, un
//! aperçu de ce qu'il a lu ou écrit dans l'historique, et le tout pour la
//! visionneuse (Ctrl-O).
//!
//! L'hôte rend un extrait du résultat (`result_excerpt`, ses 4 premiers Ko) et
//! la taille entière (`bytes`). Un `read_file` y porte le chemin, la plage de
//! lignes et le contenu entre deux clôtures, chaque ligne préfixée de son
//! numéro (`00001| `). Un extrait coupé se dit, il ne paraît jamais entier.

use crate::vue::{Genre, Validee};
use serde_json::Value;

/// Un appel d'outil, de son début à sa fin.
#[derive(Debug, Clone, PartialEq)]
pub struct Appel {
    /// Son numéro dans la séance, à partir de 1.
    pub n: usize,
    pub outil: String,
    pub arguments: Value,
    /// `None` tant qu'il tourne.
    pub ok: Option<bool>,
    pub ms: u64,
    pub erreur: Option<String>,
    pub extrait: Option<String>,
    pub tronque: bool,
    /// La taille entière du résultat, en octets.
    pub octets: Option<u64>,
}

/// Ce qu'un `read_file` a lu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lecture {
    pub chemin: String,
    /// « lignes 1-35 sur 35 ».
    pub plage: String,
    pub lignes: Vec<String>,
}

impl Appel {
    pub fn debut(n: usize, outil: &str, arguments: &str) -> Self {
        Self { n, outil: outil.to_string(), arguments: serde_json::from_str(arguments).unwrap_or(Value::Null), ok: None, ms: 0, erreur: None, extrait: None, tronque: false, octets: None }
    }

    /// Le fichier (ou le motif) que vise l'appel.
    pub fn cible(&self) -> Option<String> {
        ["path", "file_path", "pattern", "query", "path_prefix"].iter().find_map(|k| self.arguments[k].as_str().filter(|v| !v.is_empty()).map(str::to_string))
    }

    /// La ligne d'historique de son début : `» read_file src/vue.rs  ⟨3⟩`.
    pub fn ligne(&self) -> String {
        match self.cible() {
            Some(c) => format!("» {} {c}  ⟨{}⟩", self.outil, self.n),
            None => format!("» {}  ⟨{}⟩", self.outil, self.n),
        }
    }

    /// Ce que l'appel a lu, s'il a lu un fichier.
    pub fn lecture(&self) -> Option<Lecture> {
        if self.outil != "read_file" {
            return None;
        }
        let resultat = chaine_json(self.extrait.as_deref()?, "result")?;
        let mut lignes_brutes = resultat.lines();
        let tete = lignes_brutes.next()?;
        let (chemin, plage) = tete.split_once(" — lines ")?;
        let chemin = chemin.trim_matches('*').to_string();
        let plage = match plage.split_once(" of ") {
            Some((a, n)) => format!("lignes {a} sur {n}"),
            None => format!("lignes {plage}"),
        };
        let mut lignes = Vec::new();
        let mut dedans = false;
        for l in lignes_brutes {
            if l.starts_with("```") {
                if dedans {
                    break;
                }
                dedans = true;
                continue;
            }
            if dedans {
                let sans_numero = match l.split_once("| ") {
                    Some((num, reste)) if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) => reste,
                    _ if l.chars().all(|c| c.is_ascii_digit() || c == '|') => "",
                    _ => l,
                };
                lignes.push(sans_numero.to_string());
            }
        }
        Some(Lecture { chemin, plage, lignes })
    }

    /// Ce que l'appel écrit : `(langue, texte)`, un remplacement en `diff`.
    fn ecriture(&self) -> Option<(String, String)> {
        if self.outil != "edit_file" {
            return None;
        }
        let ext = extension(&self.cible()?);
        if let (Some(old), Some(new)) = (self.arguments["old"].as_str(), self.arguments["new"].as_str()) {
            let mut d: Vec<String> = old.lines().map(|l| format!("- {l}")).collect();
            d.extend(new.lines().map(|l| format!("+ {l}")));
            return Some(("diff".into(), d.join("\n")));
        }
        self.arguments["content"].as_str().map(|c| (ext, c.to_string()))
    }

    /// Ce que l'appel a lu ou écrit, en lignes validées, et sa plage ou sa cible.
    fn contenu(&self, max: Option<usize>) -> Option<(Vec<Validee>, usize, String)> {
        let (langue, lignes, total, quoi): (String, Vec<String>, usize, String) = if let Some(l) = self.lecture() {
            let total = l.plage.rsplit(' ').next().and_then(|n| n.parse().ok()).unwrap_or(l.lignes.len());
            (extension(&l.chemin), l.lignes, total, format!("{} · {}", l.chemin, l.plage))
        } else if let Some((langue, texte)) = self.ecriture() {
            let lignes: Vec<String> = texte.lines().map(str::to_string).collect();
            let n = lignes.len();
            (langue, lignes, n, self.cible().unwrap_or_default())
        } else {
            return None;
        };
        let montrees = max.map(|m| lignes.len().min(m)).unwrap_or(lignes.len());
        let texte = lignes[..montrees].join("\n");
        let v = if matches!(langue.as_str(), "md" | "mdx" | "markdown") { blocs(&texte, Genre::Texte) } else { vec![Validee { genre: Genre::Code, texte, langue, tete: 0 }] };
        Some((v, total.saturating_sub(montrees), quoi))
    }

    /// Ce que dit un extrait coupé : « extrait, 4 Ko sur 16 Ko ».
    fn coupe(&self) -> Option<String> {
        self.tronque.then(|| format!("extrait, {} Ko sur {} Ko", ko(self.extrait.as_ref().map(|e| e.len() as u64).unwrap_or(0)), ko(self.octets.unwrap_or(0))))
    }

    /// **L'aperçu** qui suit la fin de l'appel dans l'historique : au plus `max`
    /// lignes de ce qu'il a lu ou écrit, puis ce qui reste à voir par Ctrl-O.
    pub fn apercu(&self, max: usize) -> Vec<Validee> {
        let Some((mut v, reste, quoi)) = self.contenu(Some(max)) else { return Vec::new() };
        let mut dit = Vec::new();
        if reste > 0 {
            dit.push(format!("{reste} lignes de plus : Ctrl-O ⟨{}⟩", self.n));
        }
        if let Some(c) = self.coupe() {
            dit.push(c);
        }
        if !dit.is_empty() {
            v.push(note(format!("  {quoi} · {}", dit.join(" · "))));
        }
        v
    }

    /// **Tout ce qu'on sait de l'appel**, pour la visionneuse.
    pub fn complet(&self) -> Vec<Validee> {
        let etat = match (self.ok, &self.erreur) {
            (None, _) => "en cours".to_string(),
            (Some(true), _) => format!("fait ({} ms)", self.ms),
            (Some(false), e) => format!("refusé ({})", e.as_deref().unwrap_or("erreur")),
        };
        let mut v = vec![Validee { genre: Genre::Outil, texte: format!("{} — {etat}", self.ligne()), langue: String::new(), tete: 0 }];
        if !self.arguments.is_null() {
            v.push(note(format!("arguments : {}", self.arguments)));
        }
        match self.contenu(None) {
            Some((c, _, _)) => v.extend(c),
            None => {
                if let Some(e) = &self.extrait {
                    let texte = chaine_json(e, "result").unwrap_or_else(|| e.clone());
                    v.extend(blocs(&texte, Genre::Texte));
                }
            }
        }
        if let Some(c) = self.coupe() {
            v.push(note(c));
        }
        v
    }
}

fn note(texte: String) -> Validee {
    Validee { genre: Genre::Note, texte, langue: String::new(), tete: 0 }
}

/// Des octets en Ko arrondis.
fn ko(o: u64) -> u64 {
    (o + 512) / 1024
}

/// L'extension d'un chemin, la langue que comprend `/highlight`.
fn extension(chemin: &str) -> String {
    std::path::Path::new(chemin).extension().and_then(|e| e.to_str()).unwrap_or_default().to_lowercase()
}

/// **Du Markdown en lignes validées** : la prose ligne à ligne, chaque bloc
/// clos entre deux clôtures en un bloc de code de sa langue.
pub fn blocs(texte: &str, genre_de_prose: Genre) -> Vec<Validee> {
    let prose = |t: &str| Validee { genre: genre_de_prose, texte: t.to_string(), langue: String::new(), tete: 0 };
    let mut out = Vec::new();
    let mut ouvert: Option<(String, String, Vec<String>)> = None;
    for l in texte.split('\n') {
        let cloture = l.trim_start().starts_with("```");
        match (&mut ouvert, cloture) {
            (Some(_), true) => {
                let (_, langue, corps) = ouvert.take().unwrap_or_default();
                out.push(Validee { genre: Genre::Code, texte: corps.join("\n"), langue, tete: 0 });
            }
            (Some((_, _, corps)), false) => corps.push(l.to_string()),
            (None, true) => ouvert = Some((l.to_string(), l.trim_start().trim_start_matches('`').trim().to_string(), Vec::new())),
            (None, false) => out.push(prose(l)),
        }
    }
    // Un bloc jamais clos reste de la prose, sa clôture comprise.
    if let Some((cloture, _, corps)) = ouvert {
        out.push(prose(&cloture));
        out.extend(corps.iter().map(|l| prose(l)));
    }
    out
}

/// La valeur de la chaîne JSON `"cle":"…"` dans `json`, même coupé au milieu :
/// l'extrait de l'hôte s'arrête à 4 Ko, au milieu d'une chaîne s'il le faut.
pub fn chaine_json(json: &str, cle: &str) -> Option<String> {
    let motif = format!("\"{cle}\":\"");
    let debut = json.find(&motif)? + motif.len();
    let mut out = String::new();
    let mut c = json[debut..].chars();
    while let Some(x) = c.next() {
        match x {
            '"' => break,
            '\\' => match c.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('u') => {
                    let h: String = c.by_ref().take(4).collect();
                    match u32::from_str_radix(&h, 16).ok().and_then(char::from_u32) {
                        Some(ch) if h.len() == 4 => out.push(ch),
                        _ => break,
                    }
                }
                Some(o) => out.push(o),
                None => break,
            },
            o => out.push(o),
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn lu(chemin: &str, lignes: &[&str], total: usize) -> String {
        let corps: String = lignes.iter().enumerate().map(|(i, l)| format!("{:05}| {l}\n", i + 1)).collect();
        let result = format!("**{chemin}** — lines 1-{} of {total}\nScopes: texte_brut `{chemin}:1-{total}`\n```\n{corps}```", lignes.len());
        json!({ "delivery": { "ok": true }, "result": result }).to_string()
    }

    fn fini(mut a: Appel, extrait: String, octets: u64) -> Appel {
        a.ok = Some(true);
        a.ms = 12;
        a.tronque = extrait.len() as u64 > 4096;
        a.extrait = Some(if a.tronque { extrait[..4096].to_string() } else { extrait });
        a.octets = Some(octets);
        a
    }

    /// Chaque appel est numéroté, et dit ce qu'il vise.
    #[test]
    fn a_call_is_numbered_and_says_what_it_targets() {
        let a = Appel::debut(3, "read_file", r#"{"path":"src/vue.rs","offset":1,"limit":50}"#);
        assert_eq!(a.cible().as_deref(), Some("src/vue.rs"));
        assert_eq!(a.ligne(), "» read_file src/vue.rs  ⟨3⟩");
        let g = Appel::debut(4, "grep_files", r#"{"pattern":"fn main"}"#);
        assert_eq!(g.ligne(), "» grep_files fn main  ⟨4⟩");
        assert_eq!(Appel::debut(5, "step_done", "{}").ligne(), "» step_done  ⟨5⟩");
        assert_eq!(Appel::debut(6, "x", "pas du json").ligne(), "» x  ⟨6⟩", "des arguments illisibles ne cassent rien");
    }

    /// Un `read_file` se relit : son chemin, sa plage, ses lignes sans leur numéro.
    #[test]
    fn a_read_file_excerpt_gives_its_path_range_and_lines() {
        let a = fini(Appel::debut(1, "read_file", r#"{"path":"src/a.rs"}"#), lu("src/a.rs", &["fn main() {", "    println!(\"é\");", "}"], 3), 200);
        assert_eq!(a.lecture(), Some(Lecture { chemin: "src/a.rs".into(), plage: "lignes 1-3 sur 3".into(), lignes: vec!["fn main() {".into(), "    println!(\"é\");".into(), "}".into()] }));
    }

    /// L'aperçu d'une lecture : un bloc de code dans la langue du fichier, au
    /// plus `max` lignes, puis combien il en reste et la touche qui les montre.
    #[test]
    fn the_preview_of_a_read_is_a_code_block_cut_at_max_lines() {
        let lignes: Vec<String> = (1..=30).map(|i| format!("let x{i} = {i};")).collect();
        let refs: Vec<&str> = lignes.iter().map(String::as_str).collect();
        let a = fini(Appel::debut(2, "read_file", r#"{"path":"src/b.rs"}"#), lu("src/b.rs", &refs, 30), 900);
        let v = a.apercu(12);
        assert_eq!((v[0].genre, v[0].langue.as_str()), (Genre::Code, "rs"));
        assert_eq!(v[0].texte.lines().count(), 12);
        assert_eq!(v[1].genre, Genre::Note);
        assert_eq!(v[1].texte, "  src/b.rs · lignes 1-30 sur 30 · 18 lignes de plus : Ctrl-O ⟨2⟩");
    }

    /// Un `.md` lu se rend comme de la prose, ses blocs de code dans leur langue.
    #[test]
    fn a_markdown_file_renders_as_prose_with_its_fenced_blocks_in_their_language() {
        let a = fini(Appel::debut(1, "read_file", r#"{"path":"docs/a.md"}"#), lu("docs/a.md", &["# Titre", "Du texte.", "```typescript", "class SomeClass {}", "```", "Fin."], 6), 100);
        let v = a.apercu(12);
        let vus: Vec<(Genre, &str, &str)> = v.iter().map(|l| (l.genre, l.langue.as_str(), l.texte.as_str())).collect();
        assert_eq!(&vus[..4], &[(Genre::Texte, "", "# Titre"), (Genre::Texte, "", "Du texte."), (Genre::Code, "typescript", "class SomeClass {}"), (Genre::Texte, "", "Fin.")]);
    }

    /// Un extrait coupé à 4 Ko le dit, avec la taille entière : il ne paraît
    /// jamais entier, ni dans l'aperçu ni dans la visionneuse.
    #[test]
    fn a_cut_excerpt_says_it_is_an_excerpt_of_n_bytes() {
        let lignes: Vec<String> = (1..=400).map(|i| format!("ligne {i} assez longue pour remplir")).collect();
        let refs: Vec<&str> = lignes.iter().map(String::as_str).collect();
        let a = fini(Appel::debut(7, "read_file", r#"{"path":"src/c.rs"}"#), lu("src/c.rs", &refs, 400), 16_000);
        assert!(a.tronque);
        let l = a.lecture().unwrap();
        assert!(l.lignes.len() > 50 && l.lignes.len() < 400, "{}", l.lignes.len());
        assert!(a.apercu(12).last().unwrap().texte.contains("extrait, 4 Ko sur 16 Ko"), "{:?}", a.apercu(12).last());
        assert!(a.complet().last().unwrap().texte.contains("extrait, 4 Ko sur 16 Ko"));
    }

    /// Une écriture se montre : le remplacement en lignes retirées et ajoutées,
    /// ou le début du contenu écrit, dans la langue du fichier.
    #[test]
    fn a_write_shows_its_replacement_or_its_content() {
        let mut e = Appel::debut(1, "edit_file", r#"{"path":"src/d.rs","old":"let a = 1;\nlet b = 2;","new":"let a = 3;"}"#);
        e.ok = Some(true);
        let v = e.apercu(12);
        assert_eq!((v[0].genre, v[0].langue.as_str(), v[0].texte.as_str()), (Genre::Code, "diff", "- let a = 1;\n- let b = 2;\n+ let a = 3;"));
        let mut c = Appel::debut(2, "edit_file", r#"{"path":"src/e.py","content":"x = 1\ny = 2"}"#);
        c.ok = Some(true);
        assert_eq!((c.apercu(12)[0].langue.as_str(), c.apercu(12)[0].texte.as_str()), ("py", "x = 1\ny = 2"));
    }

    /// La chaîne d'une clé se lit même coupée, échappements compris.
    #[test]
    fn a_json_string_reads_even_when_cut() {
        assert_eq!(chaine_json(r#"{"a":1,"result":"un\n\"deux\" é"}"#, "result").as_deref(), Some("un\n\"deux\" é"));
        assert_eq!(chaine_json(r#"{"result":"coupé au mil"#, "result").as_deref(), Some("coupé au mil"));
        assert_eq!(chaine_json(r#"{"result":"fin\"#, "result").as_deref(), Some("fin"), "un échappement coupé tombe");
        assert_eq!(chaine_json(r#"{"autre":"x"}"#, "result"), None);
    }

    /// Le Markdown en blocs : la prose ligne à ligne, un bloc clos dans sa
    /// langue ; un bloc jamais clos reste de la prose.
    #[test]
    fn markdown_splits_into_prose_lines_and_closed_code_blocks() {
        let v = blocs("Voici :\n```rust\nfn a() {}\n```\nfin\n```\nouvert", Genre::Boite);
        let vus: Vec<(Genre, &str, &str)> = v.iter().map(|l| (l.genre, l.langue.as_str(), l.texte.as_str())).collect();
        assert_eq!(vus, vec![(Genre::Boite, "", "Voici :"), (Genre::Code, "rust", "fn a() {}"), (Genre::Boite, "", "fin"), (Genre::Boite, "", "```"), (Genre::Boite, "", "ouvert")]);
    }
}
