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


## Relecture Banc de la préparation de paire 2b3d66/fec351

[Rapport spécialisé14](../../banc/16-11-2026/14-ro-paire-sonde-rust.md), trois réserves groupées avant GO : nouvel ELF/ledger/copie avant test et rehash final ; composition1800 bornée incluant finale/publication ; attribution cache et fresh-owned final après Common. Couple/source/corpus/cohortes et comparaison multiensemble cohérents, aucun run par Banc. Porteur Codeparsers, critères : nouveau pilote/pins publiés puis RO delta, aucune modification produit nécessaire pour ces réserves de preuve. Ticket et REGLES7 restent ouverts/inchangés.


RO delta c1cd/pair02f919 par Banc : les trois réserves052d sont levées statiquement (ELF/2ACK avant test, composition1800 finale incluse, fresh attribution/owned/bus). Voir complément du rapport14. Avis préparatoire favorable, aucun GO ; essai CPU1916=refusCLI rc2/zéro test, correctifc1cd non rejoué, CPU/natif toujours non qualifiés. Paire/cohortes identiques, aucun REGLES8 ni fermeture du ticket.
