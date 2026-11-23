# Rapport de session — orchestration, 22 nov. 21 h → 23 nov. 14 h 15

## La direction de Lucie

- **La semaine** (23 nov.) : « des agents vraiment capables de coder, et un IDE super facile à utiliser ». Ligne 1
  (l'agent de code va au bout) avant ligne 2 (swarms). La sécurité est une amélioration, pas un préalable.
- **Leitmotiv** : « Become the hive mind. Control entire swarms of agents. » (en tête d'`AGENTS.md`). Vocabulaire :
  swarm, hive, orchestrateur-<projet>, « Accueil » par défaut, libellés par thème.
- **Règles posées aujourd'hui** : jamais de liste de commandes en dur (une règle par l'effet) ; tout texte qui
  atterrit chez le modèle vient d'un gabarit à variables (`{$nom}`), identifiants compris ; un message de commit dit
  le fait, jamais une session ni un outil d'IA (contrôle au pousseur, refus à partir du 24) ; la ligne de
  `docs/nouveautes.md` part dans le commit du lot.

## Ce qui a atterri et est posé (hôte 0084d2b217, posé à 14 h 09, terminal 1bf6d52c)

L'agent de code va au bout sur de vraies demandes (three.js, fastapi, jeu en python, nestjs) : mode auto par défaut,
niveau de porte choisi au lancement (`bac` par défaut), commit de fin de tour signé rag3weaver, travail atterri dans
le dossier (git, vide ou sans git), faits de chaque tour, mémoire de la conversation, commande introuvable dite,
lignes de shell complètes, PATH du shell de connexion, commandes de fond qui ne bloquent plus et meurent avec leur
tâche, 40 000 jetons de sortie, réponse vide dite et relancée, tour silencieux relancé, arrêt qui arrête, terminal
sans argument, Ctrl-C/Échap, lignes de l'hôte dans la vue, oui humain seul. Côté IDE : onze lots sur douze sur
master (le dernier, l'ordre de l'affichage, en passe). Lib épinglée 3a2639714e.

## En cours, par session

- **archi** : réflexion affichée par défaut (tri des morceaux `thought`), `new-project-index` (l'hôte indexe un
  projet qu'il a créé), jachère (fenêtres toujours découpées, compaction à 80 %, résumés en ajout seul), appel de
  fond (« outil asynchrone lancé », annonce en tête du retour d'outil suivant, réveil par événement), reprise au
  chargement, gabarit de messages par langue. **À ajouter** : un dossier plein sans git ouvert au terminal (Lucie,
  14 h 1x) : c'est l'agent qui doit avoir la puce à l'oreille (proposer git + index).
- **tty** : `tty-turn-calls`, `tty-context` déposés ; ensuite `--continue` et la reprise au chargement.
- **mémoire** : `timings_ms`, l'appelant et les filtres `author`/`swarm`, puis `recall_for_request` (direct), puis
  la préparation par un petit modèle (modes `recent`/`whole_thread`/`memory`).
- **recherche** : `recherche-call-hop` déposé (saut d'appel déclaré dans le gabarit) ; mesure modèle contre grain
  avec embarquement ; puis rejouer le parcours de Lucie dans Chrome.
- **banc** : la mesure de la politique de contexte (fil de 20 et 200 tours) ; essai 6 sans biais.
- **G** : registre des targets (25/60 Gio, global 300), rpath stable, check_features qui dit pourquoi, le pousseur
  qui n'attend plus B, docs pendant un train ; places à 4.
- **codeparsers** : arrêt pendant une repasse, lot 20 des relations (dont la 13e, `hasher.update`).
- **hotfixs** : balayage RegionDelete ce soir à 20 h 05.

## À faire en premier en reprenant

1. Rejouer le parcours sur l'hôte de 14 h 09 (`~/essai-c7-pose-1415` en cours : une demande puis « je lance
   comment ? ») avant de dire « prêt » à Lucie.
2. Donner à archi le point du dossier plein sans git.
3. Lucivy 4.5.1 attend le seul mot de Lucie.
4. Questions ouvertes à Lucie : la relecture (`heavy judge`) à confier ? la règle d'ensemble des réveils
   (pause, plafond, carte, vue) à confirmer.
