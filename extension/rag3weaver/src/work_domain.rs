//! **Le domaine de travail** — ce qu'un agent a dans sa vision.
//!
//! Pas un *espace* de travail : un espace évoque un endroit, un projet, une
//! chose où l'on range. Un domaine se **disperse** — trois dépôts, un
//! sous-dossier d'un quatrième, et les notes qui traînent ailleurs sur le
//! disque. C'est le nom que Lucie a choisi, et il dit la bonne chose.
//!
//! Voir [doc 05](../docs/26-aout-2026-20h29/05-origine-cellule-domaine.md) §3.
//!
//! ## Une sélection, pas un contenant
//!
//! Rien n'est *rangé* dans un domaine ; tout y est **reconnu**. C'est le
//! choix qui décide de tout le reste : un contenant, il faut y penser, et il
//! périme. Une sélection s'évalue à chaque usage, se compose avec les filtres
//! de recherche qu'on a déjà, et ne coûte rien à changer.
//!
//! ## Trois règles, pour qu'il ne devienne pas une cinquième « racine »
//!
//! 1. **Il ne donne jamais un droit.** Il rétrécit la vision *à l'intérieur*
//!    de ce que la politique de lecture ([`crate::code_tools::RootPolicy`])
//!    autorise déjà. Vision et permission sont deux axes ; les confondre,
//!    c'est refaire l'erreur du doc 04.
//! 2. **Son défaut n'est pas « tout ».** Un agent sans domaine déclaré voit
//!    l'origine du fichier sur lequel il travaille ([`WorkDomain::around`]),
//!    pas le disque. C'est la réponse à « j'ai indexé tout mon disque, je
//!    lance un agent, il est perdu ».
//! 3. **Il dit ce qu'il ne montre pas** ([`WorkDomain::describe`]). Sans ça
//!    l'absence est invisible — la famille de défauts qu'on passe nos
//!    journées à débusquer.
//!
//! ## Ce à quoi on l'attache
//!
//! À rien en particulier, et c'est voulu : un domaine est un objet nommé.
//! Un agent y est lié, mais une boucle, une fiche, un abonnement d'écoute ou
//! une session peuvent l'être aussi. Le faire appartenir à l'agent aurait
//! interdit tout le reste.

use serde::{Deserialize, Serialize};

use crate::filter::{FilterCondition, FilterOp, FilterValue};

/// Un critère de reconnaissance. Les champs remplis sont combinés en **et** ;
/// dans un champ, les valeurs sont combinées en **ou**.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selector {
    /// D'où viennent les octets : `file`, `snapshot:…`.
    #[serde(default)]
    pub sources: Vec<String>,
    /// La coordonnée portable d'un dépôt — `github.com/org/dépôt`.
    #[serde(default)]
    pub repos: Vec<String>,
    /// Des préfixes de chemin. C'est ce qui permet à un domaine de ne prendre
    /// qu'une branche d'un dépôt.
    #[serde(default)]
    pub under: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
}

impl Selector {
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty() && self.repos.is_empty() && self.under.is_empty() && self.languages.is_empty()
    }

    /// Les conditions de ce critère, sur l'entité dont le chemin est
    /// `path_field` (`path` pour un fichier, `file_path` pour un scope).
    fn conditions(&self, path_field: &str) -> Vec<FilterCondition> {
        let mut out = Vec::new();
        let list = |values: &[String]| FilterValue::List(values.iter().map(|v| crate::connection::CypherValue::String(v.clone())).collect());

        if !self.sources.is_empty() {
            out.push(FilterCondition::Field { key: "source".into(), value: list(&self.sources) });
        }
        if !self.repos.is_empty() {
            out.push(FilterCondition::Field { key: "repo".into(), value: list(&self.repos) });
        }
        if !self.languages.is_empty() {
            out.push(FilterCondition::Field { key: "language".into(), value: list(&self.languages) });
        }
        // Plusieurs préfixes : l'un **ou** l'autre.
        if !self.under.is_empty() {
            let any = self
                .under
                .iter()
                .map(|p| FilterCondition::Field {
                    key: path_field.to_string(),
                    value: FilterValue::Ops(vec![FilterOp::StartsWith(p.clone())]),
                })
                .collect();
            out.push(FilterCondition::Should(any));
        }
        out
    }
}

/// Un domaine nommé : ce qu'on reconnaît, et ce qu'on écarte.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkDomain {
    pub name: String,
    /// Ce qui est en vision. **Vide = tout** — mais un appelant ne devrait
    /// jamais construire ça par défaut ; voir [`Self::around`].
    #[serde(default)]
    pub include: Vec<Selector>,
    /// Ce qui en est retiré, quoi qu'en dise `include`. Une exclusion
    /// l'emporte toujours : c'est la seule façon d'écrire « tout ce dépôt
    /// sauf ses fixtures » sans énumérer le reste.
    #[serde(default)]
    pub exclude: Vec<Selector>,
}

impl WorkDomain {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), ..Default::default() }
    }

    pub fn including(mut self, s: Selector) -> Self {
        self.include.push(s);
        self
    }

    pub fn excluding(mut self, s: Selector) -> Self {
        self.exclude.push(s);
        self
    }

    /// **Le défaut dérivé** : le dépôt du fichier sur lequel on travaille.
    ///
    /// C'est la règle n° 2 du module, sous sa forme utile. Un agent qu'on
    /// lâche sans rien déclarer ne voit pas le disque : il voit l'origine de
    /// ce qu'il touche. Si le fichier n'est dans aucun dépôt, on retombe sur
    /// son répertoire — étroit, donc juste.
    pub fn around(absolute_path: &std::path::Path) -> Self {
        let origin = crate::origin::Origin::discover(absolute_path, "");
        let name = format!("autour de {}", absolute_path.display());
        let coords = crate::origin::CoordinateRegistry::default().of(absolute_path);
        match coords.get("repo") {
            Some(repo) => Self::new(&name).including(Selector { repos: vec![repo.clone()], ..Default::default() }),
            None => {
                let dir = origin.anchor.to_string_lossy().to_string();
                Self::new(&name).including(Selector { under: vec![dir], ..Default::default() })
            }
        }
    }

    /// Tout, explicitement. Un domaine vide et un domaine « tout » sont la
    /// même chose pour le moteur, mais pas pour qui lit le journal.
    pub fn everything() -> Self {
        Self::new("tout")
    }

    pub fn is_everything(&self) -> bool {
        self.include.iter().all(Selector::is_empty) && self.exclude.iter().all(Selector::is_empty)
    }

    /// La condition de recherche correspondante, ou `None` quand le domaine
    /// ne restreint rien — auquel cas il ne faut surtout pas fabriquer un
    /// filtre vide, qui coûterait sans rien filtrer.
    ///
    /// `path_field` : `path` sur un fichier, `file_path` sur un scope.
    pub fn to_filter(&self, path_field: &str) -> Option<FilterCondition> {
        let mut must: Vec<FilterCondition> = Vec::new();

        let alternatives: Vec<FilterCondition> = self
            .include
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| FilterCondition::Must(s.conditions(path_field)))
            .collect();
        if !alternatives.is_empty() {
            // Plusieurs critères d'inclusion : l'un **ou** l'autre. Un
            // domaine dispersé est une union, pas une intersection — sinon
            // « trois dépôts » ne voudrait rien dire.
            must.push(FilterCondition::Should(alternatives));
        }

        let refused: Vec<FilterCondition> = self
            .exclude
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| FilterCondition::Must(s.conditions(path_field)))
            .collect();
        if !refused.is_empty() {
            must.push(FilterCondition::MustNot(vec![FilterCondition::Should(refused)]));
        }

        (!must.is_empty()).then(|| FilterCondition::Must(must))
    }

    /// **Ce qu'il montre, et ce qu'il cache** — une ligne, pour le rendu.
    ///
    /// La règle n° 3. Un agent qui ne trouve pas doit pouvoir distinguer
    /// « ça n'existe pas » de « ce n'est pas dans mon champ », sinon
    /// l'absence est un mensonge par omission.
    pub fn describe(&self) -> String {
        if self.is_everything() {
            return "vision : tout l'index".to_string();
        }
        let mut parts = Vec::new();
        for s in self.include.iter().filter(|s| !s.is_empty()) {
            let mut bits = Vec::new();
            if !s.repos.is_empty() {
                bits.push(s.repos.join(", "));
            }
            if !s.under.is_empty() {
                bits.push(s.under.join(", "));
            }
            if !s.sources.is_empty() {
                bits.push(s.sources.join(", "));
            }
            if !s.languages.is_empty() {
                bits.push(format!("en {}", s.languages.join("/")));
            }
            parts.push(bits.join(" sous "));
        }
        let mut out = format!("vision : {}", if parts.is_empty() { "tout l'index".to_string() } else { parts.join(" + ") });
        let refused: Vec<String> = self
            .exclude
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| [s.repos.clone(), s.under.clone(), s.languages.clone()].concat().join(", "))
            .collect();
        if !refused.is_empty() {
            out.push_str(&format!(" · hors champ : {}", refused.join(" ; ")));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rag3db() -> Selector {
        Selector { repos: vec!["github.com/L-Defraiteur/rag3db".into()], ..Default::default() }
    }

    #[test]
    fn a_domain_that_restricts_nothing_produces_no_filter() {
        // Le piège serait de rendre `Must([])` : une clause qui coûte et ne
        // filtre rien, donc un ralentissement invisible.
        assert!(WorkDomain::everything().to_filter("path").is_none());
        assert!(WorkDomain::new("vide").including(Selector::default()).to_filter("path").is_none());
        assert!(WorkDomain::everything().is_everything());
    }

    #[test]
    fn several_inclusions_are_a_union_and_an_exclusion_always_wins() {
        let d = WorkDomain::new("mien")
            .including(rag3db())
            .including(Selector { under: vec!["/home/lucied/notes".into()], ..Default::default() })
            .excluding(Selector { under: vec!["/home/lucied/notes/brouillons".into()], ..Default::default() });

        let f = d.to_filter("file_path").expect("un filtre");
        let json = serde_json::to_string(&f).unwrap();
        // Un domaine dispersé est une union : sinon « deux dépôts » serait
        // « les fichiers qui sont dans les deux à la fois », c'est-à-dire rien.
        assert!(json.contains("should"), "{json}");
        assert!(json.contains("must_not"), "{json}");
        assert!(json.contains("github.com/L-Defraiteur/rag3db") && json.contains("brouillons"), "{json}");
    }

    #[test]
    fn the_path_field_follows_the_entity() {
        let d = WorkDomain::new("d").including(Selector { under: vec!["/x".into()], ..Default::default() });
        assert!(serde_json::to_string(&d.to_filter("path").unwrap()).unwrap().contains("\"path\""));
        assert!(serde_json::to_string(&d.to_filter("file_path").unwrap()).unwrap().contains("file_path"));
    }

    /// La règle n° 2 : le défaut est étroit et **dérivé**, pas large et
    /// déclaré.
    #[test]
    fn the_default_domain_is_the_repo_of_the_file_being_worked_on() {
        let here = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("src/work_domain.rs");
        let d = WorkDomain::around(&here);
        assert!(!d.is_everything(), "un agent lâché sans rien ne voit pas le disque");
        let repos: Vec<&String> = d.include.iter().flat_map(|s| s.repos.iter()).collect();
        assert!(repos.iter().any(|r| r.contains("rag3db")), "{:?}", d.include);
        assert!(d.describe().contains("rag3db"), "{}", d.describe());

        // Hors de tout dépôt : on retombe sur un répertoire, donc étroit —
        // jamais sur « tout ».
        let dir = std::env::temp_dir().join(format!("rag3weaver-domaine-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let orphan = dir.join("seul.rs");
        std::fs::write(&orphan, "\n").unwrap();
        let d = WorkDomain::around(&orphan);
        assert!(!d.is_everything(), "{d:?}");
        assert!(d.to_filter("path").is_some(), "{d:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// La règle n° 3 : il dit ce qu'il cache.
    #[test]
    fn a_domain_says_what_it_hides() {
        let d = WorkDomain::new("code")
            .including(rag3db())
            .excluding(Selector { under: vec!["extension/rag3weaver/tests".into()], ..Default::default() });
        let said = d.describe();
        assert!(said.contains("vision :") && said.contains("hors champ :"), "{said}");
        assert!(said.contains("tests"), "{said}");
        assert_eq!(WorkDomain::everything().describe(), "vision : tout l'index");
    }

    #[test]
    fn a_domain_survives_a_round_trip() {
        let d = WorkDomain::new("mien").including(rag3db()).excluding(Selector { languages: vec!["markdown".into()], ..Default::default() });
        let back: WorkDomain = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(back, d, "un domaine se sauvegarde et se relit — c'est un objet, pas un paramètre");
    }
}
