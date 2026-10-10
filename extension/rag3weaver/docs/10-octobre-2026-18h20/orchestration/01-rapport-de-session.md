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
