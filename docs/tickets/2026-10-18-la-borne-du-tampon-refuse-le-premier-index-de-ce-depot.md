# La borne du tampon refuse par défaut le premier index de ce dépôt : 65 Mo de texte demandent 9 Gio

- **État** : ouvert (18 octobre 2026, codeparsers, à la demande de l'orchestration). Contournement
  posé dans l'env de l'hôte de luciepc ; borne non revue.
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
