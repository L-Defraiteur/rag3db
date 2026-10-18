# Une Ref ne vieillit jamais : son cycle de vie est déclaré, mais rien ne le déclenche

- **État** : ouvert (relevé le 18 octobre 2026 par la session mémoire longue, en étudiant les
  fiches qui visent du code ; le correctif — le vieillissement par l'index de code — attend le
  mot de Lucie, avec la graine du pont entre bases)
- **Gravité** : réponse fausse en silence. Une fiche qui cite un fichier ou une ligne du
  catalogue garde sa Ref à l'état `vue` quand la cible change, disparaît ou est retirée :
  rien ne dit qu'elle est à revoir, ce qui contredit « jamais fausse en silence ». Ni perte ni
  blocage.
- **Atteignable en service** : oui, dès qu'une fiche porte une Ref et que sa cible bouge
  (aujourd'hui, la mémoire de Lucie a une seule Ref ; l'explorateur de code en fera l'usage
  ordinaire).
- **Touche rag3weaver** : oui (et seulement) : le gabarit `memory` et le harnais des genres.
- **Ouvert le** : 18 octobre 2026, session mémoire longue, à la demande de l'orchestration.
- **Pour** : mémoire longue et proto, après le mot de Lucie sur le pont.

## Ce qu'on voit (lecture du code, master 17350184e)

- Le cycle est déclaré : `Ref.etat` part de `vue`, avec `revoir` (vue → a_revoir),
  `confirmer`, `perdre` / `perdre_revue` (→ introuvable), `retrouver`, `retirer`
  (`extension/rag3weaver/templates/backends/memory/backend.json:185-224`).
- **Aucune réaction ne regarde les Refs** : la seule réaction du gabarit est
  `review_anchored`, sur `Memory` par `ANCHORED_TO` (`:404-415`). Rien ne compare une
  empreinte, rien ne pose `a_revoir` ni `introuvable`.
- **L'empreinte n'est calculée qu'à la pose**, par le script du genre, qui ne lit rien :
  l'hôte ne lui prête que `json_string` et `content_hash`, sans entrée-sortie
  (`src/dataflow/ref_nodes.rs:99-155`, et `:115-119` : « `resolve` n'est pas faisable du
  tout aujourd'hui »). Une empreinte posée n'est donc jamais recomparée.
- `schemas/ref_type.json:9` décrit encore trois fonctions (`validate`, `resolve`,
  `fingerprint`) qui n'existent pas : le harnais est une seule expression.

## Ce qu'il faudrait

Le détail est dans `extension/rag3weaver/docs/18-octobre-2026-memoire-longue/01-une-fiche-qui-vise-un-symbole.md`, §3-§4 :

1. Un service de lecture d'un backend nommé, prêté par l'hôte au harnais
   (`code_fingerprint(valeur)` pour les genres `path` et `symbol`), par la vue en lecture de
   l'autre base, jamais son mutex.
2. Le vieillissement **à la lecture** d'abord : `cited_by` et `add_ref` recomparent
   l'empreinte des Refs qu'ils rendent ou posent et appliquent `revoir` / `perdre` ; une base
   cible occupée ou absente rend « non vérifié », jamais un faux `a_revoir`.
3. Plus tard, une réaction au fil de la montre du code.
4. D'ici là, **le dire** : `cited_by` rend l'état des Refs tel quel ; sa description et sa
   réponse doivent dire que cet état n'est pas vérifié contre la cible.

## Témoin rouge à écrire

Une fiche cite `path:<fichier>` ; le fichier change ; la Ref doit passer `a_revoir` à la
lecture suivante. Aujourd'hui elle reste `vue`.
