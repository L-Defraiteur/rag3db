// archive_params — l'enregistrement de la Source (L0) d'un document envoyé à l'archiviste (M4).
// input : #{ titre, contenu, chemin, origine, value: le rapport du genre doc (ignoré : il ordonne) }
// Rend le record Source : chemin par défaut sources/<jour>-<slug du titre>.md, via archiviste.
let titre = input.titre.trim();
if titre == "" { throw "archive : un titre est requis"; }
let contenu = input.contenu;
if contenu.trim() == "" { throw "archive : un contenu est requis"; }
let chemin = input.chemin.trim();
if chemin == "" {
    let mut slug = "";
    let mut tiret = false;
    for c in titre.to_lower().chars() {
        if c.is_alphanumeric() && c.to_string().len() == 1 {
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
