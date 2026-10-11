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

## 9. La nuit, fin (0 h 50 → 2 h 30) — à lire au réveil

### Les trois priorités de la nuit : tenues

1. **Le poids des tests e2e** : fait (`c9ff68a4f`). Un binaire e2e passe de
   469 à 273 Mo ; le moteur partagé au bâti ; `cargo check --tests` ; les
   vieux hachages balayés ; le repli silencieux refusé. Disque à 50 % ici,
   78 % sur luciepc, stable toute la nuit.
2. **Indexer notre code** : fait, et prouvé. La page de l'exemple réel pour
   toi : `../codeparsers/03-impact-node-table-update.md` — « qui appelle
   `NodeTable::update` » passe de 0 à ses deux appelants réels marqués sûrs ;
   les verrous de 0 à 181 (169 sur champs mutex, 12 par les fonctions de
   verrou déclarées) ; `impact` ne montre plus que les appelants sûrs (le
   compte des autres à part, `include_by_name` pour les voir) ; `callees`,
   treizième outil ; `locks_via` déclaré au manifeste ; les types différés
   résolus à la matérialisation (le graphe ne dépend pas du paquet) ; B2a,
   B2c, B2d côté codeparsers ; appels C++ reliés entre fichiers 35,0 →
   40,3 % ; B9 Rust : les cinq trous nommés par cas minimaux (impl qui
   dédouble le type, méthodes de trait homonymes, receveur de boucle muet,
   corps de macro au scope fichier, fermetures invisibles). Tout
   `rag3db-lourd` indexé en 20 min sur luciepc (7 172 fichiers), base gardée
   (1,1 Go), qui survit au rebâti du moteur.
3. **MCP** : sur master (`69bb9f4a9`), B6 fermé (le refus nomme la
   déclaration), **B7 : la recette « une base par session » jouée**
   (`79d56229e`, la question réelle posée par MCP et sa réponse collée) ; le
   mode `--demon` ni validé ni réfuté (ticket, trois énigmes) ; la page
   « mémoire par MCP » (`faad33f8e`).

### Entré sur master cette nuit (après 0 h 50)

Mise à jour massive de vecteurs (`bfc1434e1`, deux vrais défauts : le
contrôle qui n'entrait pas comme une requête, la réinsertion qui partait du
nœud qu'elle insère ; le crochet de fin d'instruction gardé sur branche, il
régressait le ligne à ligne) ; **V2** (`11887dc21`, `CALL acquire_locks`) ;
**le lecteur affamé** (`e4198f862`, coordination à la SQLite, 0 refus sur 80
contre 34) ; le **contrôle de bâti des extensions** (`458ff7157`) ; **Write
complet** (sept formes, F) ; les **noms réservés** refusés au chargement
(liste tirée de la grammaire du moteur) ; les transactions PostgreSQL
déclarées (non prouvées vivantes : conteneur) ; le NULL en tête de liste ;
« duplicate node name: sparse » (une fixture, pas le produit ; filet des 31
fiches d'outils) ; le proto : lots 3, 4 entiers (rechargement à chaud, la
boutique jouet, routes → graphe → vue, `serve`, **la recette commence : une
colonne ajoutée à une vue en service apparaît après rechargement, sans
compilation**), lot 5 (`declare`) en cours ; I1 (verrou d'index exclusif)
en fusion vers 3 h 30. Banc de concurrence : 56 → 15 rouges connus après
I1. Le ticket « une ligne sur cent » fermé par les correctifs du banc.

### Les décisions prises à ta place, toutes renversables

| Décision | Où |
|---|---|
| tableau des sémantiques A4′ (lignes comme PostgreSQL, détachement comme Neo4j) | `coeur-cpp/07` §0 |
| forme A du nœud scripté (`nodes/<nom>.node.json` + script relatif, dossier découvert) | proto, lot 2 |
| sept formes de Write, Mark justifié, Load sous capacité | `embarquements/05` |
| npm : tout en `0.0.1-alpha.2`, même commit, épinglage exact ; profil `paquet` LTO | G |
| la montre des fichiers vit dans l'hôte, pas dans un graphe | proto, lot 3 |
| I1 avant I2 (les écrivains d'une table indexée s'attendent jusqu'à I2 mesurée) | `coeur-cpp/09` §0 |
| `impact` : les sûrs d'abord, `include_by_name` ; `callees` ; `locks_via` au manifeste | A, codeparsers |
| le lecteur affamé à la SQLite (verrou `<base>.readers`, bornes nommées, filet gardé) | hotfixs |
| nom réservé : (a) refus au chargement ; (b) les permettre en échappant tout le texte = ta question | F |
| `declare` : un commit par chemins dans le dépôt englobant, identité git de la personne, raison de l'agent en message | proto, lot 5 |
| un crochet de test dans la lib livrée (`RAG3DB_TEST_HOOKS=1` + `RAG3DB_TEST_DIE_AT`), ligne d'avertissement au démarrage | hotfixs |
| analyze explicite seulement ; estimation des relations : ticket, pas de code | hotfixs |

### Ce qui t'attend

- **Quatre OTP** pour la séance npm (linux, windows, darwin, tête, tout en
  alpha.2) — Windows 23 en cours à 2 h 20 (fsync des fichiers corrigé) ; si
  rouge, séance à trois et Windows ensuite. Rien n'est parti.
- **Relancer les deux sessions cœur C++ avec le serveur MCP** sur la base de
  `rag3db-lourd` (recette `docs/10-octobre-2026-18h00/04`).
- Le conteneur `pgvector:pg17` sur 5433 (trois tickets PostgreSQL attendent
  un témoin vivant) ; `usermod -aG docker`.
- Le ticket du mode `--demon` ; **l'indexation lancée par un outil MCP meurt
  avec le tube du client** (le travail long devrait appartenir au démon :
  page de A à venir).
- La règle du target : « effacé à chaque fusion » a rendu 101 Go ; la
  mémoire mesure 8 min de bâti à froid pour 3 min de vérification et
  propose « gardé entre les lots d'une même session, effacé à la fin de la
  session ». À toi.
- Les 21 tickets « perte / réponse fausse » côté produit : comptent-ils pour
  la stèle du moteur ou pour le produit ?
- Les « nouveaux projets émergents » de classification, pour la page
  mémoire ; sa recommandation : brancher le gabarit tel quel (A), voir si
  Claude range spontanément, puis soigner « ranger » (B).
- Le ménage des branches distantes (liste au §6, plus celles de la nuit).
- F a supprimé trois de ses branches de travail fusionnées (ses propres
  branches, rien de forcé) : à savoir.

### Règles nées de la nuit

Une session chaîne ses propres lourds ; lib et extension du même bâti,
identifiants vérifiés par le moteur ; tout `src/dataflow/` relu par la
recherche avant master ; copier un gabarit livré et en retirer ; `pgrep -x`
ne voit pas un nom de plus de 15 caractères (`pidof` + `/proc/<pid>/exe`) ;
après un rebase, on ne repousse pas la branche ; jamais un push derrière un
test non lu.

## 10. Ajout de 2 h 40 : une décision de plus pour le matin

**L'indexation portée par le démon** (page de A :
`docs/10-octobre-2026-arbre-principal/03-l-indexation-portee-par-le-demon.md`).
Deux faits lus cette nuit : `--demon` ne sauve pas une indexation
(`rag3daemon` ne tient que la base, le catalogue reste dans le serveur MCP),
et la commande en fond a le même défaut (tubes et poignée dans le
processus) ; relancer `index` après un serveur tué est refusé (reprise qui
exige le takeover, ticket ouvert). Trois formes : **A** arrêt propre entre
deux paquets dans le serveur (1-2 j) ; **B** un processus d'index détaché
(3-4 j, cession de la base pour point dur) ; **C** le démon possède le
catalogue et les serveurs MCP deviennent des clients minces (1-2 semaines).
Fait cette nuit : la ligne de vérité dans la réponse d'`index`, les journaux
de la commande en fond en fichiers. Recommandation portée à Lucie : A puis C
comme cible (c'est « installer = un démon » jusqu'au bout, et ça va avec le
paquet npm) ; B ne vaut pas son coût. Rien de plus sans son mot.

## 11. Ajout de 2 h 55

- **I1 est sur master** (`3ad09c8cc`) : la condition 1 n'a plus devant elle
  que I2 (la maintenance de l'index au commit), après la mesure du banc
  (en cours). Banc de concurrence : 9 rouges comparés sur 255.
- **Un rouge de rejeu probabiliste, perte possible** (ticket `584452d9b` et
  suite) : après le rejeu d'un COPY journalisé tué, la colonne DOUBLE des
  dernières lignes lit zéro, un cas sur deux passes complètes. Hors d'I1.
  Bissection par hotfixs (×30 / ×10+×30) ; lecture de A : une page de la
  colonne rendue au-delà de l'étendue puis réécrite — l'étendue relue après
  le rejeu ou figée à l'ouverture ? Ce qui désigne plutôt l'étendue du
  fichier (`0aed3c4b5`, A) que le lecteur affamé. À suivre au réveil si ce
  n'est pas fermé d'ici là.
- **Le proto** : lot 5 `declare` sur master (`b460e9ac8`) ; lot 6 (la page
  vivante `/_atelier`) en cours, avec trois choix pris pour toi : modèle du
  chat déclaré par `models.llm` (sans modèle la page dit pourquoi le chat
  est éteint), mermaid chargé du réseau pour le proto seulement, session de
  chat persistante hors de git. La page 08 portera les trois commandes pour
  jouer la recette au réveil.

## 12. Ajout de 3 h 20 : une corruption du COPY journalisé, le défaut du moteur

**À lire en premier au réveil.** Le rouge de rejeu de la nuit n'était ni le
lecteur affamé ni l'étendue du fichier : A a reproduit en 9 s, sans gtest,
sur master, que **le COPY journalisé** (le défaut du moteur depuis
`ff9bad960`, hier 15 h 54) **corrompt une colonne DOUBLE en mémoire, dans le
processus vivant, avant toute mort** : 3 sur 3 avec `copy_journal_threshold`
à 1 Gio, une colonne FLOAT[4] présente et 2 fils (134 à 241 lignes à 0.0,
ids ~199 550 à 199 999) ; juste avec 1 fil, juste sans colonne tableau, juste
sans seuil, juste sous `force_checkpoint_on_copy`. Le journal porte les
valeurs fausses, le rejeu les écrit, le point de reprise les garde. Chemin :
`0f4a54b2c` (étape 2 du chargement journalisé). Piste : le groupe partiel
fusionné par deux fils, relu en mémoire avec une colonne ARRAY.

Décidé cette nuit, renversable : **le cœur C++ prend le correctif en priorité
absolue** avec la sonde de A (`~/.cache/rag3weaver-notes-A/etendue/variantes.cpp`) ;
**rag3weaver pose un filet** : `force_checkpoint_on_copy=true` (l'ancien
défaut, +1 % mesuré) tant que le ticket est ouvert, avec témoin rouge sans le
filet ; hotfixs ne pousse rien sur `wal_replayer` avant le correctif. La
**condition 2 de la stèle** (« le COPY journalisé est le défaut du moteur »)
est de fait rouverte jusqu'au correctif : le cœur l'écrira à la stèle avec le
ticket. rag3weaver charge ses entités avec des vecteurs FLOAT[768] et à
plusieurs fils : le chemin fautif est le sien ; les bases indexées depuis hier
15 h 54 avec une colonne DOUBLE à côté d'un vecteur sont à vérifier (la sonde
dit comment : `MATCH … WHERE d.score <> …`).

**Ajout de 3 h 25, séance npm** : Windows est vert (23ᵉ essai, tête
786dfec13 ; l'identifiant de bâti y porte le commit) et les quatre archives
`0.0.1-alpha.2` sont prêtes — mais elles portent le COPY journalisé
défectueux. **Séance retenue** : dès que le filet de A est sur master, G
refait les trois bâtis (caches chauds, ~35 min), ajoute à l'épreuve une
entité avec un FLOAT[4] et un DOUBLE qui vérifie les valeurs écrites,
remballe, et rend les quatre commandes. Les archives de 786dfec13 sont
gardées à part, étiquetées « ne pas publier ». Quatre OTP au matin, sur les
archives refaites.

## 13. Ajout de 3 h 35 : le proto « tout déclaratif » est entier

Les six lots sont sur master (`0d8482f70`, relevé `8f0180049`), en une nuit,
chacun avec ses témoins : le moteur de script (TypeScript/JavaScript/rhai), le
nœud 100 % scripté, le rechargement à chaud, route → graphe → vue avec la
boutique jouet, `declare` (un commit par chemin avec la raison de l'agent), et
**la page vivante `/_atelier`** (chat à gauche, la page produite, le graphe
mermaid de l'outil, le journal des rechargements, un flux d'événements
serveur), essayée sur luciepc avec le binaire réel. **La recette pour Lucie
est en trois commandes** : `docs/10-octobre-2026-18h20/everything-declarative/08-lot-6-la-page-vivante.md`
§1 (bâtir avec `openai-llm` sous `poste lourd` ; copier le jouet dans
`~/boutique` et lui déclarer Gemini/Vertex ; servir avec les identifiants dans
l'environnement seulement ; ouvrir `/_atelier` ; dire « ajoute une colonne
prix à la liste »). Le tour avec un vrai modèle n'a pas été joué cette nuit
(pas d'accès à `.vault`, et c'est son témoin) ; le chemin modèle → declare →
page est prouvé avec un modèle enregistré. Reste ouvert : la montre branchée
sur la porte de la mémoire à sa fusion, le retour en arrière d'une
déclaration, mermaid embarqué, plusieurs sessions.

**Ajout de 3 h 45** : le filet est sur master (`6d7f2a811`) — toute connexion
rag3weaver pose `force_checkpoint_on_copy=true` tant que le ticket
`2026-10-11-copy-journalise-lit-zero-dans-un-double-du-groupe-partiel.md`
est ouvert ; témoin rouge sans le filet (198 lignes fausses à la troisième
passe de 200 000 lignes, 2 fils, FLOAT[4] + DOUBLE), vert avec. Fait nouveau
pour le cœur : un CHECKPOINT dans le même processus rend les valeurs justes —
la mémoire est saine, c'est le balayage du groupe partiel qui lit 0, et il
faut plus d'un groupe. Les bases écrites par rag3weaver depuis hier 15 h 54
n'ont donc pas de valeurs fausses au repos si un point de reprise a suivi
(ce que l'indexation fait) ; à vérifier quand même sur une base qui compte.
G refait les quatre archives alpha.2 sur ce master.

## 14. Ajout de 4 h : la mémoire longue est branchée par MCP

Ordre A joué par la session mémoire (`ca9806d06`) : manifeste à
`~/.config/rag3weaver/memoire-longue/`, base à
`~/.local/share/rag3weaver/memoire-longue.rag3db`, binaire copié hors du
target dans `~/.local/bin/rag3weaver-backend`. **La page pour Lucie** :
`docs/10-octobre-2026-18h00/07-l-ordre-a-la-memoire-branchee.md`, recette en
trois lignes dont la troisième, `claude mcp add`, **prête et non jouée** (elle
écrit dans sa configuration). Mesuré : un processus écrit sujet + mémoire +
genre + référence, un autre les retrouve. Deux défauts du lot précédent
trouvés en jouant (l'arrêt n'arrêtait pas les réactions ; le serveur ne disait
pas ses réactions montées) et corrigés. Un troisième **non réglé, et dit** :
aucun outil n'écrit `ANCHORED_TO`, la réaction `review_anchored` tourne à
vide ; l'outil d'ancrage « en six lignes » que j'avais accepté sur parole ne
tient pas — `RelationBatchNode` refuse de lier deux entités gérées (cycle de
vie, dates), et aucun nœud ne relie aujourd'hui deux entités gérées par leur
identité. La mémoire l'a mesuré avant d'écrire, l'a retiré, et le gabarit a
sept outils. Deux tickets pour le matin, avec leur coût : un `LinkRecordsNode`
générique (une signature nouvelle : à Lucie), puis le huitième refus de la
section `reactions` (une réaction qui remonte une relation qu'aucun outil
n'écrit), après le nœud, sinon il refuserait le gabarit `memory` livré. **La question de
l'ordre A — Claude range-t-il spontanément ? — se tranche au matin par une
session vierge, chez Lucie, après son `claude mcp add`** ; la session qui a
écrit l'outil ne peut pas y répondre.
