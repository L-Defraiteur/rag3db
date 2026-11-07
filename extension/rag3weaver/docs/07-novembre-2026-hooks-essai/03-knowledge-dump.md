# Knowledge dump de hooks-essai (7 novembre 2026, ~22 h)

Ce qu'une session qui reprend ce travail doit savoir sans relire le fil.

## Où sont les choses

- **Le worktree** `~/git_workspaces/rag3db-hooks-essai`. Sa branche courante
  est celle du lot ; les docs de session sont dans le worktree de docs
  `~/.cache/rag3weaver-build/wt-docs-hooks-essai` (branche `hooks-essai`).
- **Les scripts à luciepc**, dans `~/.cache/rag3weaver-build/` :
  - `passe-lot-hooks-essai.sh <branche> <tête> <master> <id lib> <ancien 0|1>`,
    avec `SUITES="…"` pour les e2e : la passe hors verrou, puis le dernier pas
    sous `poste push` (`push-trois-hooks-essai.sh`) ;
  - `lib-un`, `nav-un` et `e2e-un-hooks-essai.sh` pour un test ciblé.
  - Tous sont épinglés sur
    `rag3db-build-17a094d12d8c29418fbf3b3ed941ad64e73b4c29-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`
    avec `RAG3WEAVER_MOTEUR_ANCIEN=1`, jusqu'à la prochaine lib ordinaire (la
    lib ne porte pas c4f8610b9).
- **La règle de rejeu au push** : seul le code arrivé sous `extension/rag3weaver`,
  hors `.md`, impose de rejouer la passe ; le C++ du moteur (`src/`, `test/` de
  la racine) et les docs se rebasent sans rejeu.
- **La mémoire** : fiche `hooks-essai`, L1 `handle` et L1 `identity` (révision 3,
  sept lignes) ; `author: "hooks-essai"`, `via: "claude-code"`.

## Les formes posées (à ne pas casser)

- `ToolCall.at_ms` (départ de l'appel), `Turn.at_ms` (retour du résultat) :
  facultatifs, serde skip.
- `ToolCallFinished.result_excerpt` (4 Kio, frontière de caractère),
  `truncated` : absent quand l'outil ne rend rien.
- `role` sur `ToolAttachment` et `ReactionAttachment`, relayé dans
  `RunStarted.target.role` et `describe()`.
- **`GET /ide/chat`** rend :
  - `context: {used, max, source, at}` : used est le prompt du dernier appel,
    max vient de `Llm::context_len` ;
  - `memory_writes` / `memory_writes_why` : par `select_sources` et
    `select_apports` de la mémoire déclarée (`bilans.memoire`), seulement si
    l'hôte la tient ouverte, sur les vingt derniers runs, borne 500.
- **`/ide/etat`** rend `repo`, `branch` (lus dans le `HEAD`, sans processus),
  `kind` (code, memory ou project) et `kind_source` (manifest ou fallback).
  `BackendManifest.kind` est posé dans les gabarits `code` et `memory`.
- **Le flux de l'IDE** : l'id est tiré sous le verrou `abonnes`, avec le tampon
  et l'envoi.
- **`/call` et `deadline_ms`** (en file) : en cas de dépassement, 422 avec
  `error_kind: "deadline"` et `transient: true` ; le run finit avec
  `cancelled_by: "deadline"`.
- **La page du chat** :
  - parties dans l'ordre d'arrivée ; la session fait foi à la fin du tour ;
  - `window.__chatTokens` (rangs reçus et posés, débordements) pour les
    témoins ;
  - le chat en plein écran pour sa seule route.

## Les témoins neufs

- **Par l'hôte** :
  - `un_tour_a_deux_outils_garde_l_ordre_et_l_heure_de_ses_appels` ;
  - `le_role_d_un_outil_…` ;
  - `une_reaction_a_role_l_annonce_dans_son_run` ;
  - `l_etat_du_chat_rend_les_fiches_…` ;
  - `sans_outil_de_lecture_…` ;
  - `la_nature_d_un_backend_vient_de_son_gabarit_…` ;
  - `un_appel_a_delai_s_arrete_a_son_delai` ;
  - `le_journal_de_l_hote_dit_l_heure_et_l_auteur_…` (e2e MCP).
- **Navigateur** :
  - `la_page_du_chat_tient_l_ordre_le_defilement_et_la_place` ;
  - `la_barre_du_chat_parle_en_depots_…` ;
  - `le_premier_jeton_d_un_tour_arrive_dans_la_page`.
- **Unitaires** :
  - `relays_publishing_together_lose_nothing_and_keep_the_order` ;
  - `la_branche_d_un_depot_et_d_un_worktree_se_lit_dans_head` ;
  - les tests de `host_log`.

## Ce qui attend

- **`appel-a-delai`** : au « à toi » de c7, rebase sur son hash, puis
  `SUITES="e2e_lectures e2e_demon_hote e2e_mcp_cycle_de_vie"`, la passe, le
  push, le hash à c7, archi (son lot d), ED et G, puis la ligne de nouveautés
  dans la minute.
- **Chez Lucie** :
  - sa mémoire paraît sous « Projets » (repli) jusqu'à la montée de ses
    gabarits par 96 (M8) ;
  - `bilans.memoire` dans son manifeste de code allumerait le juge et les
    fiches écrites. C'est son mot.
