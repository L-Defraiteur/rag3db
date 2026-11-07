// archive_params — l'enregistrement de la Source (L0) d'un document envoyé à l'archiviste (M4).
// input : #{ titre, contenu, chemin, origine, jour, value: le rapport du genre doc (il ne fait qu'ordonner) }
// Rend le record Source : chemin par défaut sources/<jour>-<slug du titre>.md, via archiviste.
// (Rhai : `trim` change la chaîne en place et ne rend rien ; `let` est mutable.)
let titre = input.titre;
titre.trim();
if titre == "" { throw "archive : un titre est requis"; }
let contenu = input.contenu;
let vide = contenu;
vide.trim();
if vide == "" { throw "archive : un contenu est requis"; }
let chemin = input.chemin;
chemin.trim();
if chemin == "" {
    let slug = "";
    let tiret = false;
    let bas = titre.to_lower();
    for c in bas.chars() {
        let lettre = (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9');
        if lettre {
            slug += c; tiret = false;
        } else if !tiret && slug != "" {
            slug += "-"; tiret = true;
        }
    }
    if slug.ends_with("-") { slug = slug.sub_string(0, slug.len() - 1); }
    if slug == "" { slug = "document"; }
    if slug.len() > 60 { slug = slug.sub_string(0, 60); }
    chemin = `sources/${input.jour}-${slug}.md`;
}
#{ chemin: chemin, titre: titre, contenu: contenu, origine: input.origine, via: "archiviste" }
