// archive_query — la requête de la recherche des candidates d'une section : son titre et le début de son
// texte, coupés à query_chars caractères sur une frontière de mot (15 novembre 2026 : la section entière,
// embarquée comme requête, faisait 8 000 jetons et tuait le service). Rend #{query, query_chars}.
let s = input.section;
let borne = input.query_chars;
if borne <= 0 { throw "archive : query_chars doit être positif (reçu " + borne + ")"; }
let titre = if s.title == () { "" } else { s.title };
let texte = if s.text == () { "" } else { s.text };
let q = if titre == "" { texte } else { titre + "\n" + texte };
if q.len() > borne {
    let coupe = q.sub_string(0, borne);
    let fin = coupe.len() - 1;
    let blanc = -1;
    while fin > borne / 2 {
        let c = coupe.sub_string(fin, 1);
        if c == " " || c == "\n" || c == "\t" { blanc = fin; break; }
        fin -= 1;
    }
    q = if blanc > 0 { coupe.sub_string(0, blanc) } else { coupe };
}
#{ query: q, query_chars: q.len() }
