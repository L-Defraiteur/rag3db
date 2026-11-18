# Évaluation du travail de Codex dans le périmètre de la mémoire

18 novembre 2026, session mémoire (96), à la reprise demandée par Lucie. Codex a tenu le rôle du
16 novembre, 11 h, au 17 novembre, 21 h 08.

Ce qui est relu :
- les 76 commits de master depuis `927941a67a` ;
- les branches `cx-memoire-*` de `origin` ;
- sa passation : `origin/cx-memoire-passation-claude-17nov-r1`,
  `docs/memoire/17-11-2026/99-passation-pour-claude.md`.

## 1. Le tableau

Dans le périmètre de la mémoire, Codex n'a pris qu'**une** des quatorze entrées de ma page du
16 novembre : la 2.1, le témoin de `codex_threads` qui rougit sur « Text file busy ». Le reste de
son temps, dans ce rôle, est fait de relectures sans exécution pour les sessions archi, codeparsers
et tty, et d'un nettoyage.

| Objet | Ce que c'est | Verdict |
| --- | --- | --- |
| `7b3a88e13b` (code) | `src/codex_threads.rs` : `write_executable` fait écrire les scripts de test par un enfant `/bin/sh`. Le parent ne tient aucun descripteur d'écriture qu'un fork parallèle hériterait. Utilisé aux deux endroits, `livrer` inchangé. Test neuf en anglais : `executable_fixture_preserves_literal_path_and_script_arguments` | **utile, à garder**. Le correctif est juste, et son commentaire a raison : un `rename` (ma proposition du ticket) ne suffisait pas. |
| `50fea68511`, `1ef583ac02`, `docs/memoire/16-11-2026/script_fixture_probe.py` | le témoin système du descripteur hérité : rouge `bc07b0cb85` (errno 26 déterministe), vert `40e7f2c37c` | **utile, à garder**. Le rouge d'abord existe et il est déterministe. Il prouve le mécanisme, pas la seule cause, et la page le dit honnêtement. Les hashes existent. Les journaux `rouge-bc07.log` et `vert-40e7.log` sont dans `~/.cache/rag3weaver-build/journaux-cx-memoire-script-fixtures-16nov/`. |
| `3195719a41`, `bb2eb418d4`, `cacbb32a5f`, `3baf78696f` (docs) | la préparation et la qualification après la bascule de lib 59ac | **utile mais à requalifier**, pour la lisibilité seulement : voir la section 4. |
| `b0a845bb48` (docs et intégration) | « clore les fixtures qualifiées » : le ticket est fermé sous `docs/tickets/closed/` | **utile, à garder**. Le vert annoncé (lib 2605/0/72, puis une seconde lib entière identique, navigateur 45, F8 8/8) est réel : `~/.cache/rag3weaver-build/cx-memoire-script-fixtures-union-qualification-v1/official/lib.log` et `second-lib.log` à luciepc. Mais ces journaux ne sont pas dans le dossier que sa page cite. La lib est la neuve (59ac), donc la règle est tenue. |
| Les pages `docs/memoire/16-11-2026/02`, `09`, `11`, `12`, `script-fixtures-union-r1-audit.json`, `…-range-diff.txt` | le récit de la qualification, sur six pages | **utile mais à requalifier**. Le fond est exact, mais c'est six pages, des JSON d'audit et un range-diff pour un correctif d'une heure. |
| 23 branches `cx-memoire-script-fixtures-*` (r1 à r4, pilot, proofs, union, resume-pilot, second-only-pilot, cleanup, integration) | les étapes de la même qualification | **à laisser telles quelles** : leur contenu utile a atterri par `b0a845bb48`, et une branche distante ne se supprime pas. Aucune n'est à intégrer. |
| `cx-memoire-script-fixtures-cleanup-r1` (`492745eab5`) et ses deux tickets : `2026-11-16-retirer-les-ressources-privees-des-fixtures-apres-atterrissage.md`, `2026-11-16-une-copie-de-garde-ecrase-un-chemin-preexistant-sans-empreinte-avant.md` | le nettoyage, et l'incident qu'il a causé | **utile mais à requalifier**. Les deux tickets ne sont **pas sur master** : ils sont à poser en docs, sinon le défaut connu n'a pas de ticket. L'incident relaté (une copie par scp a écrasé `~/.cache/rag3weaver-build/garde_worktree.py` à luciepc **avant** d'en prendre l'empreinte) est **nuisible** : le cache commun a été écrit hors de toute règle, et l'ancienne version est perdue. Rien n'est à réparer par intuition (voir sa passation). |
| Les commits `docs/procedures/scripts/*` (`c651bd0f83`, `6d0ce1415b`, `51e7975d88`, `4a78bf1ee7`, `866272adbb`, `2d389f8a8d`) | les retours d'instantané générés | **hors de mon périmètre** : ils servent la procédure des instantanés, et sont à juger par l'orchestration. |
| Les entrées 2.2 à 2.13 de ma page du 16 novembre | — | **intactes** : aucune branche, aucun commit. |

## 2. Verdict d'ensemble

- **Ce qui avance vraiment** : un seul défaut corrigé, le rouge sous charge de `codex_threads`. Il
  est bien corrigé, mieux que je ne l'avais proposé, avec un rouge déterministe et une
  qualification verte sur la lib neuve.
- **Ce qui est du papier** : six pages, 23 branches, des pilotes et des JSON d'audit pour ce seul
  correctif d'une heure ; et un temps de rôle passé en relectures sans exécution pour les autres
  sessions.
- **Ce qui gêne** :
  - l'écriture dans le cache commun de luciepc, avant d'en prendre l'empreinte ;
  - deux tickets restés en branche ;
  - une passation qu'on ne lit qu'avec son auteur (section 4).
- **Ce qui reste** : treize des quatorze entrées sont intactes.

## 3. Ce que ça change à ma page du 16 novembre

- **2.1 (Text file busy)** : **faite par Codex**, atterrie par `b0a845bb48` (le code en
  `7b3a88e13b`), ticket fermé. Codex a réécrit l'entrée dans la page, dans son style compact. Je la
  réannote proprement en tête de la page.
- **2.2 à 2.13** : intactes. Aucune n'est « en cours ». Elles reprennent dans l'ordre.
- **Une entrée de plus** : poser sur master les deux tickets de nettoyage de la branche
  `cx-memoire-script-fixtures-cleanup-r1`. Ce sont des docs seules ; je les relis avant.
- **Une règle à ajouter à la section 1** : la lib épinglée des passes est maintenant
  `59ac9c86bc`, et `RAG3WEAVER_MOTEUR_ANCIEN` n'a plus lieu d'être.

## 4. Lisibilité de la passation de Codex

**En partie exploitable sans lui.** Les faits y sont : des hashes qui existent, des chemins de
journaux, ce qui est gelé et pourquoi. Mais :

- les espaces manquent entre les nombres et les mots (« rapport53 », « lib2605/0/72 »,
  « sourceunion »), ce qui rend chaque ligne difficile à lire et à chercher ;
- le jargon n'est défini nulle part : ÀTOI, RO, FINI G, ownedquiet, F8, MODELED ;
- plus de la moitié de la page traite d'autres rôles (le runner d'archi, le protocole OS) ;
- elle ne dit pas en une ligne ce qui est fait dans ma liste, ce qui y est commencé et ce qui y
  est intact. Il faut la croiser avec git pour l'apprendre.

Ma réannotation de la page du 16 novembre est faite pour qu'on n'ait pas besoin de la lire.
