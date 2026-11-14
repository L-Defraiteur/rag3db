# Archi (session Claude « rag3weaver archi ») — 14 novembre 2026

## Ce qui est sur master

| Lot | Master | Ce qu'il change |
|---|---|---|
| L'empreinte de lecture voit le contenu | `36ac9aea6` (13 nov.) | une écriture refusée si le fichier a changé depuis sa lecture, en disant quoi relire |
| openssl hors du bâti | `015172c00` | hf-hub et le reqwest des témoins en rustls ; douze paquets quittent le lock ; un target neuf bâtit sous openssl 4 sans rien poser |
| La recherche lit le catalogue comme l'état | `6981de5ec` | `lock_for_read` au lieu d'un `try_lock` nu : une lecture d'état ne fait plus balayer une recherche ; au-delà de 1 s, le repli le dit |
| Les thèmes de l'IDE | `061669ce5` | `/ide/theme/<nom>.json` et `/ide/themes.json` depuis les gabarits embarqués ; le menu des cinq thèmes |
| La jauge du contexte dit ce qui pèse | `cdbd346aa` | instructions, outils, historique, résultats d'outils, au prorata, au survol |
| Les quatre mineurs de l'atelier | `d4f0b5a74` | occurrences qui se chevauchent, CRLF, worktree attaché hors racine, création depuis le chat |
| Les cinq clients sans délai | `69b2f5474` | `sonde_s` (5 s) et `reponse_s` (120 s) dans daemon.json ; « <route> : ne répond pas après <n> s » |
| La fermeture qui tient | `1cbf1cf3a` | `/backend/close` avec une raison tient jusqu'à `/backend/open` ; le refus dit qui et jusqu'à quand |
| Le fil index-suivi joint | `0a5cd8168` | l'hôte joint ses suiveurs de premier index en se défaisant |

En attente de la pile : **l'équipier rend le contenu demandé** (`equipier-rend-le-contenu`,
`47b1955de`, vert) — `report` porte le texte lui-même, et le texte d'un pas qui finit par
`step_done` va dans la boîte.

## Ce que la soirée a appris

- **Un rouge de suite se départage par deux bornes jouées.** Le rouge de réouverture
  (`e2e_demon_hote::une_base_a_rouvrir_ne_lache_que_son_backend`) a d'abord été mis sur le compte
  de la lib, puis de deux lots du soir ; trois runs rouges, un `git bisect` qui avait pris sa borne
  « bonne » sans la jouer, puis le dernier vert archivé (13 nov., 8 h 01) et deux jeux croisés lib
  contre Rust ont nommé `1e9bf5089e` (arbre principal), corrigé par `a76f36a08d`.
- **Une aide de test qui avale l'erreur cache le rouge.** `fiche()` rendait `None` sur tout
  `ok: false` : une ligne `eprintln!` a donné le vrai message (« already open for writing — 1
  référence tenue après 10 s »), et G l'a relu sous deux libs.
- **Le verdict d'une bascule de lib doit jouer e2e_demon_hote** : ce rouge y a dormi depuis le 13.
- **Un seul target par session, gardé entre les lots** (rappel de c7, trois fois) ; repris d'un
  autre worktree par `RAG3WEAVER_TARGET_REPRENDRE=1` ; au plafond de 32 Go, il s'efface et se
  rebâtit (deux fois ce soir).
- **La base testée est le parent du premier commit du lot**, pas `origin/master` relu plus tard.
- **Un `git mv` suivi d'un commit sans `-a`** n'emporte que le déplacement : le contenu édité reste
  hors du commit (le ticket openssl, rattrapé).
- **Un vrai modèle ne se rougit pas à la demande** : le cas Gemini de l'équipier était vert une fois
  sans le correctif ; le rouge sûr est le faux modèle.
  Banc, 14 nov. 23 h 58 – 0 h 10, luciepc, Vertex : la demande du ticket jouée dix fois sans le
  correctif (47b1955de moins ba2994a97) et dix fois avec (47b1955de), jugée par
  `l_equipier_rend_le_texte_demande_et_non_un_compte_rendu` — le contenu dans la boîte 10/10 sans,
  10/10 avec (`docs/banc/14-11-2026/03-l-equipier-sous-gemini.md`). Des deux côtés le contenu est en
  `report` et la phrase de compte rendu en `step_done`. **Le banc ne reproduit donc pas le défaut** de
  la séance de tty : sa tâche (un dépôt de test, sans l'historique ni les déclarations de la vraie
  tâche `essai-tty`) n'est pas la séance du ticket. Ce que le correctif garantit est ce que le faux
  modèle prouve (un texte écrit dans le pas arrive ; la consigne dit où va un texte demandé), pas un
  taux de Gemini.

## Reprendre

Rien en cours hors de la pile. L'équipier attend sa base (derrière la chaîne Rust de G et les
orphelins de recherche) ; worktree `~/.cache/rag3weaver-build/wt-equipier`, target `delais-c`,
journaux `equipier-complete/` (resume.log et base écrits).
