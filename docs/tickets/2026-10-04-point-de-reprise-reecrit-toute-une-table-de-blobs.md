# Le point de reprise réécrit en entier une table de blobs dès qu'une de ses lignes change

- **État** : ouvert — une optimisation, hors de la stèle (décision de l'orchestration, 4 octobre 2026)
- **Gravité** : lenteur, sans perte
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui — sa table `_index_blobs` (les segments du plein texte rangés en base) ; contourné de son côté en ne poussant les blobs qu'une fois, à la fin du premier index
- **Ouvert le** : 4 octobre 2026, session cœur C++
- **Pour** : cœur C++ (stockage, colonnes de chaînes)

## Ce que c'est

Quand une ligne d'une table qui porte de gros blobs a changé, le point de reprise suivant réécrit toute la table, puis rend ses anciennes pages : son coût suit la taille totale des blobs, pas celle du changement.

## Ce qui a été mesuré

Premier index de ce dépôt sur disque, transaction par paquet, 512 fichiers par paquet, tampon de 8 Gio, moteur `5771f0afb` avec `RAG3DB_PROFILE_CHECKPOINT=1` ; deux passes de la session des embarquements, seules sous le verrou exclusif (`~/.cache/rag3weaver-build/mesure-cp-k1.out` et `mesure-cp-k4.out`, non durables).

| | une validation par paquet | une validation tous les 4 paquets |
|---|---|---|
| durée de la passe | 157 s | 113 s |
| somme des `COMMIT` | 75,0 s | 28,2 s |
| dont la table `_index_blobs` | 48,9 s en 16 points de reprise | 23,8 s en 6 points de reprise |
| dont `Scope`, `Scope_Chunk`, `Symbol`, `File_Chunk` | 12,0 s | 3,6 s |
| dont toutes les relations | ≈ 2 s | ≈ 1,5 s |
| journal et pages d'ombre | ≈ 6 s | ≈ 3 s |
| catalogue, métadonnées, en-tête | < 0,1 s | < 0,1 s |

Un point de reprise de `_index_blobs` coûte 3 à 4,7 s, le même prix que l'on valide un paquet ou quatre. Le fichier de la base grossit puis rétrécit de 100 000 à 270 000 pages (0,4 à 1 Gio) d'un point de reprise à l'autre.

## Recette minimale

Aucune écrite. À faire : une table `(clé STRING PRIMARY KEY, data BLOB)` de quelques centaines de Mio, un `SET` d'un seul blob, `CHECKPOINT` sous `RAG3DB_PROFILE_CHECKPOINT=1` — le temps de la table doit suivre la taille du blob changé, pas celle de la table.

## Témoin

Aucun, assumé : c'est une mesure, pas une justesse.

## Cause

Non lue. Pistes : la mise à jour en place d'une colonne de chaînes est refusée dès que les nouvelles valeurs ne tiennent pas dans les pages du dictionnaire (`DictionaryColumn::canCommitInPlace`), et le groupe entier est alors réécrit hors place ; le dictionnaire est peut-être refait (hachage de chaque blob).

## Ce qu'il faut pour le fermer

Qu'un point de reprise n'écrive que les blobs neufs ou changés. Lire d'abord pourquoi il réécrit tout (une à deux heures) avant d'estimer.

## La poussée unique, côté rag3weaver (arbre principal, 4 octobre, 20 h 30)

Pousser les 699 Mo du plein texte une seule fois à la fin a coûté 52 s, contre 25 s de COMMIT épargnés (113 → 135 s, mesure de rag3db-eb). Hypothèse, non mesurée : `CypherBlobStore::save_many` pousse en ~22 lots de 32 Mo, chacun validé seul, et chaque validation porte un point de reprise qui réécrit toute la table, de plus en plus grosse — un coût quadratique. L'option `RAG3WEAVER_TX_POUSSEE_A_LA_FIN=1` pousse désormais dans une seule transaction (un point de reprise) ; le défaut est revenu à la poussée par paquet.
