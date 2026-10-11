# Knowledge dump de l'orchestration — 11 octobre 2026, 21 h (soir), mis à jour le 12 à 0 h 40

Remplace `03-knowledge-dump.md` (midi) là où il diffère ; ce qui n'est pas
répété ici y reste vrai (les projets, l'architecture par couche, la méthode).
Pour une session vierge qui reprend l'orchestration.

## 1. Les sessions et leurs noms (à ne plus confondre)

| Nom dans ListAgents | Chantier | Tient |
|---|---|---|
| `coeur c++` (relancée, contexte entier) | le moteur C++, condition 3, le planificateur | luciepc : worktrees par lot, la lib commune `rag3db-lourd` |
| `rag3db-cc` (hotfixs, relancée) | hotfixs et tickets moteur, `CALL build_id()`, le visited set | luciepc |
| `rag3db-10` | le banc de concurrence, les témoins avant le code, le lecteur du journal | luciepc en `poste mesure` |
| `rag3db-73` | l'arbre principal : backend, catalog, code_tools, l'hôte du démon, les routes, le rendez-vous, les manques du MCP | la lib partagée d'ici (`build/lecteurs-csv`) |
| `rag3db-6f` (F) | les dialectes et l'IR : échappement, chaînage, PostgreSQL, les mesures par phase, la relecture de dialect.rs | le conteneur PostgreSQL (port 5433) |
| `rag3db-96` | la mémoire longue : le client MCP, les marches 3-5 du démon, l'hôte de Lucie (binaire `~/.local/bin`), les fiches classées, l'enveloppe | l'hôte de Lucie ici |
| `rag3db-ab` | codeparsers : la couverture, le banc des absents, les index de code (ici et sur luciepc), `code-tree`, la page des manques | les manifestes de code, l'hôte de luciepc (arrêt/relance) |
| `rag3db-97` | la recherche : l'exécuteur du dataflow, les flux, les e2e en un binaire, check_features, la garde du target | `scripts/` du crate |
| `rag3db-90` (G) | le paquet npm, `poste` et `poste push`, l'unité systemd, l'hôte de luciepc (binaire), les lots A/B/C de portabilité, B4 les identités relatives | luciepc : binaire, unité, relais du verrou |
| `everything declarative` | le proto tout déclaratif, les agents qui se parlent, la montre de fichiers, Decide/Route, la réconciliation | `src/atelier.rs`, `src/watch.rs`, les réactions |
| `lucied-68` | la session de Lucie, pas cadrée | — |

Codeparsers = `rag3db-ab`, le banc = `rag3db-10`, G = `rag3db-90` : trois
erreurs d'adressage aujourd'hui venaient de là.

## 2. L'état des hôtes et des binaires (au matin du 12 octobre, 0 h 40)

- **Ici** : l'hôte de Lucie (`~/.local/bin/rag3weaver-backend`) **redémarré à
  0 h 26 par la mémoire** sur le binaire md5 3e95f0302023 (master eabc53e86 :
  la montre, les défenses contre la boucle, le client qui se rétablit seul),
  lié à la lib d'ici de 20 h 39 (inode 29309980, d'avant le visited set et
  la section ELF) ; l'ancien gardé en `.avant-defenses`. Bases memoire-longue
  et code-rag3db (montre à 1 s, réconciliation au démarrage). **La lib d'ici
  est à rebâtir au matin** (visited set `3e0f98f08`, section
  `.rag3db_build_id` `f5fdc7e9a`) par l'arbre principal sur les « fini », puis
  le binaire de Lucie rebâti dans la foulée par la mémoire, les deux annoncés
  ensemble ; d'ici là, les e2e déclarent `RAG3WEAVER_MOTEUR_ANCIEN=1`.
- **luciepc** : **l'hôte est figé depuis 22 h 02** (pid 132823, code-lourd à
  20 % des vecteurs, la boucle de réclamation sur des morceaux sans texte,
  prouvée) et **gardé pour Lucie** : personne ne l'arrête. Devant la place
  partagée du poste de luciepc, le client d'index de codeparsers (pid 132918,
  `index.sh lourd principal`) tient depuis 22 h ; derrière lui la mesure de
  hotfixs (pid 242676) et des lourds (le bâti du binaire de G). **Au réveil,
  dans l'ordre** (section « Pour Lucie » du rapport de codeparsers,
  `docs/10-octobre-2026-18h20/codeparsers/01-rapport-de-session.md`) :
  terminer 132918 (libère la place) ; arrêt propre de l'hôte
  (`systemctl --user stop rag3weaver-daemon`, pile gdb facultative) ;
  compter base fermée les Scope_Chunk sans texte de code-lourd ; remettre
  la montre (les deux `cp backend.json.avec-montre backend.json`) ; installer
  le binaire prêt de G (`mv ~/.local/bin/rag3weaver-backend.pret
  ~/.local/bin/rag3weaver-backend`, G le rebâtit sur la tête avec code-tree
  et les défenses dès que la place le permet) ; `systemctl --user start`
  puis `/sante`. La lib commune `rag3db-lourd` de luciepc est celle de
  20 h 39 ; hotfixs la rebâtit après l'arrêt de l'hôte. code-principal n'a
  pas été réindexé ; code-lourd à réindexer sur le binaire neuf.
- **Les serveurs MCP de Lucie** : `memoire-longue`, `code-rag3db` (ici, de
  nouveau servis depuis 0 h 26), `code-rag3db-luciepc` (sur code-principal,
  mort tant que l'hôte de luciepc est figé). Recette : `templates/mcp/README.md`.
- **Le conteneur PostgreSQL** : `rag3weaver-pg`, pgvector pg17, port 5433.
- **Le défaut du jour, ticket P1 `6cdef9585`** : 20 % des morceaux de code
  sont écrits sans texte (26 092 sur 128 732 ici ; aucun en mémoire chez
  codeparsers : la perte est à l'écriture, chargement CSV en masse
  suspecté) ; ni cherchables ni embarqués ; le compte est dit en tête de
  chaque lecture du MCP depuis `ab12bd4c2`. Cause chez l'arbre principal,
  sonde par l'hôte au matin. **Question pour Lucie** : un morceau sans texte
  doit-il exister ou ne jamais naître.

## 3. Le flux de travail des sessions (règles vivantes)

- **Passe complète hors verrou, rejeu court sous le verrou** :
  `~/.cache/rag3weaver-build/poste push <session> --lourd -- <commande>` ;
  la commande fait fetch, rebase sur FETCH_HEAD, arrêt si rebase en cours,
  `git submodule update`, `scripts/check_features.sh` (cinq jeux + tickets,
  une ligne finale `CHECK_FEATURES VERT n/n` et un code), les suites du lot,
  contrôle que master n'a pas bougé (hors docs seules, et hors chemins hors
  du bâti du lot), push ; boucle sur une avance de .md ; vert = aucune ligne
  ROUGE + code 0 lu sans tube. Page : `docs/optimiseur/11-octobre-2026-15h27/01-poste-push.md`.
- **« Poussé »** = status propre, aucun rebase en cours, hash lu sur
  origin/master après fetch. Un ticket se ferme par `git mv` dans le commit
  qui corrige, avec sa ligne du README ; `scripts/check_tickets.py` recompte.
- **Targets** : un par session, gardé entre les lots, effacé en fin de
  session, plafond 20 Go ; un sous-dossier par arbre quand plusieurs
  worktrees ; `scripts/garde_target.sh` refuse un target bâti depuis un
  autre arbre (`RAG3WEAVER_TARGET_REPRENDRE=1` ou sous-dossier).
- **La lib partagée** : rebâtie par l'arbre principal (ici) sur demande des
  « fini », par le cœur/hotfixs (luciepc) en prévenant codeparsers ; une
  annonce dit « ABI changée » quand l'API publique bouge ; les binaires
  installés se rebâtissent avant toute relance d'hôte ; `RAG3WEAVER_MOTEUR_ANCIEN=1`
  déclaré pour un lot du produit seul en attendant.
- **Les hôtes** : un redémarrage s'annonce aux sessions qui l'interrogent ;
  jamais `kill -9` sans Lucie (trois fois ce soir, sur preuve : bases
  gardées fermées sans journal, seule la base à jeter ouverte) ; aucune
  réindexation entière d'un backend qu'une session interroge pendant qu'elle
  travaille ; un worktree et sa base vivent et meurent ensemble (la session
  qui tient le worktree fait créer et retirer le manifeste).
- **Le poste, depuis la nuit** (G, `e70a78a72`, `ced1f2722`, dans le dépôt
  sous `scripts/poste` avec `test_poste.sh`, onze cas, déployé sur les deux
  postes) : une mesure qui attend bloque tout nouveau lourd ; le délai d'une
  mesure ne court qu'une fois la place prise (`POSTE_MESURE_MAX=<s> poste
  mesure …`, 124 au dépassement, **plus jamais `timeout N poste mesure`**) ;
  un poste sous un poste du même arbre de processus passe seul ;
  `poste --qui` dit qui tient et qui attend depuis quand. **Un rejeu qui
  tourne sur l'autre poste prend sa place sous le poste lourd de là-bas,
  `--lourd` local inutile** ; et la **forme optimiste** du push (proto,
  `fdef26450`) : rejeu sur une base notée hors du verrou, sous le verrou
  seulement fetch, comparaison (égale ou docs seules), push, sinon rendre et
  rejouer. Les lots lourds contre un moteur qui bouge demandent une fenêtre
  à l'orchestration (G, lot A : quatre courses perdues puis le rejeu sous le
  verrou, 20 min).
- **Les hôtes, depuis la nuit** : « au repos » = journal soldé (retard,
  reconstruction), pas CPU nul (le redémarrage de minuit a coupé un
  rattrapage qui attendait le service) ; un refus de permission dans une
  session ne se contourne jamais par une autre ; la boucle se reconnaît au
  compte d'index_states figé, au service au repos (ou pas : le trafic ne
  prouve rien), à un cœur pris en continu et au `.wal` qui grossit
  régulièrement.
- **Les corpus vivants** : e2e_code, e2e_usages, e2e_impact indexent le
  dépôt : un relevé périmé par un lot d'un autre se met à jour à part, et le
  ticket demande un module-corpus stable.

## 4. L'IDE pour agents, ce qu'il sait et ce qui lui manque

Outils du gabarit code par MCP : usages, impact (avec « verrous pris sur le
chemin »), callees, search_code (signals bm25 ou vecteur), read_file,
grep_files, list_files, index, estimate, schema, daemon_status. Chaque réponse
commence par « index : racine @ commit, indexé il y a N min » (depuis l'envoi
virtuel) et dit son retard (montre). Les manques, tenus dans
`10-octobre-2026-18h20/codeparsers/04-ce-qui-manque-a-un-agent-dans-cet-ide.md`
par les sessions elles-mêmes : le nom qualifié rend zéro ; « 0 tests qui la
traversent » là où les tests passent par Cypher ; borner à un dossier ou un
langage ; les verrous du départ dans l'ordre ; search_code confiant sur un
jeton absent ; l'heure de la réponse ; et surtout **l'index doit suivre
l'arbre que l'agent édite**. Les mesures : `usages` 0,3 à 1,8 s sur l'index
entier (le planificateur balaie la table Scope : ticket moteur) ; une édition
vue en 5 à 6 s ici, jusqu'à 11 s sur luciepc (mesure par phase en cours ;
codeparsers hors de cause, < 10 ms par fichier).

## 5. Les scripts et où ils vivent

- `~/.cache/rag3weaver-build/poste` : `lourd`, `mesure`, `push` (`--lourd`,
  `--qui`), relais luciepc (`poste-push-hote`), sauvegardes `poste.avant-*`.
- `extension/rag3weaver/scripts/` : `check_features.sh`, `check_tickets.py`,
  `garde_target.sh`, `test_garde_target.sh`, `run_e2e.sh` (binaire e2e unique,
  `tests/e2e/main.rs`, `generer.py` pour une suite neuve, dossier par suite),
  `pousser*.sh` propres à chaque session (F : `embarquements/pousser.sh` avec
  FERMER_TICKET/FERMER_LIGNE ; arbre principal : `pousser2.sh`, `pousser-poste.sh`).
- Depuis la nuit : `extension/rag3weaver/scripts/poste` et `test_poste.sh`
  (le poste est dans le dépôt, `~/.cache/rag3weaver-build/poste` en est la
  copie déployée) ; `scripts/rag3db_libs.py` (le magasin de libs par
  identifiant de bâti : publier, courant, liste, menage ; page
  `11-octobre-2026-recherche/07-le-magasin-de-libs.md`) ;
  `rag3db-check-lib <binaire> <lib>` (le refus nommé avant le chargeur, par
  les sections ELF `.rag3db_build_id` et `.rag3db_compiled_against`) ;
  `scripts/test_pont_recompile.sh` ; `rag3weaver-backend code-tree add|remove`
  (codeparsers, `a601b8308`).
- Les notes hors dépôt : `~/.cache/rag3weaver-build/notes-G/`,
  `~/.cache/codeparsers-notes/`, `~/.cache/rag3db-moteur-notes/`,
  `~/.cache/rag3db-banc-notes/`.
- Mon worktree de docs : `~/.cache/rag3weaver-build/wt-orch`, détaché de
  origin/master, commit par chemins, push avec reprise ×3, retiré après.

## 6. Les pages à lire pour chaque chantier (les plus récentes)

- Orchestration : `01-les-decisions-du-matin.md` (le fil de la journée),
  `02-rapport-de-session.md`, `03-knowledge-dump.md`, `04-les-buts.md`,
  `05-rapport-de-session-soir.md`.
- Démon : `11-octobre-2026-arbre-principal/01-le-demon-possede-le-catalogue.md`
  (§9 les choix, §7 ce qui reste), `04-la-marche-1-bis.md`,
  `03-l-enveloppe-d-un-outil-d-ecriture.md` ; `templates/mcp/README.md`.
- Agents et montre : `11-octobre-2026-everything-declarative/03` et `04`.
- Dialectes : `11-octobre-2026-embarquements/01` (chaînage), `03` (appelants),
  `05` (Cypher brut), `06` (où va le temps), `07` (échappement, 10 oct.).
- Flux : `11-octobre-2026-recherche/03` (e2e en un binaire, §8-§11),
  `04` (exécuteur de flux), `05` (concurrent), `06` (rapport du jour).
- Code : `10-octobre-2026-18h20/codeparsers/04` (manques), `05` (marques),
  `06` (code-tree) ; `10-octobre-2026-18h20/orchestration/04-indexer-nos-propres-depots.md`.
- Moteur : `3-octobre-2026-23h31/coeur-cpp/11` (condition 3), `12` (petites
  cardinalités), la stèle (`docs/4-octobre-2026-16h57/01-la-stele-du-moteur.md`).
- Paquet et poste : `docs/optimiseur/11-octobre-2026-15h27/01-poste-push.md`,
  `15h46/01-l-unite-systemd-de-l-hote.md`, `16h40/01-l-hote-sur-luciepc.md`.
- Mémoire : `11-octobre-2026-memoire-longue/02`, `03`, `04` (fiches classées),
  `templates/experiments/2026-10-11-fiches-classees-<n>/`.
- La nuit du 11 au 12 : le fil dans `01-les-decisions-du-matin.md` à partir
  de « 21 h – 21 h 40, nuit » ; la section « Pour Lucie » de codeparsers ; le
  ticket P1 `6cdef9585` ; la page 12 §8 du cœur (la mesure du petit masque,
  la trace par tâche : 91 % du temps dans les tâches à 32 fils) ; la page du
  magasin de libs (recherche, 07) ; la page du poste push (la priorité de la
  mesure, la forme optimiste).
- Tickets : `docs/tickets/README.md` (moteur 25, produit ≈ 35, fermés ≈ 90 au
  soir), `closed/`.

## 7. Ce que l'orchestration a appris aujourd'hui (pour elle-même)

- Dire une hypothèse comme hypothèse, et la retirer à la mesure (deux fois).
- Ne jamais faire réindexer un backend sous la session qui l'interroge.
- Chaque règle diffusée vaut une mémoire ; chaque faute dite par une session
  devient une règle de forme pour toutes.
- Les relevés toutes les 30 minutes la nuit, par `ScheduleWakeup`, le fil
  consigné dans `01-les-decisions-du-matin.md` section par section.
