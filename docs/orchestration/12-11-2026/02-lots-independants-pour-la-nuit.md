# Lots indépendants pour la nuit du 12 au 13 novembre 2026

Lucie demande de mettre les sessions à profit cette nuit, de découper ce qui
peut avancer en parallèle et de prendre les visions ou correctifs du cœur
quand ils constituent une suite utile. Ce document autorise les lots bornés
ci-dessous, sans créer d'activité pour remplir les machines. Une session qui
a fini ou attend une dépendance réelle reste normalement en attente.

## Ressources et créneau GPU

- Fuseau Europe/Paris. Pour **cette nuit seulement**, card0 est aussi disponible
  le13novembre de00:00 à08:00 ; card2 reste une ressource normale du projet.
  Cette exception remplace la réserve de card0 au jeu pendant ce créneau.
- Optimiseur prépare avant usage une entrée qui vérifie l'heure réelle, une
  portée privée identifiée et un arrêt borné avant08:00. Prévoir une marge de
  restitution, par exemple fin des modèles privés à07:55. Toute nouvelle
  acquisition hors créneau est refusée ; les tâches suivantes utilisent card2.
- Tous les modèles nocturnes sur card0 appartiennent à cette portée et ont une
  durée bornée par le temps restant. Leurs PID/starttime, modèle, dimension,
  périphérique/PCI et consommateurs sont enregistrés. Arrêter seulement ces
  unités privées ; ne pas couper un jeu, le compositeur ou les services existants.
- Cartographie antérieure à revérifier : card2=PCI04:00.0/ROCm0,
  card0=PCI07:00.0/ROCm1. Un numéro Vulkan ou un filtre ROCm seul ne prouve pas
  le placement des poids et du calcul. Choisir le profil puis mesurer les vrais
  processus ; ne pas exporter un choix GPU global aux autres sessions.
- Le besoin réel décide de l'usage : aucun chargement de poids pour occuper la
  carte. Conserver granite278/7878 et BGE/7879 si utilisés ; 107m/7880 et privé7882
  restent froids sauf besoin qualifié, sans substitution de modèle/dimension.
- Au plus4 lourds, j8 par travail, Max32G/target32Go privés, priorité mesure.
  Étaler les compilations selon CPU/RAM/IO réels ; un plafond n'est pas une
  réservation de mémoire. Une mesure comparative attend une fenêtre exclusive.

## Répartition, propriétaires et interfaces

| Porteur | Lot et premier résultat concret | Frontière d'édition |
|---|---|---|
| Arbre | Suite du ticket migration après A et RO : protocole de génération/publication composite puis première primitive bornée sur fixtures | migrations/plan/journal et métadonnées de publication ; pas de données ou d'activation de Lucie |
| ED | Prouver le parcours de code complet par le vrai chat/outillage du framework dans un projet isolé, avec Gemini configuré | hôte/agent/exécution/reprise, tests d'intégration ; pas navigation des symboles ni résidence des modèles |
| Archi | Première tranche utile des symboles : arbre de fichier et membres/parents/enfants qualifiés à partir des primitives existantes | nouveau module de navigation, Plans/gabarits et présentation ; annoncer avant modification de code_tools ou d'un manifeste partagé |
| Codeparsers | Qualifier la couverture effective des membres et conteneurs pour Archi, sur petites fixtures | son périmètre IR/fixtures ; pas de pointeur parent ni réparation des anciens worktrees/admins |
| Embarquement | Première tranche de résidence/activation : acquisition commune, état de chauffe et annulation isolée, accessible à un appelant existant | model_source/serveur et fournisseur ; pas de nouveau modèle de décision ni changement global de politique |
| Banc | Préparer puis mesurer les faits de chauffe/reprise de la tranche d'embarquement sur un modèle privé utile | harness et preuves, aucun fichier runtime d'embarquement édité en parallèle |
| Cœur C++ | Reprendre le ticket CI clang-format18, fenêtre de formatage pur ouverte après inventaire | C++ de master et exclusion des annexes ; aucun autre porteur ne modifie ces mêmes fichiers pendant le lot |
| Optimiseur | Entrée/sortie effective de la fenêtre card0 et coordination des consommateurs | unités/profils temporaires et preuves hors dépôt ; pas de lib courante ou config utilisateur substituée |
| Mémoire | Revue des contrats de publication et de l'interface de ressource quand les diffs arrivent | revue seule, sans modifier les fichiers des propriétaires |

Les petits ajouts à lib.rs, Cargo.toml ou un manifeste partagé sont annoncés et
coordonnés avant édition ; les interfaces se relisent tôt. Les worktrees/targets,
branches cx- et fixtures de chaque lot sont distincts. Pas de nouvel orchestrateur
multiagent builtin. Les protocoles, politiques et mécanismes restent séparés.

## Critères des lots

### Couches — suite après RO

Conserver la décision777 : identité de publication distincte de l'empreinte
d'état, réutilisation après requalification et nouveau plan figé, aucun plan
ancien modifié. Inspecter d'abord les primitives de journal/plan existantes.
Produire un contrat et des témoins pour création explicite d'une origine neuve,
publication P1→P2, retour logique A→B→A, rejeu de la même opération, identité ou
génération étrangère et interruption. Tout fait incomplet reste un refus.

Une première persistance ne concerne que des fixtures/catalogues neufs possédés,
avec preuve de leur état et transaction réelle. Ni bootstrap implicite de
legacy ni écriture de storage_descriptor sur une base non qualifiée. Une mécanique
de journal ne suffit pas à déclarer un transfert ou un format appliqué. Publier
le plan borné et le diff avant lourd ; remonter seulement un vrai choix nouveau.

### Agent IDE — parcours réellement accessible

Dans un hôte et un projet Git neufs : demande synthétique, exploration par les
outils réellement exposés au chat, modification, exécution d'un vrai test,
lecture de son résultat, diff puis reprise de conversation/tâche. Gemini passe
par le fournisseur Vertex configuré et l'agent du framework, avec outils ; un
appel de primitives directes ou une réponse affirmant la réussite ne suffit pas.
Les actions et preuves de fichiers/processus/outil doivent correspondre.

Commencer par une fixture petite et une recette bornée ; conserver les refus,
erreurs et limites d'accès. Corriger une lacune dans le contrat commun, sans
mode permissif ad hoc ni consentement utilisateur fabriqué. Pas de pose/restart
du daemon personnel pour ce lot initial. Une pose concrète ultérieure se prépare
avec retour et vérification du parcours avant coordination.

### Symboles

Relire le ticket et la vision, puis qualifier ce qui est déjà outline et ce qui
manque. Une première tranche doit apporter une vraie exploration utile et être
accessible par les outils communs. Ambiguïté, profondeur, cycles/troncature,
index périmé et couverture partielle restent explicites. Les relations de
contenance, appartenance et héritage ne sont pas confondues. Ne pas inventer
une exhaustivité des champs/membres absents de l'IR.

### Résidence et mesure des modèles

Composer ModelSource et Serveur/Attache : identités explicites, activation unique
sous concurrence, annulation d'un demandeur sans tuer le travail d'un autre,
délai et état de chauffe observable. Les politiques (maintien chaud, TTL,
éviction) sont déclarées, pas inventées par le client ; aucune éviction globale
ou changement des services existants n'est activé sans son lot qualifié.
Commencer par processus/constructeurs factices ; brancher une première capacité
réelle avant de la dire livrée. Banc mesure ensuite les démarrages/partages/
annulations utiles sur service privé, avec recette et placement publiés avant
le réel. Pas de répétition du banc Qwen déjà terminé.

### Cœur C++

Ticket du25octobre : le retrait cassert est déjà intégré. Relire les derniers
faits CI puis utiliser exactement clang-format18 de la CI, pas la version23
locale. Inventorier/exclure les annexes de documentation et sous-modules à ne
pas réécrire, produire un formatage pur, vérifier absence de logique, puis bâti
privé et transaction_test selon recette publiée/coordonnée. Aucune nouvelle lib
posée ni rejeu de la trace privée précédemment bloquée par un refus automatique.
Ce refus particulier ne se contourne pas par un autre agent/modèle ; les travaux
distincts de CI et de source autorisés peuvent avancer normalement.

## Modèles de décision et Cloud

Gemini et les modèles locaux sont des ressources de travail normales quand
utiles, conformément417 ; pas d'appel cérémoniel obligatoire. Secrets par les
clients configurés et leur chemin, jamais ouverts/affichés/copiés par les agents.

Le12novembre, Lucie demande d'attendre Vertex pour Gemini Decision1 : ses crédits
sont Cloud. La [publication officielle](https://blog.google/innovation-and-ai/technology/developers-tools/gemini-decisions-api/)
annonce AI Studio et Google Cloud « coming soon » ; la
[documentation](https://aistudio.google.com/docs/decisions-api) montre la Gemini
Developer API. Aucune clé AI Studio, activation de facturation ni voie alternative
n'est créée. Documenter le candidat et préparer le protocole comparatif sans
appel tant que l'accès Vertex n'est pas disponible et vérifié.

## Validation et intégration

Chaque travail reprend son ticket existant ou celui créé ci-dessous avant
action. Source et recette publiées avant tests ; premier rouge inattendu arrêté
et diagnostiqué, pas de replay inchangé ni reset silencieux de budget. Borne
annoncée après acquisition réelle, sous la primitive commune pour les suites.
Une fonction ajoutée sans appelant accessible n'est pas une fonctionnalité finie.

Les sessions avancent autonomement dans ces frontières, utilisent Gemini ou
card2 quand pertinent, et card0 dans sa fenêtre après entrée vérifiée par
Optimiseur. Les besoins de carte ordinaires ne réclament pas une nouvelle
permission de Lucie. Les vrais choix produit manquants vont sur leur page de
conception avec questions numérotées, sans arrêter le reste du lot.

Revue, passe adaptée puis tour master « à toi », poste push/dernierfetch/ifVERT/
trois contrôles ; nouveau nom après rebase et préservation des travaux entrants.
Nettoyage immédiat des seuls biens du lot après preuves/archives/garde fraîche ;
remotes, rouges et journaux conservés. Ticket réalisé/abandonné déplacé closed,
attente et prototype seul laissés ouverts. Le rappel90min contrôle activité,
dépendances et ressources ; Lucie n'est prévenue que d'une avancée utile ou
d'un vrai blocage. À08:00, card0 est rendue ; CPU et card2 restent disponibles.
