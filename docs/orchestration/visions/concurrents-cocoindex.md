# CocoIndex, un voisin à regarder (13 novembre 2026)

Lucie signale `github.com/cocoindex-io/cocoindex` (11,9 k étoiles, 930 forks, Apache 2.0, cœur Rust,
SDK Python). Lecture du README du 13 novembre, pour s'en inspirer sur deux plans : la présentation et
ce qu'ils proposent.

## Ce qu'ils font

- Un moteur **déclaratif et incrémental** : on déclare ce que la cible doit contenir (« Target = F(Source) »),
  le moteur maintient la cible à jour en ne retraitant que le delta ; un cache par empreinte de l'entrée
  et du code (`memo=True`).
- Sources : dépôts de code, notes, mail, Slack, PDF, vidéo, bases, files ; cibles : bases relationnelles,
  vectorielles, graphes, files, feature stores. C'est un **ETL pour le contexte des agents**, pas une base.
- **CocoIndex-code** : un serveur MCP pour agents de code, index sémantique du code à partir de l'AST,
  mis à jour à chaque commit, Python/TypeScript/Rust/Go, avec « call graphs » et « blast radius »
  revendiqués ; une « skill » à installer dans l'agent pour qu'il écrive du CocoIndex correct ;
  `AGENTS.md` et `CLAUDE.md` à la racine du dépôt.
- Provenance revendiquée (chaque cible remonte à l'octet source), gestion des erreurs (reprises, files
  de rebut, isolation), chiffres d'accroche (« 70 % de jetons en moins », « 10× moins cher ») sans
  méthode ni banc publiés. Pas de multi-worktree, pas de graphe de code documenté, pas de version
  affichée.

## Ce qu'on a et qu'ils n'ont pas

- **Le moteur est à nous** : rag3db est une base de graphe (fork de Kuzu) avec plein texte, dense et
  creux dedans ; chez eux, la cible est une base tierce.
- **Un index par dépôt, des vues par worktree et par branche** (la pyramide), la provenance par
  occurrence : leur « à jour à chaque commit » est un seul courant.
- **L'atelier** : un agent qui lit, écrit, lance des commandes derrière une porte d'accord, avec un
  journal des décisions pour un décideur local ; **la mémoire longue** avec ses fiches, son archiviste,
  son juge local mesuré ; **les hooks** qui apportent la mémoire aux sessions sans qu'elles y pensent.
- **Des bancs publiés avec leur méthode** (juge local 0,76 / AUC 0,92, 440 paires vérifiées ; la précision
  du rappel ; la charge d'ingestion), là où eux affichent des pourcentages sans banc.
- Des tests de robustesse du moteur (arrêt brutal, vingt écrivains, points de reprise) et des tickets
  publics pour chaque défaut connu.

## Ce qu'on peut leur emprunter

**Pour la présentation** (le README de rag3weaver, puis celui de rag3db) :
1. Une accroche d'une phrase en tête (« Your agents deserve fresh context »), puis les boutons et badges
   (étoiles, paquets npm/PyPI/crates, licence, CI).
2. **Un seul exemple de code** dans « Get started », qui marche en copiant : chez nous, déclarer un
   dépôt, l'indexer, poser une question par le MCP.
3. **Huit cartes de démos** avec une image chacune : index de code par worktree, mémoire longue d'une
   session, l'atelier qui édite derrière une carte d'accord, le juge local, l'IDE, la trace d'un index,
   un dépôt en PostgreSQL, lucivy seul.
4. Une section « Pourquoi incrémental ? » / « Pourquoi des vues par worktree ? » : une idée, un dessin.
5. Les textes alternatifs longs des images (ils portent les mots-clés pour la recherche).
6. `AGENTS.md` à la racine, à côté de `CLAUDE.md` : la même consigne pour tout agent de code.

**Pour le produit** :
1. La **skill** : un paquet d'instructions que l'agent de code installe pour se servir de nos outils
   (search_code, usages, outline, la mémoire), au lieu de compter sur ses hooks seuls ; c'est le pas 1
   de la page de hooks-essai, en version portable.
2. Le **cache par empreinte de l'entrée et du code** (`memo=True`) pour nos nœuds de graphe : on a le
   gabarit vivant et l'index incrémental, pas un cache général des nœuds ; à mesurer sur les passes.
3. **Dire la provenance jusqu'à l'octet** dans l'IDE (ils la revendiquent, on l'a par occurrence : la
   montrer).
4. Les **files de rebut** (dead-letter) pour les morceaux refusés par un service d'embarquement : on a
   les réclamations et le rattrapage ; nommer et exposer ce qui a été mis de côté et pourquoi.

## Ce qu'on ne copie pas

Les chiffres d'accroche sans banc. Chez nous, un chiffre vient avec sa méthode et sa page.
