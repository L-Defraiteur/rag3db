# La borne du tampon refuse par défaut le premier index de ce dépôt : 65 Mo de texte demandent 9 Gio

- **État** : **fermé** le 18 octobre 2026 (arbre principal) : la borne est recalculée sur la
  mesure, 28 Mo par Gio au lieu de 8 (§ « Fermé »). Ouvert le même jour par codeparsers.
- **Gravité** : blocage. `index` refuse avant d'écrire, rien n'est perdu ni faux, mais le dépôt
  ne s'indexe pas sans réglage.
- **Atteignable en service** : oui, sur tout poste au défaut du produit, dès que le texte retenu
  dépasse 64 Mo. Le dépôt rag3db lui-même a passé ce seuil.
- **Touche rag3weaver** : oui (`index`, `estimate`).

## Ce que c'est

Le refus d'avant l'écriture (`Estimate::buffer_refusal`, `extension/rag3weaver/src/estimate.rs`)
compte 1 Gio de tampon par 8 Mo de texte retenu (`TEXT_BYTES_PER_BUFFER_GIB = 8_000_000`). La
règle du produit donne au plus 8 Gio de tampon (la moitié de la mémoire, `connection.rs`,
`BufferPoolSource::Rule`). Tout dépôt de plus de 64 Mo de texte est donc refusé par défaut, sur
tous les postes, même avec 93 Go de mémoire vive.

Le 18 octobre à 15 h 00, sur luciepc, l'index de `code-principal` (master `4c3e8179a`) a été
refusé ainsi :

> tampon du moteur trop petit pour indexer ce dépôt — rien n'a été écrit. 65.2 Mo de texte
> demandent environ 9.0 Gio de tampon ; ce processus en a 8.0 Gio (8.0 Gio, règle du produit […]

## Les chiffres

| date, poste | base | texte | tampon posé | durée | pic de l'hôte |
|---|---|---|---|---|---|
| 12 oct., poste principal | `code-rag3db` (autonome) | 64 Mo | 10 Gio | 1 235 s | non relevé |
| 18 oct., luciepc | `code-principal` @ `4c3e8179a` | 65,2 Mo | 10 Gio | 260 s | VmHWM 8,2 Go (tout le processus) |

Le 18, avec 10 Gio posés, la passe a fini (141 471 morceaux, base de 1,3 Go et plein texte de
841 Mo) sous un pic de 8,2 Go pour le processus entier : tampon, embarquement, lucivy et pile
compris. Rien ne montre que 9 Gio de tampon étaient nécessaires.

## La cause

La borne est **provisoire, d'après deux mesures du 4 octobre** (le commentaire de la constante le
dit) : 62 Mo passaient sous 8 Gio, et sous 4 Gio la passe mourait vers 40 Mo sur un point de
reprise qui ne tenait plus dans le tampon. Ce défaut moteur est corrigé depuis le 11 octobre en
`47a283e6c` pour les écritures ligne à ligne
([ticket fermé](closed/2026-10-04-point-de-reprise-echoue-quand-le-tampon-est-petit.md)). Il reste
ouvert pour le COPY de blobs
([ticket](2026-10-11-copy-de-blobs-en-point-de-reprise-force-sous-un-petit-tampon.md), dont le
filet produit borne déjà les COPY en octets). La borne du produit n'a pas été revue depuis : elle
mesure un défaut corrigé.

C'est donc plutôt une estimation dépassée qu'une vraie démesure. Mais la confirmation sous un
petit tampon n'est pas jouée
([ticket ouvert](2026-10-11-confirmer-le-premier-index-sous-un-petit-tampon.md)).

## Contournement posé

Sur luciepc, `RAG3DB_BUFFER_POOL_SIZE=10737418240` (10 Gio) dans
`~/.config/rag3weaver/rag3weaver-daemon.env` (sauvegarde `.avant-tampon-18-oct`). Sur le poste
principal, la même valeur se pose à la main par passe (script de réindexation de codeparsers, le
12 octobre).

## Lien avec la mémoire de l'hôte

Monter le tampon pour passer la borne agrandit ce que l'hôte peut garder après l'index : voir
[l'hôte ne rend pas la mémoire d'un gros index](2026-10-18-l-hote-ne-rend-pas-la-memoire-d-un-gros-index.md).
Une borne trop haute pousse à poser de gros tampons dans des hôtes qui vivent des jours.

## Recette

Sans Cypher : `index {"confirm": true}` sur un dépôt de plus de 64 Mo de texte retenu, hôte sans
`RAG3DB_BUFFER_POOL_SIZE` (règle du produit à 8 Gio). Le refus tombe en moins de 3 s.

## Témoin

Aucun pour la borne elle-même. Le témoin utile est la confirmation : le premier index de ce dépôt
sous un tampon étroit (2 puis 4 Gio), sans la borne, sur la lib de master. Il dira la pente
réelle, s'il en reste une.

## Pour le fermer

1. Jouer la confirmation sous 2 et 4 Gio sans la borne (le ticket ouvert du 11 octobre), avec le
   pic de mémoire du processus.
2. Selon le résultat : retirer la borne, ou la recalculer sur ces mesures, en gardant le refus
   nommé pour le seul cas qui casse encore (le COPY de blobs, tant que son ticket est ouvert).
3. Retirer alors `RAG3DB_BUFFER_POOL_SIZE` de l'env de luciepc, ou dire pourquoi il reste.

## Fermé (18 octobre 2026, arbre principal)

**Ce que la borne mesurait** : la première indexation morte sur un point de reprise sous un petit
tampon. C'est le seul objet de la borne, et il ne se montre plus, ni en fichiers ni en blobs, même
sous 2 Gio. Le 4 octobre, la passe mourait sous 4 Gio vers 40 Mo ; le 11 octobre, elle passait sous
2 Gio en fichiers et mourait en blobs.

Luciepc, master `c6bb094fb` (lib du magasin 0fc7dd10f), worktree `rag3db-tampon`, la recette du
ticket du 11 octobre (`RAG3WEAVER_ESTIMATE_REPO=1`, `e2e_estimate
ce_depot_est_cherchable_par_mots_avant_ses_vecteurs`, base sur disque), qui passe à côté de la
borne, sous `poste lourd`, services d'embarquement locaux, régime doux. 6 318 fichiers, 57 Mo de
texte compté par la passe, 687 467 liens. Journaux : `~/.cache/rag3weaver-notes-A/tampon/` sur
luciepc. Le pic est relevé par le test toutes les 100 ms ; il tombe chaque fois pendant la pose
des relations.

| passe | plein texte | tampon | résultat | durée | pic au fil de l'eau | anonyme à 1 024 / 3 072 / 5 120 / 6 318 fichiers |
|---|---|---|---|---|---|---|
| paquets de 512 | fichiers | 2 Gio | **vert** | 69 s | 7 907 Mo | 2 359 / 4 576 / 5 569 / 6 288 Mo |
| paquets de 512 | fichiers | 4 Gio | **vert** | 68 s | 7 677 Mo | 2 365 / 4 493 / 5 265 / 5 967 Mo |
| paquets de 512 | blobs | 2 Gio | **vert** | 124 s | 9 212 Mo | 2 635 / 5 253 / 6 801 / 7 580 Mo |
| paquets de 512 | blobs | 4 Gio | **vert** | 122 s | 9 805 Mo | 2 673 / 5 609 / 7 269 / 8 405 Mo |
| paquets de 2 048, un COMMIT par paquet (le défaut d'`index`) | fichiers | 4 Gio | **vert** | 58 s | 7 982 Mo | 3 769 à 2 048 ; 6 446 à 6 144 |
| paquets de 2 048, une seule transaction (`RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION=tous`) | fichiers | 4 Gio | **vert** | 56 s | 8 242 Mo | 3 699 à 2 048 ; 6 599 à 6 144 |

**Ce qui change** (`extension/rag3weaver/src/estimate.rs`, `TEXT_BYTES_PER_BUFFER_GIB`) : 8 Mo → 28
Mo de texte par Gio, au plus petit point mesuré qui a passé (57 Mo sous 2 Gio). La règle du produit
(8 Gio) porte alors 224 Mo, au lieu de 64. La borne reste, faute de mesure sous 2 Gio, et parce
qu'un gros COPY de blobs peut encore échouer sous un petit tampon : elle devient le **filet** du
[ticket du COPY de blobs](../2026-10-11-copy-de-blobs-en-point-de-reprise-force-sous-un-petit-tampon.md), qu'elle cite et qui la liste.

**Le contournement de luciepc** (`RAG3DB_BUFFER_POOL_SIZE=10737418240` dans
`~/.config/rag3weaver/rag3weaver-daemon.env`) n'est plus nécessaire pour ce dépôt. Il est à
codeparsers et touche son hôte : je ne l'ai pas retiré, je le lui ai dit.

**Ce que disent aussi ces passes** (pour le
[ticket de la mémoire de l'hôte](../2026-10-18-l-hote-ne-rend-pas-la-memoire-d-un-gros-index.md)) :
- le tampon ne fait presque rien au pic : 7,9 Go sous 2 Gio, 7,7 Go sous 4 Gio ;
- la transaction unique non plus : 8,2 Go contre 8,0 Go par paquets ;
- c'est l'anonyme qui fait le pic, environ 1 Go par millier de fichiers, et le pic tombe pendant la
  pose des relations.
