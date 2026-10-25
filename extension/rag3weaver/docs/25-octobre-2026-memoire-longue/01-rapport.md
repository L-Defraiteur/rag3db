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

## L'après-midi : quatre lots de plus sur master

| hash | lot | ce qu'il change |
|---|---|---|
| 665a4a97c | `cited_by` | Les fiches qui citent une chose (un chemin, un dossier, un symbole, un source), ancrées à elle ou par un apport qui la cite, lues par un Plan déclaré, sans Cypher. Elles se rangent par leur dernier apport (la clé « max » de l'arbre principal, beda3d8f1). Deux changements génériques : le RhaiNode prend une clé de config par champ et déclare ses `inputs` ; un paramètre json au défaut liste attend une liste. |
| 195358024 | le rouge openai-llm | `cited_by` avait rougi master : le témoin du mode strict visait les `signals` d'EmbedNode, qui sont une liste. Il vise maintenant un json sans défaut. La cause du trou : mon tour jouait la lib sans `openai-llm`, et la règle vaut maintenant pour toutes les sessions. |
| d72bcf5e0 | la mémoire dans ses fichiers | Une source à son chemin, un genre à `genres/`, une fiche composée (État, Ancres, Apports). La fermeture du bloc machine : une ligne sans fichier y voyage. Trois défauts de M3 que la mémoire réelle a révélés : les vecteurs dans le bloc, les lignes sans champs du serveur, et une entité à la racine qui relisait les dossiers des autres. |
| bea6c6a98 | la version du format, le manifeste du dossier | `format: N` dans chaque en-tête et chaque bloc, avec des migrations Rhai déclarées. `manifeste.yaml` à la racine des fichiers, engendré depuis le gabarit. Un dossier sans manifeste, d'un autre gabarit ou d'un format inconnu se dit. C'était la demande de Lucie : « faut-il versionner ? ». |

Les rouges de l'après-midi, tous vus avant master sauf le premier :
- **Le rouge openai-llm.** Mon tour jouait la lib sans cette feature, et 80 tests manquaient. Je l'ai corrigé dans le tour.
- **Un faux diff.** Un diff à deux points tiré après un fetch défaisait 276 lignes de master. Je l'ai vu au `--stat` avant qu'il ne serve ; la règle est maintenant le diff à trois points.
- **La bascule de 17 h 43.** Elle a eu lieu pendant un rejeu. La lib s'épingle désormais au départ du tour.
- **Le garde de fraîcheur.** Avec un commit du moteur plus récent que la lib du magasin, `RAG3WEAVER_MOTEUR_ANCIEN` doit rester posé.
- **Deux témoins sous charge.** Le navigateur de l'IDE et le chat ont rougi sous la charge de la lib entière, et sont verts joués seuls. Ils sont ticketés ou corrigés par leurs sessions.

## Ce qui attend

1. **La migration de la fiche de Lucie**, après le rebâti des hôtes de ce soir (il doit porter bea6c6a98) et sa réponse sur l'emplacement du dossier. Elle passe par son hôte :
   - annoncer à c7 ;
   - fermer la base (unregister) ;
   - `instantanes.sh migration-fiche-25-oct` ;
   - mettre le manifeste au gabarit livré ;
   - rouvrir ;
   - écrire la fiche « régime GPU », la Source et l'apport ;
   - vérifier, et dire les deux lignes avant et après.
2. **La forme des fiches de bilan du juge** d'everything declarative : relue, réponses dans sa page §5 (branche juge-1).
3. **En fin de session**, effacer les worktrees et les targets :
   - ici : `wt-*-96` ;
   - à luciepc : `rag3db-p1-96`, `p1r-96`, `p1sans-96`, `seed96`, et les `target-96*`.
