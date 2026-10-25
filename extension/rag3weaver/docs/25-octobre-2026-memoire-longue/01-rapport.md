# Rapport de la session mémoire longue — 25 octobre 2026

Session rag3db-96, dite « mémoire ». Le fil d'avant est dans
`../19-octobre-2026-memoire-longue/` ; le modèle des fiches en fichiers est à
`../18-octobre-2026-memoire-longue/04-les-fiches-dans-des-fichiers.md`.

## Ce qui a atterri sur master

Six lots, chacun avec un rejeu vert sur le master du moment, puis poussé par un tour sous
`poste push` dans un `if VERT`, et contrôlé trois fois (status propre, aucun rebase en
cours, `origin/master` = le hash après fetch).

| hash | lot | ce qu'il change |
|---|---|---|
| b6270fc19 | la fuite des réglages de test | Le seuil du COPY des liens, la taille des paquets et le crochet d'arrêt passent sur le catalogue du témoin (`set_link_copy_threshold`, `set_file_batch_size`, `stop_after_batch`). Un test ne règle plus le produit pour tout le processus, et `RAG3WEAVER_TEST_STOP_AFTER_BATCH` n'est plus lue. |
| 2c44d9f38 | la reprise | Une reprise de drain garde les liens vers les lignes nées dans le même drain : leurs bouts se retrouvent par les clés des lignes du point de reprise. Une clé inconnue rend une référence échouée, pas un lien deviné. |
| 02454ac98 | l'annulation des liens | L'annulation d'un lot de liens ne retire que les paires qu'il a créées. Une déliaison annulée se repose avec ses propriétés. |
| c3fdcb979 | les écritures silencieuses | L'annulation d'une fin qui transitionne s'annonce, et un drain, une pose, une ingestion ou une reprise publient ce qu'ils ont écrit même quand ils tombent ensuite. Un ticket neuf : « un Plan qui écrit ne s'annonce pas ». |
| 388f2260b | le retrait du filet | `copier_les_liens` nomme toutes les propriétés déclarées d'une relation, avec le mot NULL pour celles que le groupe ne porte pas. Le plancher dcbaef7b1 est dans `scripts/planchers-de-la-lib`, et `rag3db_libs.py verifier` rend « planchers tenus (3) » sur `courant`. |
| 80ea996c2 | M3, la reconstruction | La base se reconstruit depuis ses fichiers et la parité se prouve champ à champ. Un fichier composé porte un bloc machine. L'ouverture compare le dossier à la base, et `verify_files` reconstruit à blanc (page 04, §5 ter). |

## M3 en bref

- **La restauration** : `EntityRecordNode` en action `restore`. Elle n'est permise que sous
  le service de restauration, que seul `rebuild_from_files` enregistre.
- **Le refus** : une base qui n'est ni vide ni perdue est refusée, et le refus nomme
  l'entité et son compte.
- **Le bloc machine** : du YAML dans un commentaire HTML, produit depuis le schéma pour
  toute entité à fichier composé. Un lien vers une entité sans fichier se compte
  (`links.to_absent_rows`) ; il n'est jamais deviné.
- **À l'ouverture** : `index_state.files`, et l'avertissement « dossier ≠ base : N
  écart(s) ».
- **En ligne de commande** : `rag3weaver-backend files verify|rebuild <backend.json>`.
- **Les témoins** : `e2e_reconstruction`, 5 tests.

## Les rouges du jour, tous trouvés avant master

- **Le témoin du manifeste de M3b ne chargeait pas** (« Ref : aucun champ de contenu »).
  Corrigé en déclarant un champ de contenu.
- **La lib de M3c hors tests ne compilait pas.** `tempfile` n'est qu'une dépendance de
  développement. `check_features.sh`, c'est-à-dire `cargo check --tests`, ne le voit pas ; le
  bâti des e2e, si. C'est corrigé par un répertoire jetable en std, et la leçon est consignée.
- **`verifier` rendait 0 hors du dépôt**, avec « git ne sait pas ». Je l'ai remonté au proto,
  qui l'a corrigé : codes 0, 2 et 3, et une ligne de verdict (82c4f7ff9). Ma seconde remarque
  porte sur `courant` nu et sur un identifiant court ; elle est sur `planchers-verdict-2`.
  Mon tour exige désormais la ligne « planchers tenus ».
- **Les rebases ont conflictué**, mécaniquement : l'index des tickets, et les littéraux de
  `Backend` après K1. À chaque fois, un nom de branche neuf, sans force.

## Ce qui attend

1. **`cited_by` par PlanNode**, sur le master d'outline (le PlanNode y prend un paramètre
   par clé). La page 04 §7 vise la Fiche (ANCRE vers les Ref, A_POUR_APPORT vers les L1) et
   non plus Memory/CITE comme la fixture de l'IR. Le tri « par la date du dernier L1 »
   demande un ordre sur un agrégat de lignes liées, que l'IR n'a pas : soit une clé d'ordre
   de plus, soit le tri sur la date de la fiche. Ce choix se dira avec le lot.
2. **Le gabarit mémoire câblé sur ses fichiers**, et la migration de la fiche de Lucie.
3. **En fin de session**, effacer les worktrees et les targets :
   - ici : `wt-*-96` ;
   - à luciepc : `rag3db-p1-96`, `p1r-96`, `p1sans-96`, `seed96`, et les `target-96*`.
