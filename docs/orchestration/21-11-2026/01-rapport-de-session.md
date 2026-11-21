# Rapport de session de l'orchestration, 21 novembre 2026 (nuit et matinée, jusqu'à 12 h)

Session c7 (orchestration, ListAgents `rag3db-5a`). Ce rapport est fait pour reprendre la main sans rien d'autre : il
dit l'état, ce que Lucie a décidé aujourd'hui, ce qui tourne et chez qui, et ce qui attend. La chronologie détaillée de
la nuit est dans `docs/orchestration/14-11-2026/03-la-file-en-pile.md` (entrées de 00 h 37, 02 h 09, 05 h 4x).

## L'état au 21 novembre, midi

- **Hôte en service aux deux postes** : master c5f3bb819e, posé à 08 h 38 (tablette et luciepc), lib **424ce93e56**
  (les deux correctifs moteur du matin : suppressions HNSW de hotfixs, garde du dictionnaire relu). Services
  d'embarquement 7878 (granite) et 7879 (bge-m3, avec sa référence) reposés et vérifiés à 06 h 20.
- **Lib épinglée des passes** : 424ce93e56 (`~/.cache/rag3db-libs/courant` aux deux postes ; le magasin est
  `~/.cache/rag3db-libs/`, pas `~/.cache/rag3weaver-build/`).
- **Atterri depuis l'hôte de 08 h 38** (dans le prochain hôte) : la page Poste, les commandes dans une vue, le compteur
  de jetons, la note des comptes, lucivy 4.4.0, l'atelier posé depuis l'hôte, « atelier » retiré de l'écran,
  `GET /ide/agents`, la clé stable des agents, la commande `rag3weaver-backend open`, la porte de vérification de
  l'équipier sur faux modèle, la pile de mémoire (gabarits versionnés, graphes sur disque, check_proposal, Decide en
  parts), le pousseur (identifiants français signalés, lot retiré pendant sa passe non poussé, test SAUTÉ compté non
  jugé).
- **Pose d'un hôte** : fermer les backends de code par `/backend/close` avant l'arrêt (le point de reprise évite le rejeu
  de 16 min), puis les **rouvrir** par `/backend/open` et vérifier `held_closed: null` (une fermeture survit au
  redémarrage) ; la réouverture du dépôt de la tablette coûte ~35 s même sans journal à rejouer (ticket de l'arrêt).
  Lucie dit quand elle teste vraiment ; hors de ses essais annoncés, une coupure courte et annoncée est permise.

## Ce que Lucie a décidé aujourd'hui (ses mots en mémoire)

**L'IDE et le produit** : l'IDE se range autour des **agents de code** (pas de la recherche, qui est pour l'agent ; la
plomberie sous « Avancé ») ; l'accueil est un arbre d'agents racines dépliés en sous-agents, tâches et plan en cours,
avec « À intervenir » en tête ; chaque agent montre sa mémoire, ses vues, sa boucle en graphe (forme fixe animée,
repliée) ; sous une vue, seulement ce qui n'est pas poussé (commits non poussés et non-commité) ; la liste des PR d'un
agent (ouvertes par défaut), et le diff d'une PR ; « Expliquer le code » en deux vues (« Comprendre le code »,
« Choix techniques », « Architecture » si elle se sépare), nourries des justifications du relecteur et de l'auteur ;
un **relecteur de PR** (deux vues, recherche, mémoire, sous-agents, relancés après leur fin), indépendant (la
conversation de l'auteur accessible mais pas injectée ; il vérifie chaque affirmation par le résultat réel des
outils), en boucle bornée avec l'auteur, la fusion toujours à l'humain. Un **agent part neutre** : on lui dit où aller ;
il ne propose l'index que pour un projet de code ou de docs suivi ; un dossier non-git s'ouvre tel quel ; l'accès à une
racine passe par la porte et se range, stable, par agent (case « tous mes agents »), révocable.

**Le mode auto** (premier point du produit) : une carte « accepter / refuser » jamais cachée, dont l'état change ;
derrière elle, dans l'ordre : le rappel des autorisations rangées en mémoire (option par agent), à 30 s l'**appel** de
l'agent (preuves par référence, l'hôte relit le journal), 10 min plus tard le **survivant** (« dangerous standalone
survivor », vérificateur lourd de dernier recours, modèles admis dans un fichier de l'utilisateur, refus avec raison,
risque et alternative ; v1 « dangereux et basta »). Afficher un secret jamais. Supprimer dans ses worktrees sans
demander si le fichier est sur un distant ou créé par l'agent. La politique de la porte est un **graphe déclaré**,
non éditable par l'agent ; les gardes restent en Rust ; un nœud `postpone` (et le cron) porte les délais.

**Le code** : identifiants neufs **en anglais** même quand la demande est en français ; les chaînes (invites, textes
affichés) restent en français jusqu'au multilingue ; le pousseur signale un identifiant neuf en français (listes de
mots MIT dans `extension/rag3weaver/data/wordlists/`) ; ticket de la refacto :
`docs/orchestration/tickets/2026-11-21-des-identifiants-en-francais-a-renommer-avant-le-multilingue.md`.

**Le scripting** : validé (page ED 01 du 21) ; à avancer avec ED (erreur qui nomme le fichier, rechargement à chaud,
contrat d'accès aux entités) ; un essai de **nœud WebAssembly** à ED.

**Lucivy** : 4.4.0 publiée (au mot de Lucie) ; `commit_tagged(g)` : (A), la génération du paquet, lot à lucivy ;
**compaction par shard** voulue (« oui complètement »).

**Les postes** : un relevé de charge toutes les 2 min (`~/.cache/rag3weaver-build/poste-load.tsv`) ; des places
déclarées par ressource, `poste cpu` et `poste gpu <Gio>` (« lourd » seul ne se dit plus) ; GPU admis sur la mémoire
restante : tablette le week-end en laissant 30 Go libres, luciepc toute la semaine sur card2 (réserve 19 Gio des
services), card0 de 01 h à 08 h sans session sur seat1, pour des bancs jetables seulement.

**Les graines** : tableau dans `extension/rag3weaver/visions/graines.csv` (intérêt, luxe, vendeur, taille, dépendances,
priorité) ; P1 dans l'ordre de Lucie : mode auto, outils de navigation dans le code, dépôt de référence à la demande,
branchement sur Claude Code et Codex, sortie des commandes gardée, « ne jamais toucher ». Graines du jour : deltas
LUCID entre postes (2/10, luxe 10/10), langue parlée comme un dialecte, fork de llama.cpp aux idées de FreeToken (P4),
**un agent « super orchestrateur »** qui lit les résumés de l'archiviste et les PR, écoute les évènements et ce qui
attend une intervention, et en parle (le rôle de c7 rendu produit).

## Ce qui tourne, chez qui

- **arbre** : l'agent neutre en pile (`open` atterri ; sans index, index à la demande, offre d'index, dossier non-git en
  file), `root-grants` redéposé, puis le bac monté sur les racines accordées et la commande rejouée, puis cloner,
  puis les PR d'un agent. Ticket : le target partagé sous une vue.
- **archi** : « fermer sans détruire » (le dossier d'une tâche archivé, la fiche fermée complète, la clé dedans), puis
  `GET /ide/run`, les primitives de la porte (Card, étage 0, RuleClass, ResolveEvidence), la porte en graphe avec ED,
  les juges, l'appel, le survivant ; les pages 03 et 04 du 21 à mettre à jour d'un seul dépôt.
- **recherche** : tranche 1 (l'accueil en arbre) déposée ; tranche 2 (la page d'un agent), puis les diffs non poussés.
- **mémoire** : l'attribution `author = agent:<clé>` (1b), puis `GET /ide/agent/memoire` en lecture, puis le masquage
  des secrets sur les sorties d'outils, puis les autorisations durables (catégorie `authorization`). La copie locale
  du gabarit memory du serveur de Lucie suivra le binaire après son essai (décidé par c7).
- **ED** : l'erreur qui nomme le fichier (redéposée), le rechargement à chaud, les pièces de la porte (`postpone`, types
  fermés, chargement de `gate/`), l'essai WebAssembly, la page du contrat, explain_diff ; la mesure des vues à règles
  égales (copie de la tablette en règles 13 prise à 11 h 50).
- **embarquement** : la reprise d'un vecteur par l'empreinte du texte ; (b) le partage des morceaux est déjà la
  conception du 6 nov., à prouver par la mesure d'ED.
- **hotfixs** : C1 sous TSan à luciepc, puis l'insertion groupée dans HNSW et le coût d'`efc` (36,6 ms par vecteur au
  `SET`, mesuré par ED).
- **cœur** : C1, la sonde de relecture à la tablette.
- **lucivy** : le lot (A) des générations étiquetées, puis la compaction par shard.
- **G** : le pousseur (verdict sur sa propre entrée déposé), `poste cpu`/`gpu` et card0, le relevé de charge (page après
  une journée), la vérification des identifiants (fichiers générés à ignorer).
- **banc** : les juges du mode auto. Mesuré : Gemini Flash 1/51 « autorisé » à tort sur une classe dangereuse, Qwen3-4B
  28/51 ; des règles en code aident plus que deux juges. En cours : Qwen3.5-9B (téléchargé, vérifié), puis Inkling 2
  Mini (le GGUF d'Unsloth ne charge pas sur le llama.cpp officiel), avec une mesure de cohérence (quatre formes par
  cas).
- **tty** : le navigateur, lot 1 (lire la page, les onglets, la console), sur fausse extension.
- **codeparsers** : les occurrences attendent la décision de conception (voir ci-dessous).

## Ce qui attend Lucie

1. **Les occurrences** (codeparsers, `docs/codeparsers/21-11-2026/01-le-cout-des-occurrences.md`) : (a) plusieurs liens
   par paire, distingués par la position (touche le drain et le contournement d'octobre d'un défaut du moteur qui
   perdait une écriture répétée sur une paire), ou (b) une entité `Occurrence` légère, non cherchable (sert aussi
   `Local` et `Expression`, approuvées le 17 nov.). Recommandé : (b).
2. **Les juges du mode auto** : Gemini Flash pour l'autorisation, Qwen local pour le danger, plus quatre règles en code
   (citation mot pour mot, commentaires ôtés, cible nommée pour base, sortie et secret, accord durable sans destruction
   hors de ce que l'agent a créé) ; à revoir après Qwen3.5-9B et Inkling.
3. **Inkling 2 Mini** : le GGUF de `ggml-org` (`Inkling-2-Mini-IQ4_NL.gguf`, 17,6 Go) au lieu de celui d'Unsloth.
4. **Les trois autres juges intermédiaires** (Qwen3-8B, Qwen3-14B, Gemma 4 12B), après le résultat de Qwen3.5-9B.
5. **Les dossiers de notes** comme projets (proposé : oui s'ils sont surtout en Markdown) et le compte et la
   visibilité GitHub par défaut d'un dépôt créé par un agent (proposé : le compte de `gh`, privé, montrés avant
   l'accord).
6. **Les règles d'analyse** : si la mesure d'ED montre qu'un écart de règles (bases en 13, binaire en 15) fait tout
   recalculer à l'ouverture d'une vue, repasser ses bases aux règles 15 dans une fenêtre annoncée.

## Les leçons de la nuit et de la matinée (en mémoire)

- Un témoin neuf va dans **un fichier de tests à soi par lot** (`<module>/tests/<lot>.rs`, lignes `mod` en ordre
  alphabétique) ; un conflit qui coupe dans des corps se résout en repartant du fichier de master ; une pile se
  redépose entière, dans l'ordre ; le champ témoin du pousseur est **un seul** filtre cargo, dans la bonne suite.
- Une fermeture par `/backend/close` survit au redémarrage : toute pose rouvre ce qu'elle a fermé.
- Un état « posé ou pas » se prouve par `git merge-base --is-ancestor` sur la tête de l'hôte.
- Un relevé ou une mesure se lit sur le bon magasin et la bonne lib (fausse alerte de c7 à 07 h 02).
- Une suite qui ne joue rien n'est pas verte (e2e_equipier sautait ses cas ; le pousseur compte maintenant SAUTÉ non
  jugé).
- Ne pas laisser une demande française dériver en identifiants français, dans les consignes comprises.
