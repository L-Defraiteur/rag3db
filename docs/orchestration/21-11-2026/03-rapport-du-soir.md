# Rapport de session de c7 — 21 novembre, après-midi et soir (jusqu'à 23 h 40)

Suite de la page 01 (nuit et matinée) et de la page 02 (note de pause avant le redémarrage). Écrit pour reprendre la
main sans rien d'autre, après la compaction du contexte de c7.

## L'état à 23 h 40

- **master** : `1a1539034c`, l'arbre de Lucie y est avancé. File du pousseur vide à 23 h 39.
- **Lib épinglée** : toujours `424ce93e56`. **Hôte en service** : celui de 08 h 34 (c5f3bb819e), à reposer au matin.
- **Tablette** : `/tmp` sur le SSD depuis le redémarrage de 15 h 4x (ligne tmpfs de `/etc/fstab` commentée et
  `tmp.mount` masqué par Lucie) ; ~300 Go libres après le ménage du disque plein (4,6 Go libres à 13 h 1x) ; bâtis et
  passes à luciepc.
- **Juge du mode auto servi à la tablette** depuis 21 h 58 : llama-server b12603 Vulkan, Qwen3.5-9B Q8_0,
  `127.0.0.1:8096`, `poste gpu 12 -- timeout 14h` (jusque vers 12 h le 22), journal
  `~/.cache/rag3weaver-build/juge-8096.log`. `~/.config/rag3weaver/gate/rules.json` posé par c7 au oui de Lucie
  (prompt P4 copié dans `~/.config/rag3weaver/gate/prompts/yn-P4.txt`, seuil 0,5).
- **lucivy 4.4.1 publiée** (20 h 30, au mot de Lucie dans la session lucivy) ; (A) atterri sur elle (PR 471).
- **`ant` 2.12.1 installé** par Lucie (script `~/.cache/installer-ant.sh`) ; pas encore de `ant auth login`.

## Atterri cet après-midi et ce soir (PR)

Accueil en arbre des agents (419) ; dossier sans index, index à la demande, offre d'index, dossier non-git (405, 406,
415, 429) ; diffs des vues (412) ; `/ide/run` (423) ; registre des racines (418) ; bac et lecture seule par défaut
(441) ; dossiers protégés (456) et profil d'`ant` + état de l'hôte protégés (465) ; défauts de la v1 du bac (470 :
racine de création `~/git_workspaces/projets`, caches nommés, réseau ouvert, `judge_all`, dossier protégé ouvert pour
une commande sur un oui humain, `strict` en option) ; carte d'accord, cœur (454) et IDE (472) ; signature de la
mémoire par l'hôte, M7 (445) ; le navigateur de l'équipier en lecture (448) et le contenu du dehors marqué non sûr
(452) ; vue qui suit son worktree (457, rendue fiable 473) et sa fraîcheur (474) ; liens à clé et retrait du
contournement d'octobre (438, 451) ; reprise par empreinte (435) ; rechargement à chaud des nœuds (444) ; `poste cpu`
/ `poste gpu` (426, noms anglais 431) et leurs procédures (427) ; confineur réparé (428) ; relevé de charge (417) ;
index de master à luciepc suivi par ff (439) ; pages : porte du mode auto gelée (07, PR 440/447/459/462), un client
MCP à plusieurs hôtes (434/437), juges de banc (424, 443, 446, 449, 460, 461, 463), batch Gemini (458), forme B
d'embarquement (450, 468), facturation d'ED (469), score négatif de lucivy (467), choix wasmi (433) ; tickets.

## Décisions de Lucie (après-midi et soir)

- **Occurrences** : voie (a), liens à clé ; d'abord retirer le contournement (fait).
- **Mode auto** : v1 d'abord, puis « tout de suite au bout ». Racine par défaut pour les projets neufs ; le bac
  n'empêche pas tout (réseau, caches, `judge_all`) ; dossiers protégés ouverts sur carte et oui humain, pas « jamais » ;
  « jamais au-dessus du projet » est une consigne d'invite, pas une limite ; une racine large va à sa carte sans le
  juge (sauf l'option du survivant) ; `npm install` seulement sur demande explicite ; l'accord se lit sur la paire
  (tour de l'agent, réponse) en quatre valeurs ; une fabrique de classifieurs, le tout en config (graphe : hook,
  collecte, classifieurs, décision) ; le survivant reçoit les tours utilisateur et agent et cherche lui-même les
  accords ; **survivant = Gemini 3.8 Flash** pour l'instant ; **Claude ne se branche pas** avant le registre des
  dépenses ; plafond **200 USD par mois** ; tâches de fond qui notifient l'agent.
- **Outils d'agent** : contrats explicites, aucune syntaxe parsée dans un texte (`query` en texte pur, paramètres
  typés, requêtes lucivy construites) ; les ratés du MCP sont bloquants pour la v1.
- **Forme B** (partage du vecteur d'un texte) : « oui absolument », migration sur copie d'abord.
- **lucivy** : 4.4.1 publiée ; la 4.4.2 (score BM25 × couverture au lieu de −1000 par trigramme manqué) attend sa
  relecture et son mot.
- **Ornith 122B uncensored** retéléchargé puis déchargé (il inventait des outils) ; « pas la prio ».
- **Graines notées** : resurgissement d'une base de code (descente en spécification, description des relations par
  leur raison) ; travaux de fond en batch ; rêveries de l'archiviste (laissées de côté).

## Ce qui tourne ou attend, pour l'essai de Lucie le 22 au matin

- **archi** : `mode-auto-v1-2` (signature humaine + porte, scénario de six cas vert), passe de dépôt à luciepc ;
  `fermeture-garde-la-tete-2` ; répétition React avec le vrai juge ; marche à suivre pour Lucie au matin.
- **tty** : `cx-user-token-2` (signature au clavier), `cx-tty-card` (carte du terminal), puis les tâches de fond.
- **mémoire** : masquage des secrets 5a/5b (l'identité de Lucie jamais masquée), puis `put_fiche`/`put_apport`.
- **recherche** : `search-words-built-2` (texte pur, plus de `parse`), passe à luciepc.
- **codeparsers** : `new-name-mentions-r` (ordre déterministe du drain, liens sans bout comptés, règles **16**) ;
  réindexation mesurée à **8 min** pour l'index principal ; **à confirmer par Lucie** : l'hôte du matin la porte-t-il
  (pose vers 6 h) ? Puis le double compte de `usages`.
- **hotfixs** : correctif de la course de l'allocateur (fsm-finalize-lock, 9645be625b) **VERT depuis 21 h 31**, en
  attente du « à toi » de G pour la voie du C++, puis lib rebâtie, verdict, épinglage. Borne : sans VERT poussé à 2 h,
  l'hôte du matin se pose sans lui.
- **G (optimiseur)** : en attente dans sa fenêtre (demande de permission à Lucie) à 23 h 15 ; doit ouvrir la fenêtre
  C++, puis poser l'hôte du matin. Petits défauts à lui : `bash -n` de `poste` (ligne 587), verrou injoignable pendant
  un redémarrage, sccache, garde du disque, réserve GPU illisible qui valait 0.
- **cœur** : séries C1 avant/après le correctif (seuil du point de reprise à 1 Mio), résultat vers 5 h.
- **embarquement** : forme B, L1 (schéma et migration sur copie), derrière `shared_text_vectors` éteint.
- **ED** : verdict WASM, puis le registre des dépenses.
- **banc** : juge et survivant mesurés ; cartes par heure sur les runs de tty quand archi les a.

## Les leçons du jour

- Un disque plein gèle un poste à écran comme un OOM : targets à luciepc, ménage sur relevé d'usage (cwd, exe,
  cmdline, fichiers ouverts, `CARGO_TARGET_DIR`), jamais un build/ de série sans son auteur (faute de c7 : série C1
  coupée).
- Un `kill` ne se fait jamais sur une liste calculée (incident codeparsers à luciepc).
- Un texte lu par d'autres a des lecteurs (« tient cpu » a fait boucler le confineur).
- Une hypothèse de concurrence se tient par une garde en code et un témoin, pas un commentaire (Lucie).
- Un témoin à borne de temps tombe sous charge : attendre l'événement, mesurer le délai à part.
- Un sous-agent qui cherche le contre d'une hypothèse la corrige vite (idée de Lucie, prouvée chez codeparsers).
- Annoncer un chiffre seulement mesuré (c7 a relayé deux affirmations de codeparsers corrigées ensuite, et une fausse
  alerte sur `poste`).
