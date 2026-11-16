# Un chemin de la bibliothèque standard se relie à un homonyme du projet

- **État** : ouvert (16 novembre 2026) — trouvé par la sonde du lot des mots vides de Rust, avant son « prêt »
- **Famille** : produit
- **Gravité** : réponse fausse (une arête « nom » vers une fonction homonyme du projet)
- **Atteignable en service** : oui, dès que le lot des mots vides de Rust atterrit (branche rag3db
  `codeparsers/rust-stop-words`) ; masqué jusque-là parce que `var` était un mot vide (la liste de JavaScript)
- **Touche rag3weaver** : oui (`src/code.rs`, la collecte du rendez-vous)
- **Pour** : la session qui reprend codeparsers (Codex)
- **Signalé par** : session codeparsers

## Ce que c'est

`std::env::var("X")` : la référence `var`, qualifiée par le chemin `std::env`, prend rendez-vous ; le seul
définisseur `var` du projet (`src/ide/clic_tsx.rs`) conclut par le nom. Sur le corpus figé de rag3db-garde,
**107** arêtes « nom » de ce genre naissent avec le lot des mots vides de Rust (`var` n'y est plus un mot vide),
plus 9 `var` nus. La règle de la marque « nom » (méthodes std) ne couvre que les receveurs locaux, et celle des
bibliothèques sûres (`via_qualifier`) que les qualificatifs importés : un chemin écrit en entier
(`std::env::var`), sans `use`, n'entre dans aucune.

## Ce qu'il faut

Dans `materialiser`/la collecte de `code.rs` (là où `externes_sures` écarte déjà le membre d'une bibliothèque
de provenance sûre) : une référence dont le qualificatif est un chemin (`chemin` vrai) dont **le premier
segment** est une bibliothèque de provenance `builtin` ou `dependency` (`std`, `core`, `alloc`, une crate
déclarée) ne prend pas de rendez-vous. Le premier segment se lit sur le qualificatif (`std::env` → `std`).

## Pour le fermer

Un témoin rouge dans `tests/e2e_usages.rs` (`std::env::var("X")` dans un fichier, un `pub fn var()` dans un
autre : aucune arête), la sonde avant/après (`tests/sonde_marque_nom.rs`) : les 107 partent, rien de juste ne
part.

## Reprise préparée, sans calcul pendant la bascule du 16 novembre

WIP conservé depuis 5ef2935d29/b48da05 dans un worktree neuf
`/home/lucied/git_workspaces/cx-codeparsers-rust-stop-words`, branches
rag3db `cx-codeparsers-rust-stop-words` et codeparsers `cx-rust-stop-words`.
Le contexte de ligne est partagé et trimé (`line_at_byte`), tandis que la colonne
AST est un offset UTF-8 depuis le début de ligne non trimé : ne pas appliquer
cette colonne directement au contexte. Le filtrage doit porter sur la mention
exacte, pas toutes les occurrences homonymes de sa ligne.

Deux témoins supplémentaires publiés avant correctif : appel std puis appel
projet nu sur la même ligne, puis ordre inverse avec préfixe UTF-8. **NON JOUÉS**
(pas de Cargo/rustc/test pendant la fenêtre de bascule). Le rouge est à observer
sur la forme 5ef avant de remplacer ce filtrage ; aucun vert n'est revendiqué.
Les rouges historiques et les 14 pertes de la première sonde restent conservés.

### Préparation du pilote après ED : cible reçue absente sur PC

Nouvelle base ED d395cb1f03, source pilote504898a5c8 publiée avant préparation.
La préparation PC a laissé un dossier vide puis s'est arrêtée avant manifest
et avant Cargo : son hypothèse d'un target-import-pointe-pile existant à PC
était fausse. Relevé RO : source504898 propre, sous-module présent, targetabsent,
proofroot rust-red-504898a5c8 vide. Sortie originale non récupérée dans le retour
du lanceur interactif ; ne pas inventer son rc ou sa traceback. Ce n'est ni
un rouge Rust ni une allocation1800 consommée. Dossier conservé, aucun rejeu.

Correctif du seul profil : target-cx-codeparsers-rust-red PC initialement absent,
parent réel/possédé, PQS après poste et création exclusive dans la portée.
Ancien target d'un autre poste inchangé ; pas de copie SDK/cache. Nouveau
proofroot et capture cmd/rc/stdout/stderr externes avant toute assertion.
Le correctif fonctionnel de la collision reste absent, témoins NON JOUÉS.

## Rouge exact avant correctif de la mention, 16 novembre

Source056f552c122353a25c37f2feaf1d4e5006c712f3, pilote SHA-256
b3a7f1f0acb31ed8e01645e92ee8ee71eceeea527e24706a49989eb649a39f5d,
allocation C7-ROUGE-056f-16nov : les deux ordres d'appels ont échoué pour la
bonne raison, 0passé/2échoués/0ignoré en0,89s, assertion left0/right1
« the project call must remain visible ». Deux préalables natifs avaient passé.
HashEmbedder64, aucune requête PG/service/modèle, lib59ac vérifiée.

Reçu validation-kjnv52n2 terminé : Commonrc1/ROUGE, enfant101, erreur=null,
restes=[], nettoye=true, privé /tmp/r3v-frf5yk7u retiré. Aucun vert/global acquis.
Les bruts, manifestes et classification sont conservés sur les deux postes sous
~/.cache/codeparsers-notes/rust-red-056f552c12/proofs/. Le reçu contenant le
jeton reste dans sa preuve protégée d'origine à luciepc ; le fichier de chemin
d'autorité et le verdict peuvent être relus sans recopier ce jeton.
red-pair.stdout SHA-256 ecabf0b3b22075aef0600d115cde8c57c9159e6aee1f44cfa991f4504d21ab49,
prepared.json 1fbef7c4caf9243b4c12b96a04b1834d5567626808f4348a97e9ee0b430d24ae,
classification.json fad8edcc7dd66875922e749a50faf2be8a9cacf819de32d0340aaaa022a3d6cb.

Le worktree rouge demeure figé. Correction préparée séparément dans
/home/lucied/git_workspaces/cx-codeparsers-rust-fix, branche
cx-codeparsers-rust-fix-1 ; aucun budget rouge réactivé. La ligne brute sera
indexée une seule fois par fichier ; la position de chaque référence sera
vérifiée en octets avant de qualifier le chemin de cette occurrence seulement.
REGLES reste7 jusqu'à preuve verte et qualification du gitlink.
