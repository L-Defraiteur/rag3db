//! Jaro et Jaro-Winkler — ordonner un sommet que l'index a rapporté large.
//!
//! **Pourquoi ici et pas dans la base.** `pg_trgm` sait rappeler vite et bien :
//! son index GIN rend en un balayage d'index tout ce qui partage assez de
//! trigrammes. Ce qu'il ordonne moins bien, c'est le sommet — la similarité
//! trigramme ignore l'**ordre** des caractères et pénalise durement les mots
//! courts.
//!
//! Jaro-Winkler tient les deux : il regarde les correspondances dans une
//! fenêtre, compte les transpositions, et donne une prime au préfixe commun —
//! ce qui colle à la façon dont on cherche un identifiant ou un nom.
//!
//! Et il n'existe pas côté serveur : `fuzzystrmatch` fournit `levenshtein`,
//! `soundex`, `metaphone`, pas jaro. Vérifié sur l'image `pgvector/pgvector:pg17`,
//! zéro fonction. Le rappel reste donc dans la base, où il est indexé, et
//! l'ordre vient ici, sur quelques dizaines de candidats — là où le coût d'un
//! calcul en O(n·m) ne se voit pas.

/// Similarité de Jaro, dans `[0, 1]`.
///
/// Deux chaînes vides sont identiques (1.0) ; une seule vide ne ressemble à
/// rien (0.0).
pub fn jaro(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    // Fenêtre de correspondance : deux caractères ne comptent que s'ils sont
    // assez proches. C'est ce qui empêche « abc » et « cba » de valoir 1.
    let portee = (a.len().max(b.len()) / 2).saturating_sub(1);

    let mut a_vus = vec![false; a.len()];
    let mut b_vus = vec![false; b.len()];
    let mut communs = 0usize;

    for (i, ca) in a.iter().enumerate() {
        let debut = i.saturating_sub(portee);
        let fin = (i + portee + 1).min(b.len());
        for j in debut..fin {
            if !b_vus[j] && b[j] == *ca {
                a_vus[i] = true;
                b_vus[j] = true;
                communs += 1;
                break;
            }
        }
    }

    if communs == 0 {
        return 0.0;
    }

    // Transpositions : les caractères communs pris dans le désordre, comptés
    // par paires (d'où la division par deux).
    let mut transpositions = 0usize;
    let mut j = 0usize;
    for (i, vu) in a_vus.iter().enumerate() {
        if !vu {
            continue;
        }
        while !b_vus[j] {
            j += 1;
        }
        if a[i] != b[j] {
            transpositions += 1;
        }
        j += 1;
    }
    let transpositions = transpositions as f64 / 2.0;

    let c = communs as f64;
    (c / a.len() as f64 + c / b.len() as f64 + (c - transpositions) / c) / 3.0
}

/// Prime de préfixe maximale, en caractères. Quatre est la valeur de Winkler.
const PREFIXE_MAX: usize = 4;

/// Facteur d'échelle de la prime. 0,1 est la valeur de Winkler ; au-delà de
/// 0,25 la similarité pourrait dépasser 1.
const ECHELLE: f64 = 0.1;

/// Jaro-Winkler : Jaro, plus une prime au préfixe commun.
///
/// La prime ne s'applique **qu'au-dessus d'un seuil** (0,7 chez Winkler) : sans
/// ça, deux chaînes qui ne se ressemblent pas mais commencent pareil
/// remonteraient — `configuration` et `confiture` ne sont pas proches.
pub fn jaro_winkler(a: &str, b: &str) -> f64 {
    let j = jaro(a, b);
    if j < 0.7 {
        return j;
    }
    let commun = a
        .chars()
        .zip(b.chars())
        .take(PREFIXE_MAX)
        .take_while(|(x, y)| x == y)
        .count();
    j + commun as f64 * ECHELLE * (1.0 - j)
}

/// Replier les accents, pour que « cafe » et « café » se comparent comme un
/// seul mot.
///
/// Sans ça, l'étage d'ordonnancement contredirait celui du rappel : la base
/// rapporte « café » pour « cafe » — elle normalise, elle — et Jaro-Winkler le
/// classerait ensuite en dessous d'une correspondance moins bonne mais sans
/// accent.
///
/// **Ce n'est pas un normaliseur Unicode.** La table couvre le latin
/// occidental, ce qui suffit au français, à l'espagnol, à l'allemand et aux
/// langues voisines. Un texte cyrillique ou vietnamien passe inchangé — ce qui
/// est correct, faute de mieux, plutôt que faux.
fn sans_accents(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'ç' => 'c',
        'è' | 'é' | 'ê' | 'ë' => 'e',
        'ì' | 'í' | 'î' | 'ï' => 'i',
        'ñ' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
        'ù' | 'ú' | 'û' | 'ü' => 'u',
        'ý' | 'ÿ' => 'y',
        'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => 'A',
        'Ç' => 'C',
        'È' | 'É' | 'Ê' | 'Ë' => 'E',
        'Ì' | 'Í' | 'Î' | 'Ï' => 'I',
        'Ñ' => 'N',
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => 'O',
        'Ù' | 'Ú' | 'Û' | 'Ü' => 'U',
        'Ý' => 'Y',
        autre => autre,
    }
}

/// Minuscules **et** sans accents, en un passage.
fn replie(mot: &str) -> String {
    mot.chars().flat_map(|c| c.to_lowercase()).map(sans_accents).collect()
}

/// Le meilleur Jaro-Winkler entre la requête et **un mot** du texte.
///
/// Comparer une requête de trois mots à un paragraphe entier donne toujours un
/// score bas et sans information : la longueur écrase tout. On compare donc
/// terme à terme, et on garde le meilleur — ce qui répond à la vraie question,
/// « ce texte contient-il quelque chose qui ressemble à ce que je cherche ».
pub fn meilleur_par_mot(requete: &str, texte: &str) -> f64 {
    let mots: Vec<&str> = texte
        .split(|c: char| !c.is_alphanumeric())
        .filter(|m| !m.is_empty())
        .collect();
    if mots.is_empty() {
        return 0.0;
    }
    let termes: Vec<&str> = requete
        .split(|c: char| !c.is_alphanumeric())
        .filter(|m| !m.is_empty())
        .collect();
    if termes.is_empty() {
        return 0.0;
    }

    // Moyenne sur les termes de la requête, chacun cherchant son meilleur mot :
    // une requête dont un seul terme colle ne doit pas valoir autant qu'une
    // dont tous collent.
    let minuscules: Vec<String> = mots.iter().map(|m| replie(m)).collect();
    let somme: f64 = termes
        .iter()
        .map(|t| {
            let t = replie(t);
            minuscules
                .iter()
                .map(|m| jaro_winkler(&t, m))
                .fold(0.0_f64, f64::max)
        })
        .sum();
    somme / termes.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proche(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.001
    }

    #[test]
    fn les_valeurs_de_reference_de_winkler() {
        // Les exemples que Winkler donne lui-même.
        assert!(proche(jaro("MARTHA", "MARHTA"), 0.944), "{}", jaro("MARTHA", "MARHTA"));
        assert!(
            proche(jaro_winkler("MARTHA", "MARHTA"), 0.961),
            "{}",
            jaro_winkler("MARTHA", "MARHTA")
        );
        assert!(proche(jaro("DIXON", "DICKSONX"), 0.767), "{}", jaro("DIXON", "DICKSONX"));
        assert!(
            proche(jaro_winkler("DIXON", "DICKSONX"), 0.813),
            "{}",
            jaro_winkler("DIXON", "DICKSONX")
        );
    }

    #[test]
    fn identique_vaut_un_et_etranger_vaut_zero() {
        assert!(proche(jaro_winkler("rust", "rust"), 1.0));
        assert!(proche(jaro_winkler("", ""), 1.0));
        assert!(proche(jaro_winkler("rust", ""), 0.0));
        assert!(proche(jaro_winkler("abc", "xyz"), 0.0));
    }

    #[test]
    fn la_fenetre_empeche_l_anagramme_de_valoir_un() {
        assert!(jaro("abc", "cba") < 1.0, "{}", jaro("abc", "cba"));
    }

    #[test]
    fn la_prime_de_prefixe_ne_sauve_pas_deux_mots_differents() {
        // Même préfixe de quatre lettres, sens sans rapport : le seuil de 0,7
        // doit empêcher la prime de les rapprocher artificiellement.
        let jw = jaro_winkler("configuration", "confiture");
        let j = jaro("configuration", "confiture");
        assert!(jw < 0.95, "{jw}");
        assert!(jw >= j, "la prime ne peut pas faire baisser le score");
    }

    #[test]
    fn un_mot_du_texte_suffit_a_repondre() {
        let t = "A comprehensive guide to the Rust programming language";
        // Terme exact présent.
        assert!(meilleur_par_mot("Rust", t) > 0.99);
        // Faute de frappe : le trigramme rappellerait, Jaro-Winkler ordonne.
        assert!(meilleur_par_mot("progamming", t) > 0.9, "{}", meilleur_par_mot("progamming", t));
        // Rien à voir.
        assert!(meilleur_par_mot("xylophone", t) < 0.7, "{}", meilleur_par_mot("xylophone", t));
    }

    #[test]
    fn les_accents_ne_separent_pas_deux_fois_le_meme_mot() {
        // La base rapporte « café » pour « cafe » parce qu'elle normalise ;
        // l'ordonnancement doit être d'accord avec elle.
        assert!(meilleur_par_mot("cafe", "un café serré") > 0.99);
        assert!(meilleur_par_mot("café", "un cafe serre") > 0.99);
        assert!(meilleur_par_mot("PRECEDE", "il précède") > 0.99);
    }

    #[test]
    fn tous_les_termes_comptent_pas_seulement_le_meilleur() {
        let t = "the Rust programming language";
        let un_seul = meilleur_par_mot("Rust xylophone", t);
        let les_deux = meilleur_par_mot("Rust programming", t);
        assert!(
            les_deux > un_seul,
            "deux termes qui collent doivent battre un seul : {les_deux} vs {un_seul}"
        );
    }
}
