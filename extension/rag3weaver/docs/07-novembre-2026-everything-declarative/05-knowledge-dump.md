# Knowledge dump — everything declarative, 7 novembre 2026

## Les postes et les passes

- Tout se joue à luciepc, sous `systemd-run --user`, dans mon worktree `rag3db-I-apres`, cible
  `target-I-apres` (une seule ; plafond 20 Go : effacée une fois à 23 Go). Ici, `wt-react5` seul
  (node_modules de ui/ide).
- La lib : `RAG3DB_LIB=rag3db-build-17a094d12…` (format 43) et `RAG3WEAVER_MOTEUR_ANCIEN=1` jusqu'à
  la lib ordinaire suivante (elle ne porte pas c4f8610b9) ; `I-court-reprise.sh` à luciepc porte
  l'id, le jeu large (`…,openai-llm,anthropic-llm`) et `RAG3WEAVER_PG`.
- Le Postgres de test de luciepc est le conteneur `rag3weaver-pg` (port 5433), à `docker start`
  après un redémarrage du poste.
- Mes scripts : `I-pousse-<lot>.sh` (la passe sous `poste lourd`, puis le pas du push sous
  `poste push` par `I-pousse-l2.sh`, qui refuse sans `l2/verdict` = la tête jugée VERT, et sort en
  `REJOUER` si master a reçu du code) ; `I-<lot>-un.sh` pour des témoins seuls avec leur sortie ;
  les messages de commit dans `~/.cache/rag3weaver-build/I-msg-*.txt`.
- La passe d'un lot qui touche l'équipier ou la boucle : lib entière + check_features, l'étape
  navigateur (`RAG3WEAVER_PAQUETS_REACT`), le module du chat, e2e_lectures, e2e_tty (le témoin de
  tty compile contre `ChatModel`), e2e_juge et e2e_equipier sur Gemini (identifiants par le chemin
  de `.vault` seulement).

## Le code de ce jour

- `src/daemon_host/teammate_tools.rs` : `Etanche` (le backend par appel, le repli) et `FileTools`
  (read/list/grep/edit/search sans index, chemins confinés, `expected_hash`).
- `src/daemon_host/chat.rs` : `Tour` (ce qu'un tour prend du backend) ; `run_chat_turn` sert le chat
  et le pas de l'équipier ; `run_teammate_turn` dans teammate_tools.rs.
- `src/daemon_host/teammate.rs` : `TeammateFiles { dir, key, relay }`, `append_with`, l'id
  `<parent>/<task>:<n>`, `pending_for`, `seen`, `teammate_list`, `addressed_to_teammate`,
  `reprendre_les_equipiers` (appelé dans `serve`), la boucle (bus facultatif, réveil par fichier,
  pas raté).
- `src/backend.rs` : `TeammatePolicy` (`max_failed_steps`, `tool_wait_notice_s`,
  `resume_after_restart`) ; `BilansDeclares.max_per_hour` (en file).
- Les formes accordées : la boîte (page 01 § 2, avec archi et tty), la jauge `context`
  (avec archi), la barre `chat_sessions` / `tasks` / `Space` (page 02, avec hooks-essai).

## Les sessions voisines

c7 (orchestration, `rag3db-74`), G (`optimiseur`, poses aux hôtes 7970), archi (`rag3weaver archi`,
hooks et pont Codex), hooks-essai (le chat de l'IDE, les tâches hors dépôt), tty (le terminal),
96 (`mémoire`), recherche, codeparsers, l'arbre principal (73).
