# Orchestration — rapport de session, 10 octobre 2026 au soir

Écrit à 18 h 20, contexte presque plein. Pour la session d'orchestration qui
reprend, et pour Lucie. Le relevé de connaissances est à côté
(`02-knowledge-dump.md`) ; le plan des chantiers est
`../../8-octobre-2026-16h29/orchestration/01-plan-de-reprise.md` (§0 : l'état
du soir, la branche et la tête de chacun ; §2 : les chantiers A à H).

## 1. Les visions, en une page

Tout est dans `extension/rag3weaver/visions/`, datées dans leur nom ;
`00-vision-generale.md` les réunit. Ce qui compte pour orchestrer :

- **L'idée** : tout vit dans un seul système et tout s'y déclare — la base
  (graphe, mots, vecteurs), les traitements (des graphes à ports typés), les
  outils (tirés des schémas de nœuds), les mémoires, les vues, les
  intégrations. Une chose déclarée de plus, et le système sait la faire.
- **Le cap** (§0) : *tu parles à ton code ou à ton site, et il change sous tes
  yeux* ; l'écran montre ce que l'agent fait (les graphes qui se forment et
  tournent), tout se clique. Westworld, Star Trek.
- **Ce qui se vend** : un seul abonnement (~20 €/mois) ; l'atelier local sans
  IDE ; le backend tout-en-un qui porte son atelier ; les embarquements
  servis. Le modèle de langage n'est pas vendu.
- **Les marches** (§7) : 0 la stèle du moteur (en cours) → 1 les fiches de
  contexte → 2 le backend déclaré (un contrôleur est un graphe, rechargement à
  chaud) → 3 les vues déclarées → 4 parler à une page → 5 les vitrines →
  6 à plusieurs. Chaque marche pose une pièce durable ; rien qui ne sert qu'à
  faire beau.
- **Plusieurs produits, un seul atelier** (`2026-10-09-…`) : le même atelier
  avec un domaine différent (decks, code, DXF, Blender) ; un domaine = un
  service + une déclaration + un rendu. Les ajouts du 10 au soir : le
  **magicien du chaos** (§6 : une fois un modèle déclaré, un agent dont le
  domaine est le système, qui monte un produit par déclaration et donne le
  lien de l'agent dédié), et **déclaré n'est pas exposé** (§7 :
  `exposure: <expression sur des clés>` par déclaration et par cellule,
  tenue au chargement).
- **La preuve de l'agent** (§6 de la générale) : l'agentique dans les graphes
  a coûté une déclaration et quelques nœuds ; c'est l'argument pour les
  marches 2 et 3. Réserve : jugé sur ce dépôt, par nous.
- **Installer puis régler** (page du paquet npm, § 5 bis) : installer = un
  démon sans question ; régler = un formulaire des API disponibles (et sa
  CLI), pas d'agent avant un modèle ; puis le magicien.
- **Décisions de Lucie qui fixent le cap** : rag3db reste le moteur, mais
  rag3weaver parle en dialectes (l'IR, jamais de Cypher au-dessus) ; le
  navigateur (WASM) est abandonné pour l'instant (luciole plus tard) ; les
  modèles de décision et GLiNER mis de côté ; les PR tracel-ai en pause ;
  « un jour rag3db en MIT à côté de rag3weaver » (intention, pas décision).

## 2. Où on en est, au 10 octobre 18 h 20

### La stèle du moteur (`docs/4-octobre-2026-16h57/01-la-stele-du-moteur.md`)

| Condition | État |
|---|---|
| 4 — plus de défaut qui corrompt ou perd | fermés depuis le 4 : clé perdue, annulation de COPY, DROP de colonne, statistiques, fuite de pages (`0aed3c4b5`, version de stockage 40). Restent, à l'index vectoriel : l'élagage (k2 tient le critère, un rouge nouveau au banc en attribution) et la mise à jour massive (page acceptée, pas codée). |
| 2 — chargement en masse journalisé | **fermée** le 10 à 15 h 54 (`ff9bad960`) : le COPY journalisé est le défaut du moteur ; série de confirmation au commit. |
| 1 — les verrous | V1 sur master ; A3′ en cours chez le cœur C++ (C1 ×6, SameKey ×2 verts ; une famine des fils de l'ordonnanceur à confirmer par les piles) ; puis A4′ (relation par ses extrémités, comme Neo4j), V2 (`CALL acquire_locks`, à lui), index au commit en dernier. |
| 3 — écritures parallèles | pas commencée ; 57 rouges du banc sont ses marches ; après la 1. |

### Le produit

- **Sur master** : les nouveaux défauts (plein texte en fichiers, transaction
  par paquet, 2 048 × 1, règle des dialectes) ; la crate `rag3weaver-ir`
  (`Value`, filtres, portée) ; la porte unique du Cypher ; `Hop` (trois sites)
  et `Count` ; les capacités déclarées des dialectes ; le filet des gabarits ;
  la section `reactions` du manifeste ; `tree-sitter-scss` en copie locale ;
  `landlock` Linux seul ; `extension/fts` et `fuzzy-fst` retirés.
- **Le paquet npm** : `rag3weaver@0.0.1-alpha.1` + `rag3weaver-linux-x64-gnu`
  sous `next`, compte `luciformresearch` ; démo verte depuis un dossier vide ;
  Windows (MSVC pur) et macOS arm64 se lient sur les runners ; `paquet-npm` ne
  fusionne pas avant l'embarqueur absent de A.
- **En branches** : `defauts-bascules-2` fusionnée ; `embarqueur-absent`
  (verte, attend la batterie du basculement) ; `commande-en-fond-2` (attend
  l'étape 2 de C) ; `execution-asynchrone(-2)` (étape 1 poussée, étape 2 en
  cours : trois pièces pour A, réacteur en tâche, postgres, chat ; la boucle
  d'agent reste un fil) ; `elagage-hnsw-2` (k2) ; `anthropic-llm` (client
  Claude compilé et vert, session E non relancée).
- **Chantier H** (mémoire) : rag3weaver en serveur MCP, page écrite, code en
  cours ; mémoire long terme pour Claude par le même serveur ; portée par
  cellule ; pour une mémoire partagée, connexion sur `rag3daemon`.

### Ce qui attend Lucie

- le jeton crates.io dans `.vault/cargo.env` (réserver le nom) ; `sudo usermod
  -aG docker lucied` (Docker marche sur luciepc, pas encore ici) ; le ménage
  des branches distantes (`etendue-du-fichier`, `defauts-bascules`,
  `relcopy-sous-refus`, `ir-hop` déjà retirée) ; la session E à rouvrir si
  elle veut la comparaison Gemini/Claude ; les six suites à zéro test
  (`openai-llm`, Vertex) ; macOS x64 (oui ou non) ; le README npm en anglais
  pour l'alpha suivante.

## 3. Les sessions (noms du 10 au soir — ils changent à chaque relance)

A arbre principal = `rag3db-73` ; B cœur C++ (stèle) = `rag3db-91` ; B-bis
cœur C++ hotfixs/tickets = `rag3db-50` (seconde session, créée le 10 à 13 h 30 ;
B tient `src/transaction/` et `src/storage/`) ; C recherche = `rag3db-97` ;
D banc = `rag3db-10` ; F embarquements = `rag3db-6f` ; G optimiseur = `rag3db-90`
(renommée `optimiseur` par Lucie, puis relancée) ; codeparsers = `rag3db-ab`
(lot fini) ; mémoire = `rag3db-96` (chantier H). Moi = `rag3db-c7`.
Après toute relance : « qui es-tu ? » à toutes, puis la table à chacune.

## 4. Méthode : ce que la journée a corrigé

- **Le verrou** : un lourd n'attend plus que 10 min à la porte ; « mesure »
  seulement pour ce qui mesure ; `timeout` devant tout test sous le verrou
  (un test figé en `futex_wait` a tenu le poste à 0 % de CPU — j'ai d'abord
  accusé la mesure du banc, à tort : chercher le porteur par `lsof`, pas la
  charge) ; on ne tue jamais par nom de binaire (`pidof concurrence_test`).
- **Mesurer** : prouver que l'artefact mesuré vient du code (deux colonnes
  HNSW sur la même extension périmée, cinq jours) et que le binaire de test
  n'est pas plus vieux que le moteur ; une hypothèse se rougit par exécution
  avant d'être corrigée (quatre témoins verts à 60 000 lignes, le groupe
  plein est à 131 072) ; une sonde qui fait tomber sa propre hypothèse est
  le travail bien fait (la double insertion, les îlots de doublons).
- **La preuve dans le code d'abord** (Lucie) : lignes à l'appui, la mesure
  confirme.
- **Ne pas pousser sur un critère qu'on sait non tenu**, même quand tout le
  reste va mieux (keep par le plus proche : deux rouges cachés par la liste).
- **luciepc** : un worktree par chantier, jamais un checkout dans un
  worktree partagé (le verrou protège la mémoire, pas l'arbre) ; les cinq
  variables qui évitent de rebâtir le C++ ; les mesures restent ici.
- **npm** : OTP demandé au moment de publier ; « staged publishing » ;
  `--prefer-online`.
- **Mes erreurs** : le filet 1 décrit comme ce qui arrêtait l'indexation ;
  « lourd passe sans attendre » ; la mesure du banc accusée ; une borne sur
  les doublons demandée avant la preuve ; « sans GPU » pour luciepc ;
  `record.rs` « à retirer » (il est la seule pièce qui écrit une exécution).

## 5. Comment reprendre

1. Lire ce rapport, puis le plan (`8-octobre-2026-16h29/orchestration/01`, §0
   et §2), puis la stèle et `docs/journal-des-chantiers.md`.
2. `git log origin/master --since=<dernière lecture>` : tout se livre sur
   master en avance rapide, jamais de force.
3. `ListAgents`, « qui es-tu ? » à chaque session, redistribuer la table,
   relancer chacune sur sa ligne du plan.
4. Les docs de l'orchestration passent par un worktree détaché de
   `origin/master` (sparse), commit par chemins, push, retrait ; puis
   `git merge --ff-only` dans l'arbre principal s'il est propre, pour que
   l'éditeur de Lucie voie les fichiers.

## 6. La soirée du 10 octobre (18 h 30 → 21 h), ajout de 21 h

Écrit après deux compactions de contexte (relecture des docs, « où en
es-tu ? » aux neuf sessions, tous les noms inchangés ; `lucied-68` est la
session d'assistance système de Lucie, hors rag3db).

### Le disque plein, et le nettoyage

À 18 h 45, `/home` (950 Go) avait 2,2 Go libres. Ce n'étaient pas les dépôts
(le `.git` fait 1,4 Go) mais les bâtis : chaque session avait un target cargo
de 90 à 180 Go (`target-async` 181, `memoire` 152, `wt-defauts` 86, l'arbre
principal 98), treize worktrees avec leurs `build/` (≈ 255 Go), l'amont
tracel-ai 44 Go, les bases d'index MTG 30 Go. Recette, avec le « go » de
Lucie (« rien qu'on ne puisse regen ») : chaque session a commité et poussé,
vérification des processus par `/proc/<pid>/cwd` (jamais par nom),
`git worktree remove --force` de tout sauf l'arbre principal, `rm` des
targets. De 100 % à 46 % (518 Go libres) en une minute. Gardé :
`rag3db/build` (la lib lecteurs-csv de 17 h 27, référence commune de la
soirée), les modèles, les notes du banc et du cœur, `poste`, `envoi-tracel`,
les scripts d'F, `paquet-npm-garde/` (archives alpha.1, binaires Windows),
les exports MTG (jsonl/json/sqlite, 360 Mo — « garde que les trucs exportés,
pas d'index »). Aucune branche locale touchée. Règle posée (mémoire
`un-target-cargo-par-session`) : un seul target par session,
`CARGO_INCREMENTAL=0` pour les batteries, le target gardé entre les lots et
effacé au-delà de 60 Go ou à la fin du chantier (la première version, « cargo
clean à chaque fusion », coûtait 15-20 min par lot : corrigée sur le retour
d'F) ; mesurer le disque chaque semaine. Perdu et rejoué : la batterie d'A
(61 suites avant l'arrêt, 13 après), celle de C, la chaîne A3′ de B.

Sur luciepc : le disque était à 88 % (Steam, 1,6 To) ; Lucie a désinstallé
dix jeux (384 Go rendus, liste dans `~/jeux-steam-luciepc-2026-10-10.md`,
hors dépôt). **La lib de `rag3db-lourd` datait de 14 h 20, avant la
bascule** : deux lots d'F y avaient été validés contre le moteur d'avant
(la mémoire l'a vu) ; G l'a rebâtie sur f1b5e7407 en 42 s, artefacts
prouvés. Règle : prouver la date de `librag3db.so` contre la tête de master
avant toute suite là-bas.

### Le cœur C++ travaille sur luciepc (décision de Lucie, 20 h)

Pour que ses tests de verrous et de famine ne soient pas faussés par la
charge des batteries d'ici (140 à 522 ce soir). Les deux sessions C++ y ont
leur worktree et leur `build/moteur` ; leurs mesures s'y font pour leurs deux
colonnes, dites non comparables au poste principal. Seule exception à « les
mesures restent ici ».

### Ce qui est entré sur master ce soir (acadcb16b → 03a399d72)

- **Élagage HNSW** (banc, `9a2818df9`) : règle classique, places libres
  reprises par le plus lointain, copies bornées ; TwentyRows en probabiliste
  (1 perte sur 400 avec, 0 sur 400 sans, non significatif ; mécanisme non
  établi, écrit au ticket) ; un rouge de l'index dit désormais de lui-même ce
  qu'il a perdu (`2d8ad770a`). Un index existant garde son ancien graphe : le
  recréer (README). Sur la condition 4, reste la mise à jour massive de
  vecteurs ; l'insertion double (une ligne créée puis mise à jour dans la
  même transaction entre deux fois dans le graphe, `NodeTable::update`) est
  vérifiée à la source, correctif en cours chez le banc, relecture du cœur.
- **A3′** (cœur, `21b6cb370`, `79b7be75f`, `9a57a17b2`) : clé primaire
  verrouillée à l'insertion, unicité contre le dernier état validé, fil de
  remplacement de l'ordonnanceur (famine : 59,8 s → 216 ms), journal à
  doublon gardé ; banc 59 → 56 rouges connus ; coût sur luciepc +0,65 µs par
  ligne à l'insertion, +0,23 µs à la validation, mode éteint = chemin d'avant
  plus un booléen. Suite : A4′, V2, index au commit.
- **Embarqueur absent** (A, `8e85553e4` → `8cd96e733`) : sans service, plein
  texte seul et vecteurs en dette ; `Level::NotDeclared` ; `count_rows`,
  `fetch_related_in`. La batterie du basculement : 74 suites vertes, aucun
  rouge (journal des chantiers, `18f12890d`) — confirmation produit de la
  condition 2.
- **Hop complet** (F, `2bef57c6d`, `f1b5e7407`, `e1f490e3e`) : sans table,
  étiquette ou nœud entier, borné ; tous les sites en forme de saut passent
  par lui. Select est écrit sur `ir-select`, en attente.
- Docs et tickets : tampon de 256 Mio plein (`63f0413ca`), démon instable
  sous charge (`80b7da028`), analyze non transactionnel, ouverture sans une
  extension dont une table dépend, unknown entity: File, validation à moitié
  appliquée.

### En vol à 21 h

- **C** (`execution-asynchrone-2`) : batterie rendue, tout vert sauf trois
  rouges prouvés de master (isolés au commit de base) ; mesure avant/après en
  cours ; fusion ensuite. Les trois rouges : `test_backend_persistence` et
  `sparse` (scripts Python hors `run_e2e.sh`, tampon 256 Mio, suspect : les
  défauts basculés) → A ; `test_backend_code` « 2 directement » → F a
  prouvé par bissection que ce n'est pas Hop, le 2 paraît juste (le scope du
  fichier par l'import, `main` par l'appel), vérification sur 27eeb6a7f.
- **Mémoire** (`memoire-longue-6`) : batterie 472 verts / 75 suites sur
  3416feddc ; master a pris 21 commits pendant ; arbitrage : rejeu ciblé (lib,
  deux suites MCP, réacteur, arrêt brutal) puis fusion, pas de seconde
  batterie. MCP : douze outils du gabarit `code` sur stdio, recette
  `templates/mcp/README.md` ; pas de `.mcp.json` à la racine (réglage
  commun) ; `MustReopen` → le serveur sort après l'avoir dit.
- **A** : témoin de la fuite, puis les deux rouges Python, puis le ticket
  « unknown entity: File » (remède à la surface de code,
  `code_tools.rs:425-430`) ; rebâti de la lib commune (élagage + A3′) quand C
  a fini sa mesure — pas de bâti à part, l'extension vecteur s'écrit à un
  chemin fixe.
- **B-bis** (`statistiques-2`) : bâti sur luciepc, puis témoin du ROLLBACK,
  liste, mesure d'analyze, relecture de B, avance rapide. Décision :
  analyze remplace les statistiques à l'exécution (comme `reltuples` dans
  PostgreSQL, source vérifiée), accepté avec témoin et ticket.
- **G** : macOS arm64 vert de bout en bout (essai 4 : `link-dead-code` +
  `strip -x`, 81 Mo ; mémoire vive lue par système, `390ea9510`) ; Windows
  essai 17 en cours (essai 16 : tout se lie, rouge sur un chemin `D:\a\…`
  dans du Cypher → `cypher_path_literal`, `f4f2cd854`). Prêt sous
  `paquet-npm-garde/publier/` : `rag3weaver-0.0.1-alpha.2.tgz` (README en
  anglais, trois sous-paquets optionnels épinglés alpha.1) et
  `rag3weaver-darwin-arm64-0.0.1-alpha.1.tgz`. Séance quand Windows est
  vert : trois `npm publish` (windows, darwin, tête), trois OTP de Lucie.
- **Banc** : correctif de l'insertion double (témoin rouge d'abord,
  relecture du cœur), puis la mesure de l'union par lots, puis la fin
  d'instruction. Une faute dite par lui : `--force-with-lease` sur sa branche
  joint au push de master (rien de perdu, ancienne tête sous
  `elagage-hnsw-2-avant-rebase`) ; règle de forme ajoutée : jamais deux push
  dans une commande, jamais un flag de force dans une commande qui touche
  master.

### Ce qui attend Lucie (mis à jour)

Le conteneur `pgvector:pg17` sur 5433 ou `usermod -aG docker lucied` (C et F
jouent e2e_postgres dès qu'il existe) ; trois OTP à la séance npm ; macOS
x64 oui ou non ; le jeton crates.io ; le ménage des branches distantes
(`etendue-du-fichier`, `defauts-bascules`, `relcopy-sous-refus`,
`elagage-hnsw-2`, `elagage-hnsw-2-avant-rebase`, `paquet-npm-absent`,
`statistiques`, `memoire-longue-3/-4/-5`) ; supprimer `.vault/npm.env` ;
éventuel `docker system prune` sur luciepc (98 Go réclamables).

## 7. La nuit du 10 au 11 octobre (21 h → 23 h 15), ajout de 23 h 16

Lucie dort depuis 23 h 30 ; consignes : « hésite pas à te relancer, fais les
choix les plus génériques à chaque fois, pas forcément les plus simples ;
c'est mécanique : les tests e2e, savoir indexer le code, et MCP, dans cet
ordre » ; « dis aux sessions d'éviter de lancer les suites complètes à chaque
fois » ; « effacer ensuite après fusion les target et worktree, à chaque
fois ». L'orchestration se relance toutes les 30 minutes.

### Le disque, deuxième fois (23 h, 92 %)

Cause mesurée, différente du matin : un binaire e2e pesait 1,35 Go (0,88 Go
de `.debug_*`) parce que **le moteur C++ y était lié en statique** dès que
`RAG3DB_SHARED` + `RAG3DB_LIBRARY_DIR` + `RAG3DB_INCLUDE_DIR` n'étaient posés
qu'à l'exécution et pas au bâti ; `cargo test --tests --no-run` sur tout le
dépôt bâtit 171 binaires (240 Go) ; et chaque jeu de features refait un
binaire (87 e2e sur 90 en plusieurs hachages). `target-C` faisait 307 Go
après trois heures. Lot « poids des tests » (A, `c9ff68a4f`) : un binaire
passe de 469 à 273 Mo (`[profile.test|dev.package."*"] debug = false`,
lignes gardées), `run_e2e.sh` pose les trois variables, garde un seul jeu de
features, balaie les vieux hachages à la sortie, refuse le bâti implicite du
moteur et un worktree sans `RAG3DB_BUILD` ; la page des défauts dit
« `cargo check --tests`, jamais `--tests --no-run` global ». Règle de Lucie :
après chaque fusion, le target et le worktree s'effacent. Disque à 53 %.

### Entré sur master (acadcb16b → 70a1b97ab, 26 commits depuis 23 h)

- **C fusionné** (`afdc6923e`) : l'exécution asynchrone des graphes, la
  portée du run, le réacteur en tâche, postgres avec son runtime, le chat
  sur le bus ; mesure dans le bruit (441,5 s contre 445-448 s, MRR au
  millième). Puis la connexion PostgreSQL épinglée (branche, en fusion).
- **F** : Select avec condition compilée, Hop filtré et ordonné, le journal
  de conversation par Hop, les nœuds de recherche sans Cypher direct,
  `query.rs` retiré (décision de Lucie), **Write::Upsert et Link**
  (`70a1b97ab`) ; page Write/Tx (sept formes, Tx sur la connexion) ; ticket :
  sur PostgreSQL, défaire un lien ne trouvait rien.
- **Proto « tout déclaratif »** (session « everything declarative », ouverte
  à 22 h, briefée de zéro) : lot 1, le moteur de script générique
  (TypeScript par retrait des types, JavaScript par QuickJS, rhai derrière
  la même interface ; 18 témoins) ; lot 2, le nœud entièrement scripté
  déclaré sous un nom (forme A : `nodes/<nom>.node.json` + script relatif,
  dossier découvert) et les noms possédés qui ferment deux fuites en service ;
  lot 3 (rechargement à chaud) en cours. Poids du lot 1 : +8,5 Mo, c'est
  l'effaceur de types ; profil `paquet` avec LTO complète chez G (61 contre
  89 Mo).
- **Codeparsers** (priorité maximale de Lucie, « indexer nos propres
  dépôts ») : B4 le compteur de non-résolues (rendait 0 ; 89 % d'appels non
  reliés en vrai), B5 le banc de couverture à corpus égal (rag3db src 11,3 /
  35,0 % d'appels reliés, rag3weaver 14,5 / 27,0 %), le rapport des trous
  classés par coût (premier coût partout : la méthode sur un receveur non
  typé), le lot préprocesseur (imports reliés 10 → 34 %), **B2a** les
  receveurs typés (pointeurs intelligents comme Box/Arc, gabarits qui disent
  leur type ; appelants de `NodeTable::update` 0 → 1). B2b (types différés
  résolus à la matérialisation, diff `code.rs` pour A) en cours ; B3 LOCKS
  ensuite. Page des neuf bloquants :
  `04-indexer-nos-propres-depots.md`.
- **Cœur C++** : IGNORE_ERRORS fermé (`e0fad1325`, perte de la condition 4 :
  un doublon supprimait une ligne innocente) ; analyze non transactionnel
  accepté (comme `reltuples`), pas de déclenchement automatique (sonde :
  cardinalité déjà exacte, distincts faux par l'HyperLogLog) ; tickets
  complétés (estimation des relations, RTree de geo inutilisable) ; B8
  (tampon de 256 Mio) en départage ici contre la lib de 17 h 27 puis la lib
  rebâtie. A4′ : code fait, 30/33 témoins, témoins du banc réécrits
  (`banc-a4-temoins-2`), fusion en attente.
- **Banc** : insertion double fermée (`a162b43e3`), témoins A4′, mesure de
  l'union (1,6 nœud par ligne, 38 fois moins que ligne par ligne, référence
  19,5-24,3 s), fin d'instruction en cours avec le critère « a perdu une
  arête entrante ».
- **Tickets** : 48 fermés déplacés dans `docs/tickets/closed/`, index en
  deux tables (décision de Lucie).

### Décisions prises cette nuit, renversables par Lucie

Forme A du nœud scripté ; sept formes de Write (Mark justifié contre
Update), Load dans Write sous capacité ; analyze explicite seulement ;
estimation des relations : ticket, pas de code ; geo après B8 ; profil
`paquet` LTO ; pas de `.mcp.json` à la racine ; `MustReopen` → le serveur
MCP sort après l'avoir dit ; fin d'instruction HNSW avec le critère exact
dès le départ.

### Enquête close : l'extension de 21 h 26

`libvector.rag3db_extension` de l'arbre principal réécrite à 21:26:03
(912 048 o, md5 `9ba45dd1…`) sans que la lib (17 h 27) change ; banc, G,
hotfixs, mémoire hors de cause par leurs journaux ; la taille ne désigne
aucun bâti. Conséquence : toute passe d'ici depuis 21 h 26 a tourné sur un
mélange (sans effet sur la mesure de C : même artefact aux quatre passes).
Règles : un worktree qui bâtit n'a jamais de lien `extension/vector/build`
vers l'arbre principal ; le remède générique est le ticket du 5 octobre
« extension chargée sans contrôle de bâti » (identifiant de bâti vérifié à
`LOAD EXTENSION`), confié à hotfixs après B8. A rebâtit la lib et
l'extension d'ici ensemble après les signes de hotfixs et de la mémoire.

### Pour le matin

Les trois OTP de la séance npm quand Windows 20 est vert (profil `paquet`,
bâti à froid de 85 min lancé à 23 h) ; les « nouveaux projets émergents » de
classification pour la page mémoire ; le conteneur pgvector ; le ménage des
branches distantes (liste au §6, plus `statistiques-3`, `-avant-rebase`,
`memoire-longue-6/-8`, `banc-a4-temoins`).

## 8. La nuit, suite (23 h 15 → 0 h 50), ajout de 0 h 50

### Entré sur master (a78481462 → 167eb555f)

- **A4′** (cœur, `9904fefc0..67ff3816b`, docs `ad87cf454`) : la ligne en
  exclusif à la mise à jour et à la suppression, les extrémités d'une
  relation (partagé pour créer, exclusif pour supprimer ou mettre à jour),
  l'erreur de sérialisation après l'attente, la vue du dernier état validé
  pour les relations d'un nœud supprimé ; les écritures internes de l'index
  vectoriel ne prennent pas de verrou d'extrémités. Banc de concurrence :
  **56 → 21 rouges connus** (15 comparés / 238 verts). Tableau des
  sémantiques en tête de la page 07 (lignes comme PostgreSQL, détachement
  comme Neo4j) — **pour Lucie au matin**. V2 (`CALL acquire_locks`) codée,
  page `coeur-cpp/08`, fusion vers 1 h 15 ; puis l'index au commit.
- **MCP** (mémoire, `69bb9f4a9`) : le serveur, la sous-commande `mcp` avec
  ses deux modes et `--keys`, le témoin sur stdio, la recette
  `templates/mcp/`. B7 : le mode `--demon` **ni validé ni réfuté** (trois
  énigmes : `tables : []`, un MATCH qui pend après le départ des écrivains,
  un écart d'adresse) → ticket pour le matin ; la recette « une base par
  session » s'écrit et se joue cette nuit.
- **Commande en fond** (A, `5948276eb`) : run en fond, tail, journaux en
  anneau, groupe de processus tué ; chantier A entier.
- **Connexion PostgreSQL épinglée** (C, `da740cbb4`) ; **transactions par la
  connexion** (A pour F, `167eb555f`) ; F peut ouvrir `transactions: true`.
- **Proto** : lot 3 le rechargement à chaud (`f07b6a4af` : version prise au
  départ de chaque appel, refus nommé de ce qui est fixé à l'ouverture), le
  rechargement en deux temps pour la mémoire, le trou du nom déclaré au
  point de reprise fermé ; **la montre sur les fichiers vit dans l'hôte, pas
  dans un graphe** (sinon la frontière d'exposition serait rechargeable de
  l'intérieur) — écart à la page, bien argumenté, **pour Lucie**. Lot 4
  (jouet, `nodes/`, route → graphe → vue, `serve`) en cours.
- **Codeparsers → rag3weaver** : B2b (types différés résolus à la
  matérialisation, `0d058d9eb`), B3 LOCKS (`2a29c10c6` : le mutex a une
  identité `Classe::champ`, 169 verrous relevés sur le moteur) ; B2c (chaînes
  de champs), « verrous sur le chemin » dans impact, borne à la source :
  proposés, en test chez A. **Mesure réelle** (tout `rag3db/src` indexé,
  167 s) : appelants de `NodeTable::update` 0 → 2 (tous les sites réels),
  marqués « type ». Page de l'exemple réel pour Lucie au matin.
- **Cœur hotfixs** : B8 fermé (non reproduit sur master, deux montages,
  32 fils écartés) ; **contrôle de bâti des extensions** (`458ff7157`) :
  identifiant généré à chaque bâti, vérifié à `LOAD EXTENSION`, refus nommé,
  toute extension d'avant refusée ; libs des deux postes rebâties. Suite :
  le lecteur affamé par les points de reprise d'un autre processus → forme
  (2), coordination à la SQLite par un verrou `<base>.readers` (flock /
  LockFileEx), bornes nommées des deux côtés, filet gardé — **alignement sur
  SQLite, pour Lucie**.
- **Banc** : la mise à jour massive — les rouges venaient de **deux vrais
  défauts** (le contrôle de joignabilité entrait par la couche basse ; la
  réinsertion partait du nœud qu'elle insère) ; le crochet de fin
  d'instruction régresse le ligne à ligne ×7 à ×13 → **option C** (master +
  les deux correctifs, 25 lignes) retenue si elle tient, le crochet gardé
  sur branche avec ses chiffres. Export des vrais vecteurs à régénérer par A
  (effacé par le nettoyage).
- **npm** : fsync du dossier sans objet sous Windows (cause du rouge FTS),
  profil `paquet` LTO (61 contre 89 Mo), **tout en 0.0.1-alpha.2** (tête et
  trois sous-paquets, même commit, épinglage exact) ; les archives se
  refont sur la tête avec le contrôle de bâti. Rien de publié ; quatre OTP
  au matin.

### Règles ajoutées

Une session chaîne ses propres travaux lourds (le verrou partagé laisse
passer ensemble deux lourds d'une même session). Lib et extension du même
bâti, prouvées ensemble. Tout `src/dataflow/` relu par la recherche avant
master. Copier un gabarit livré et en retirer, plutôt que composer un
manifeste de zéro (mémoire, sept interfaces supposées).

### Pour le matin (ajouts)

Le ticket du mode `--demon` ; le lecteur qui pend sur le démon après le
départ des écrivains (défaut possible) ; la décision sur l'export vivant
pour le critère « 0 introuvable ».
