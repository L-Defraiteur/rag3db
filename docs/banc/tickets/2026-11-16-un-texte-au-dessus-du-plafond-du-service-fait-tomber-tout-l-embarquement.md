# Un texte au-dessus du plafond du service fait tomber tout l'embarquement

- **État** : ouvert ; constat, pas de correctif.
- **Famille** : produit
- **Gravité** : haute. Un index neuf du dépôt ne reçoit aucun vecteur de scope, et sa dette ne se
  solde jamais.
- **Atteignable en service** : oui, pour tout index ou toute réindexation qui embarque un texte de
  plus de 12 000 caractères par un service `rag3weaver-embeddings` posé depuis le 15 novembre 2026,
  20 h 36, à luciepc (le plafond `RAG3WEAVER_EMBED_MAX_CHARS` par défaut).
- **Touche rag3weaver** : oui. L'embarquement du retard (`catalog.rs`), le découpeur des
  `ScopeContent_Chunk`, et le client du service d'embarquement.
- **Pour** : embarquement, à confirmer par c7.
- **Signalé par** : banc, mesure B′ de la parité de la migration (demande de c7, 16 novembre).
- **Ouvert le** : 16 novembre 2026

## Le constat

Un index neuf de l'arbre 284cc00c0b, dans une base vide, par master bc554c2cee, à luciepc, avec
`RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878` (granite-278m) :

- l'index dure 202 s et finit sur « indexation terminée — 92708 morceau(x) non embarqué(s), à
  reprendre (relancer index) » ;
- le journal compte 13 passes d'embarquement en échec, toutes du même refus :
  « dense embedding failed: provider error: le démon a refusé (413) : texte 0 : 13135 caractères,
  plafond 12000 (RAG3WEAVER_EMBED_MAX_CHARS, environ 3 caractères par jeton) : refusé sans
  calcul » ;
- au bout du compte, **aucun des 92 708 `ScopeContent_Chunk` n'a de vecteur**. Les 6 117
  `File_Chunk` en ont un ;
- une passe de `Catalog::embarquer_le_retard_hors_verrou` lancée après l'index rend `Ok(0)` en 2 s.
  L'échec ne remonte pas à l'appelant.

## Les textes en cause

31 `ScopeContent_Chunk` dépassent 12 000 caractères, jusqu'à 53 294. Ils viennent tous de fichiers
de données de test faits d'une ou deux lignes énormes :

| titre du morceau | plus long texte |
|---|---|
| `copy_edge_case.test:1-21` | 53 294 |
| `demo.test:1-107` | 42 784 |
| `duplicated.test:1-111` | 24 082 |
| `large_list_of_struct_answer.txt:1-1` | 22 911 |
| `q24.benchmark:1-15` | 18 600 |

Le découpeur coupe par lignes. Une ligne de 50 000 caractères reste donc un seul morceau.

## Pourquoi tout tombe

- Le service refuse un lot entier dès qu'un seul de ses textes passe le plafond.
- Le client ne lit pas ce plafond : sur master, `max_chars_per_text` n'apparaît que dans le serveur
  (`src/daemon/embeddings.rs`, `src/bin/rag3weaver-embeddings.rs`).
- Le lot refusé revient à la passe suivante, avec le même texte. La dette ne baisse jamais, et les
  autres morceaux de la table attendent derrière lui.

## Ce que ça touche

- **La copie migrée de la bascule** porte les mêmes 31 morceaux, tous embarqués (113 570 sur
  113 570). Ses vecteurs datent d'avant le plafond, ou ont été recopiés. Un repassage qui les
  réembarque prendra le refus.
- **L'hôte de Lucie**, s'il réembarque l'un de ces fichiers depuis le 15 novembre au soir.

## Les correctifs possibles

1. le client borne chaque texte au plafond du service, lu au service ou déclaré ;
2. le découpeur borne un morceau en caractères, et non seulement en lignes ;
3. un 413 sur un texte isole ce texte, qui reste dû et dit, au lieu de jeter tout le lot.

Les trois ne s'excluent pas. Le troisième protège de tout texte qui passera un jour le plafond.

## Pour rejouer

À luciepc, `~/.cache/rag3weaver-build/b-prime.sh` (sous `poste lourd`). Les sorties sont dans
`~/.cache/rag3weaver-build/banc-parite/sorties/b-prime/` : `bprime.log`, et `gros-3-*.json` pour
les tailles des morceaux de B′ et de la copie migrée.
