# Lot 3 — les écritures mécaniques : PostCompact, Stop, PreCompact, le handle, le rappel

7 novembre 2026, rag3weaver archi. Le troisième lot de la page 01 (§7), tranché par c7 et 96 :
**le hook écrit ce qui est mécanique** (`via: hook`, `author` = la clé) ; **ce qui est une parole
de l'agent reste à l'agent** — son apport `identity` lui est demandé, jamais écrit pour lui (mot
de Lucie du 25 octobre : « un agent est challengé de mettre à jour son identité à chaque
compaction »).

## 1. Ce que le hook écrit, et où

| événement | écrit | où |
|---|---|---|
| `PostCompact` | **le L0** : `put_source` `sources/<jour>-<HHhMM>-compaction-<id8>.md` (titre « Compaction du … — session <id8> », le résumé tel quel, `origine: compaction`, `session: session:<id>`, `via: hook`, `author` = la clé ou vide) ; **le L1 `compaction`** sur la **fiche de tâche** courante (la première `fiche:` de la rubrique `doing` de l'identity), `relier_apport`, `citer_source` `doc:<chemin>#<titre>` et `handle:session:<id>` | la mémoire |
| `PostCompact` sans fiche de tâche | le L0 seul (96 : jamais sur la fiche d'identité — elle cite la mémoire, elle ne la contient pas) ; une marque, et le `SessionStart` suivant dit « compaction rangée en L0 seul : nomme ta tâche » | la mémoire, une marque |
| `Stop` | **la fin de run** : `POST /journal/run` — `backend: claude-code`, `run: <session>-<prompt_id>`, `genre: claude_code_turn`, `session`, `agent` (la clé si connue), `cwd`, `tours` (le prompt, les réponses avec leurs appels d'outils, les résultats d'outils bornés à 2 000 caractères, lus dans la transcription pour ce `promptId`), `dernier_message` | le journal des runs de l'hôte, `runs/claude-code/<run>.json.zst` |
| `SessionStart` (`startup`, `resume`, `clear`) avec `RAG3WEAVER_AGENT_KEY` | **le L1 `handle`** sur la fiche d'identité de la clé — seulement si cette fiche existe et que `handle:session:<id>` n'est pas encore cité (la session cite déjà une fiche : rien) ; `texte: session:<id>`, `source: sessions`, `citer_source` genre `handle` | la mémoire |
| `PreCompact` | rien dans la mémoire : **les consignes du résumé** sur stdout (ce que le compacteur doit garder tel quel : les six rubriques de l'identité, le plan et ses pas, les worktrees et branches, les mandats mot pour mot, les pairs par leur clé, les hashes), avec le dernier `identity` connu en graine | le résumé de compaction |
| `UserPromptSubmit` | rien : au premier tour après compaction, la demande d'`identity` (lot 2) pose une marque ; au tour suivant, si aucun apport `identity` plus récent que la demande n'est là, **le rappel, une fois**, journalisé ; un apport venu ôte la marque | une marque |

Un genre de Ref qui manque est créé au passage (`create_ref_type` `handle`, `doc`, `confirm: true`)
; un refus « existe déjà » n'est pas une erreur. Toute écriture qui échoue va au journal avec sa
raison, et ce qui a été écrit avant reste (le L0 avant le L1).

## 1 bis. Corriger les hooks à la volée (exigence de Lucie, 7 novembre)

« Il faut qu'on puisse corriger ces hooks à la volée sans que Claude ait besoin de redémarrer ou
de charger un nouveau truc. » Claude Code lit la configuration des hooks au démarrage de la
session et ne reprend pas une modification sans relecture. Donc :

1. **Ce qui est posé dans `settings.local.json` ne change jamais** : une ligne de commande
   stable, `<binaire> hook <Événement>`, et rien d'autre (plus de `--config` dans la commande :
   le réglage est à son chemin par défaut, `~/.config/rag3weaver/hooks.json`). L'argument est
   relu contre l'entrée ; l'entrée fait foi.
2. **Toute la conduite se relit à chaque appel**, puisque chaque hook est un processus neuf : le
   binaire (remplacer `~/.local/bin/rag3weaver-backend` est pris au prochain appel), `hooks.json`,
   et **les gabarits de blocs, déclarés sous `templates/hooks/`** — un fichier par événement
   (`SessionStart.md`, `UserPromptSubmit.md`, `PreCompact.md`), une section `## <clé>` par bloc
   (`sans_fiche`, `session_compactee`, `compaction_l0_seul`, `sans_identity`, `defi_identity`,
   `rappel_identity`, `suggestion_contexte`, `consignes_resume`), embarqués dans le binaire et
   copiés dans `~/.config/rag3weaver/hooks/` par `rag3weaver-backend hook --poser-les-gabarits`
   (jamais écrasés) ; c'est la copie qui est relue à chaque appel, avec ses trous (`{rubriques}`,
   `{id}`, `{fiche}`, `{contexte}`, `{plafond}`, `{comment}`, `{genre}`). Une section absente de
   la copie vaut celle du livré.
3. **Le rejeu hors session** : `rag3weaver-backend hook --rejouer <transcription.jsonl>
   <Événement> [--prompt <texte>] [--source startup|compact]` reconstitue l'événement (la session
   est le nom du fichier, le prompt le dernier de la transcription), rend le bloc tel qu'il
   serait rendu et son journal, marque le journal `[rejeu]` — pour corriger et vérifier sans
   toucher une session ouverte.
4. **Le journal dit à chaque appel** l'empreinte du binaire et la version des gabarits (le
   dernier `mtime` du dossier, ou « intégrés »).

Témoin : un gabarit changé dans le dossier, l'événement rejoué, le bloc change, aucune session
relancée. La règle vaut pour la session `hooks-essai` de Lucie : ce qu'on corrige maintenant lui
arrive à son prochain tour.

**La suite, pas dans ce lot** (Lucie, par c7) : « que Claude lui-même sache redéclarer ses hooks
déclarativement, et les prompts des extractions L1, de l'archiviste, et les visualiser ». Les
gabarits de conduite — ceux des hooks, le `system_prompt` du juge, la décision de l'archiviste,
l'extraction des apports — étant déclarés, ils se redéclarent par le mécanisme des propositions
(propose → relecture dans l'IDE → appliqué sans redémarrage, puisque relus à chaque appel), et une
vue de l'IDE les liste avec leur version, leurs témoins et qui les a touchés. Ce lot pose la
première moitié : des gabarits déclarés, relus à chaud, versionnés au journal.

## 2. Dans l'hôte : la route `POST /journal/run`

Un run venu de dehors — un tour d'un agent qui n'est pas le nôtre — journalisé comme un run
racine, pour que le juge le lise comme un tour du chat (c7). Sous session et jeton, comme un
appel ; `backend` et `run` en `[A-Za-z0-9._-]`, `genre` obligatoire ; `run`, `backend`, `debut`,
`fin` (`RunFinished`, `externe: true`) et `evenements` posés s'ils manquent ; écrit compressé,
compté dans les bornes du journal, élagué comme les autres ; un run du même nom est réécrit
(`JournalDesRuns::deposer`). **Tranché par c7** : avec `manifest` dans le corps, le run porte
pour backend celui que l'hôte a ouvert pour ce manifeste (la mémoire que `hooks.json` désigne) et
**entre dans la file des bilans** comme un tour du chat, si ce backend déclare `bilans` ; sans
manifeste, un nom nu, gardé mais pas jugé. Ce que le juge doit faire d'un `claude_code_turn` (à
73) : ne rejouer que les appels à notre hôte (les outils MCP memoire-longue et code, liés par leur
handle), jamais les outils propres de Claude Code ; ouvrir le résultat par la fiche de la session
(`handle:session:<uuid>`) et ranger son bilan dessus ; les défauts d'outils ne portent que sur
nos outils. Aujourd'hui le juge lit `evenements` et `tours` ; un `claude_code_turn` a ses appels
dans `tours[].tool_calls` et pas d'événements du bus — la pièce côté juge reste à écrire.

## 2 bis. Les trois défauts du premier usage réel (c7, session hooks-essai)

- le bloc prescrivait `citer_source` genre `handle`, absent de la mémoire de Lucie : le hook lit
  maintenant le genre par `create_ref_type` sans confirmation avant de le prescrire ; absent, il
  dit lequel et l'appel exact pour le créer ;
- les apports écrits par l'agent revenaient avec `via` et `author` nuls : toutes les consignes
  passent `author = ta clé` (la clé nue) et `via: "claude-code"`, avec leurs arguments complets ;
  ce que le hook écrit porte `via: hook` ;
- le rappel rendait « régime GPU » pour un prompt sur les hooks : le bloc dit le score du meilleur
  signal (`chunk.score`, pas le rang fusionné, ~0,016 pour tout) et se tait sous
  `recall_min_score` (réglage, défaut 0,30 ; « régime GPU » sur sa propre requête fait 0,38) ; les
  scores de tout ce qui remonte vont au journal, pour régler le seuil sur l'usage.

## 3. Les retouches de la pose de G

G a posé le binaire de d42787227 à 14 h 49 et prouvé le verbe contre l'hôte vivant (un
`SessionStart` sans fiche : code 0, bloc de 525 caractères, deux lignes de journal). Deux retouches
qu'il a vues : le bloc finit maintenant par un retour à la ligne ; le journal nomme le backend par
le fichier du manifeste quand `/open` ne rend pas de nom. Et `rag3weaver-backend hook --help`
répond l'usage sans lire l'entrée ni joindre l'hôte.

## 4. Les témoins

- `claude_code_hook::tests` : `PostCompact` range le L0 et le L1 sur la fiche de tâche (`via`,
  `author`, `niveau`, le chemin, les deux Refs `doc` et `handle`) ; sans tâche, le L0 seul et le
  `SessionStart` suivant le dit une fois ; sans résumé rien, sans identité le L0 quand même ;
  `PreCompact` rend les consignes avec la graine, sans préfixe ; `Stop` dépose le run (`backend`,
  `run`, `genre`, `session`, `agent`, les quatre tours dans l'ordre, le dernier message) ; la clé
  donnée écrit le handle une fois, une session déjà citée n'écrit rien ; l'`identity` est
  redemandée une fois puis plus, et la marque tombe quand l'apport vient ; le client dépose sur
  `/journal/run` ; `--help` ; un gabarit corrigé change le bloc et la version au journal ; le
  rejeu hors session rend le bloc et son journal ; l'événement en argument relu contre l'entrée.
- `claude_code_transcript::tests` : les tours d'un prompt dans l'ordre (user, assistant avec
  `tool_calls`, tool, assistant), rien pour un prompt inconnu.
- `journal_des_runs::tests` : un run déposé se relit, se compte, se réécrit ; un corps qui n'est
  pas un objet est refusé.
- `daemon_host::tests` : la route sous session (401 sans, 200 avec, le fichier relu), 422 pour un
  nom traversant, un run vide, un genre absent, un manifeste non ouvert.
- `scripts/test_hooks_claude_code.sh` : la commande posée est `<binaire> hook <Événement>`.

## 5. Ce qui reste (lot 4)

`ranger` (M9) : la tranche de transcription depuis le dernier `Rangement` de la session, par
`archive` (pas 3 de 96) quand il sera sur master — c'est lui que `PostCompact` appellera sur le
résumé, les sections allant sur les fiches qu'elles concernent par la décision fermée ; le juge
sur un `claude_code_turn` ; l'essai de Lucie relu dans la mémoire (le L1 `compaction`, le run).
