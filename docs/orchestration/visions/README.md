# Visions

Ce dossier rassemble ce qui dit **où va le projet**, par opposition à `docs/nouveautes.md` qui dit ce
qui est atteint. Déplacé ici le 11 novembre 2026 depuis `docs/vision_roadmap_09_2026/` et
`extension/rag3weaver/docs/vision_roadmap_09_2026/` (les liens relatifs des archives datées vers ces
dossiers ne tiennent plus ; l'histoire est là, pas l'état).

## Les feuilles de route de septembre 2026

- `vision-roadmap-09-2026/` : 00-index, 01-la-vision, 02-les-deux-moitiés, 03-normaliser des tableurs,
  04-le catalogue comme graphe, 05-ce qui a tenu depuis février, 06-la feuille de route, 07-événements,
  runs et boucles, 08-des catalogues de gabarits, 09-trois rôles et une seule main, 10-l'agent, vision et
  distance, 11-donner des commandes à un agent, 12-ce qui manque à l'agent de code, 13-avaler une base
  existante, 14-le schéma comme artefact, 15-le moteur cesse d'être mono-backend.
- `vision-roadmap-09-2026-produits/09-les-produits-et-leur-empaquetage.md`.
- Les relevés de ces visions au 1er octobre : `extension/rag3weaver/docs/1-octobre-2026-22h47/03-…` et `04-…`.

## Les visions de novembre 2026 (pages de conception, laissées dans leurs dossiers de session)

- **Résidence des modèles et préchauffage** — demande de Lucie du 12 novembre :
  garder chauds les usages courants, charger et libérer les besoins ponctuels,
  composer ModelSource et Serveur avec une politique de ressources commune :
  [conception](2026-11-12-residence-des-modeles-et-prechauffage.md).
- **Un navigateur accessible aux agents par MCP** — piste future de Lucie du
  12 novembre : étudier un pont open source, avec extension Chrome éventuelle,
  réutilisable par différents agents. Recherche différée, aucune intégration livrée :
  [intention et critères](2026-11-12-un-pont-navigateur-generaliste-par-mcp.md).
- **Explorer les arbres de symboles** — demandes de Lucie du 12 novembre : vue
  par fichier, membres, parents/enfants bornés et hiérarchie optionnelle du grep ;
  composer outline et les relations existantes, qualifier couverture et provenance :
  [conception](2026-11-12-explorer-les-symboles-et-leur-hierarchie.md).
- **Mode auto contextuel et accords** — demande de Lucie du 12 novembre : relier
  les décisions à la tâche, reconnaître les accords pertinents et réduire les
  interruptions. Comparaison sourcée OpenCode/Gemini CLI/Cline et proposition sur
  les primitives existantes, sans raccord livré :
  [conception](2026-11-12-mode-auto-contextuel-et-accords.md).
- **Vitrine produit, référence CocoIndex** — choix de Lucie le 12 novembre pour le design et
  la présentation du framework : promesse lisible, démonstrations concrètes et accès progressif
  au modèle. Site à planifier, aucune publication engagée :
  [direction de présentation](2026-11-12-vitrine-produit-reference-cocoindex.md).
- **Applications déclaratives et contrats d’effets** — avis d’architecture du 11 novembre :
  distinguer application, déploiement et autorisation ; examiner les effets et la provenance des
  compositions, puis en dériver des preuves techniques. Vision future, aucun nouveau chantier lancé :
  [note](2026-11-11-applications-declaratives-et-contrats-d-effets.md).
- **Une vue est un filtre** — un index par dépôt, des vues par worktree, par branche, en pyramide ;
  le fichier composite ; décidé par Lucie le 6 et le 8 novembre :
  `extension/rag3weaver/docs/08-novembre-2026-arbre/01-conception-des-vues-marche-1.md`.
- **L'atelier lit et écrit** — lecture par tranches, trois écritures sous empreinte, périmètre à la
  racine de lancement, cartes d'accord, journal des décisions pour un futur décideur local :
  `extension/rag3weaver/docs/09-novembre-2026-atelier-lire-ecrire/01-conception.md` (atterri le 9).
- **Les hooks Claude Code** et **la recherche de code par les hooks** (l'index amené aux sessions sans
  qu'elles y pensent) : `extension/rag3weaver/docs/07-novembre-2026-hooks-claude-code/01-…` et
  `07-novembre-2026-hooks-essai/04-la-recherche-de-code-par-les-hooks.md` (en attente du mot de Lucie).
- **Le classeur local dans le rappel** (Qwen3-4B à luciepc) et **M8, la montée des gabarits** sur le
  gabarit vivant : `extension/rag3weaver/docs/09-novembre-2026-memoire-longue/03-…` et `02-…`.
- **Un décideur d'actions local** (approbations et refus journalisés comme matière d'entraînement ;
  banc du 8 novembre) : `extension/rag3weaver/docs/08-novembre-2026-banc/`.
- **L'agent de code en terminal** et le pont entre agents de code :
  `extension/rag3weaver/docs/07-novembre-2026-tty/02-conception.md`.
- **Le moteur ennuyeux** (semaine du 7 novembre : 84 tickets ouverts, le moteur d'abord) ; **la
  mémoire de code pour agents** (focus du week-end des 8 et 9 novembre).
- Graines dites par Lucie, non ouvertes : la voix sur le chat (voicepipe + Pocket TTS), Laya en
  décideur local (ModernBERT), géo à la PostGIS (« ça viendra »), l'embarquement multimodal
  (EmbeddingGemma-2).

## Les visions plus anciennes (dans les archives datées)

`extension/rag3weaver/docs/23-aout-2026-20h33/36-…`, `49-…`, `51-…` ; `29-aout-2026-12h24/01-…` ;
`3-septembre-2026-17h50/02-…` ; `docs/23-fevrier-2026-22h09/10-…` ; `docs/14-fevrier-2026-20h36/04-…` ;
`extension/rag3weaver/docs/25-octobre-2026-memoire-longue/03-roadmaps-et-visions-en-memoire.md`.

## Décision d'horizon (Lucie, 13 novembre 2026)

**Un dépôt officiel rag3weaver, séparé de rag3db**, « un jour ou l'autre, quand on aura un truc vraiment
utilisable ». D'ici là, tout ce qui s'écrit pour rag3weaver (README, `AGENTS.md`, procédures, docs) doit
tenir seul : pas de chemin vers le moteur autrement que comme dépendance, pas de `../..`. Le README de
rag3weaver se refait sur le modèle de CocoIndex (`concurrents-cocoindex.md`) : lot d'archi du 13.

## Graines de produits (Lucie, 13 novembre 2026)

Dites à l'oral, à garder telles quelles ; aucune n'est ouverte. Toutes s'appuient sur ce qui existe : un
index par dépôt et des vues par worktree, le graphe de code (scopes, usages, appels), la mémoire longue,
le juge.

1. **Chercher les fuites de variables d'environnement et de secrets** : où une variable est lue, où sa
   valeur peut sortir (journal, réponse, fichier, commit), au fil des usages du graphe plutôt que par
   motif seul.
2. **L'analyse de code à la Sonar, mais plus poussée et avec le modèle** : les mauvaises pratiques
   déduites par heuristiques, plus celles que le modèle voit en contexte (le scope, ses usages, son
   histoire), avec une raison et un lien vers le code, jamais un score sans explication.
3. **Des revues automatiques en CI/CD avec un contexte persistant** : la revue d'une PR lit l'index du
   dépôt, les vues de la branche, la mémoire des revues passées (ce qui a été dit, accepté, refusé), et
   n'oublie pas d'un run à l'autre. **Et jusqu'au correctif proposé** : comme le contexte est toujours
   dans la mémoire du correcteur, et que l'équipe peut lui donner ses règles en fiches additionnelles
   (conventions, interdits, préférences), il propose directement un correctif et l'envoie par mail à
   l'auteur, ou se pose comme relecteur de la PR avec sa proposition. Le correctif est proposé, jamais
   poussé à la place de l'auteur.

4. **Un coffre de secrets (à la Vault ou Infisical) servi directement aux CI/CD et aux agents** : le
   relecteur automatique peut alors vérifier la base de données ou un service externe (« tout va
   bien ? ») en se servant d'un accès nommé, **sans jamais lire une variable d'environnement** ni voir
   une valeur ; le coffre fait l'appel, l'agent reçoit le résultat. C'est la règle de la maison (« les
   identifiants par le chemin de .vault, jamais leur contenu ») faite produit. « On verra plus tard ».

5. **Un petit classifieur de sécurité local, en garde devant l'agent**, et la chaîne qui va avec
   (précisée par Lucie le 13 au soir). Le classifieur ne fait rien de compliqué : quand une vraie valeur
   sensible passe (variable d'environnement, jeton, mot de passe, dans un prompt, une commande ou un
   fichier), il la remplace par une **référence opaque** dont l'agent ne connaît que le nom
   (`{{secret:DB_URL}}`), qu'il peut utiliser (le lanceur résout la référence au moment d'exécuter, hors
   de la fenêtre du modèle), et qu'il sait **temporaire**. C'est ensuite **l'agent** qui, sur demande,
   range la référence dans le coffre durable (graine 4) ou fait autre chose avec. La sortie est caviardée
   au retour. Deux chaînes selon l'équipe :
   - classifieur sécurité local → classifieur des droits d'action « auto » (cloud) → l'agent et son
     modèle cloud ;
   - classifieur sécurité + « auto » local (quand l'auto est déjà local, pas de raison de les séparer)
     → l'agent et son modèle cloud.
   Dans les deux, le modèle cloud ne voit jamais un vrai secret : il ne voit que des références. C'est la
   porte d'accord de l'atelier et le décideur local, appliqués aux secrets.

Le code est le premier produit du framework ; ces cinq-là sont des vues et des agents de plus sur le
même index.
