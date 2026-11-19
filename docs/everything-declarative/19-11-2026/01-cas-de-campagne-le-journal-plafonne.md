# Cas de campagne : le journal plafonné d'une commande en fond perd sa fin

```json
{"parent": "b610560e519dbd0fb176e231a9dad677cd63ac65", "tree": ".", "needs_lib": false, "witness": "commande::tests::en_fond_un_journal_au_dela_du_plafond_garde_la_fin_et_le_dit", "test": "cd extension/rag3weaver && cargo test --lib -- commande::tests::en_fond_un_journal_au_dela_du_plafond_garde_la_fin_et_le_dit", "suites": ["cd extension/rag3weaver && cargo test --lib -- commande::"], "hints": ["extension/rag3weaver/src/commande.rs"], "ours": "1bffb7659d785cb2d09bfdf91130c89cde512aa0"}
```

Session everything declarative, 19 novembre 2026, pour la campagne des correctifs passés (chantier de Lucie ; lanceur
d'archi, juge du banc). C'est un cas court, en Rust, dans le crate rag3weaver, sans aucune feature : ni la lib rag3db,
ni modèle.

## Le point de départ

- **Commit parent** : `b610560e51` (19 novembre 2026, 14 h 16). Le témoin y est, rouge ; le correctif n'y est pas.
- **Arbre** : `.`, le dépôt entier. Le crate `extension/rag3weaver` déclare des dépendances optionnelles par chemin hors
  de son dossier (`tools/rust_api`, le sous-module `codeparsers`), et cargo lit leur manifeste même sans la feature.
  Le sous-module `extension/rag3weaver/codeparsers` doit donc être là (`git submodule update --init`).

## Le signalement, tel qu'il était

> Dans la lib entière de rag3weaver, sous charge, le témoin
> `commande::tests::en_fond_un_journal_au_dela_du_plafond_garde_la_fin_et_le_dit` rougissait une fois sur deux. Il
> rend maintenant la charge déterministe (le retard décrit plus bas) et **rougit à chaque exécution**, seul comme dans
> la lib. Une commande lancée en fond écrit elle-même son journal ; un surveillant le plafonne (100 000 octets dans le
> témoin) : au-delà, il garde la fin sous un en-tête qui compte les octets perdus. La commande écrit 30 000 lignes de
> 11 octets, 330 000 octets. Sous charge, le compte des octets écrits que rend la fin (`octets_stdout`) tombe loin
> sous ce qui a été écrit : la troncature perd des octets sans les compter. Le témoin rend la charge déterministe par
> un retard de 50 ms dans le surveillant, entre la lecture de la fin du journal et sa troncature
> (`PLAFONNER_RETARD_MS`) ; il compte alors 260 051 octets pour 330 000 écrits. Ce qu'on attend : le compte ne
> dépasse jamais l'écrit et n'en manque que quelques octets, même sur une machine chargée ; le journal reste sous son
> plafond, dit sa perte en tête et garde la fin de la sortie (`ligne29999`).

## Le témoin

- **Filtre** : `commande::tests::en_fond_un_journal_au_dela_du_plafond_garde_la_fin_et_le_dit`, dans
  `extension/rag3weaver/src/commande.rs`.
- **Suite** : `cargo test --lib` du crate `extension/rag3weaver`, sans feature ; le module `commande::` entier
  (52 tests) comme suite exigée. Aucun chemin du correctif n'est dans `tools/poste/required_suites.txt`.
- **Rouge au parent** : `assertion failed: fin.octets_stdout <= 330_000 && fin.octets_stdout > 325_000`, avec
  `260051`.
- **Sur master** : le témoin y sera identique quand le lot `ed-capped-journal` aura atterri. Il est en file au pousseur
  le 19 novembre à 14 h 30. Avant cela, master a l'ancien témoin, sans le retard et avec une borne lâche
  (plus de 300 000).

## Ce qu'une résolution doit tenir

Ne pas desserrer la borne ni retirer le retard. Le correctif est dans `plafonner`, pas dans le témoin. Les 52 tests
de `commande::` restent verts.

## Notre correctif (caché à Algareth, pour le juge)

`1bffb7659d`, « fix(rag3weaver): a capped background journal keeps its end and counts what it loses » :
`extension/rag3weaver/src/commande.rs`, +10 −2. Juste avant la troncature, `plafonner` relit ce qui s'est écrit
depuis sa lecture de la fin, ne garde que la seconde moitié du plafond et compte le reste comme perdu. Une commande qui
finissait pendant la fenêtre perdait sa fin ; elle la garde.
