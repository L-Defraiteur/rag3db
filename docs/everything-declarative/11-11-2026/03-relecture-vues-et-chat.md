# Relecture des vues et du chat — 11 novembre 2026

Relecture des conceptions vues marche 1 et atelier lire/écrire, pendant l'attente de la
seconde passe des réclamations. Remarques transmises à arbre principal, recherche, archi et
orchestration ; aucune modification de `code.rs` ni du gabarit code dans ce lot.

## Vues : contrats à préserver

- L'identité du contenu réutilisé couvre les entrées réellement embarquées : signature,
  contenu, docstring, titre selon les champs déclarés, et configuration de découpage.
  La normalisation doit être versionnée ; si elle change les octets, il faut remapper
  offsets et extraits au texte de l'occurrence, ou commencer sans cette normalisation.
  Les vecteurs restent distingués par modèle/configuration ; partager le texte n'autorise
  pas à partager les vecteurs de deux modèles.
- Une occurrence garde chemin, position et identité publique. `search_code` est lié à
  `Scope` dans le gabarit courant ; rechercher des versions partagées doit projeter les
  résultats vers les occurrences visibles avant classement, déduplication et pagination.
  Le rendu, les liens, les graphes `after` et la fraîcheur gardent leur forme publique.
- Une vue descend récursivement ses parents épinglés ; elle ne retombe pas sur le principal
  courant. Un tombstone masque les couches inférieures. Résolution, filtre, enrichissement
  et provenance utilisent une seule génération sous l'instantané de lecture.
- Le filtre de visibilité est obligatoire, composé avec le domaine et le filtre utilisateur,
  avant le top-k de chaque signal. Il ne peut être remplacé par un filtre utilisateur.
  Un backend incapable de filtrer refuse ; un avertissement avec des résultats hors vue
  ne respecte pas ce contrat. Pour les arêtes, source et cible doivent être visibles.
- La précédence domaine/filtre utilisateur existante est un sujet distinct : recherche
  constate que le domaine n'est actuellement appliqué qu'en l'absence de filtre utilisateur
  dans `generic_search_nodes.rs`. Ne pas modifier cette précédence implicitement avec les vues.
- `view` est un paramètre commun typé des outils ; une vue inconnue ou ambiguë refuse.
  Une branche sans worktree se lit par les objets git ou le composite, pas par un chemin
  disque supposé. Les écritures requièrent un worktree résolu. La provenance reste optionnelle
  dans les réponses au modèle.
- Une migration qui déduplique réduit les comptes physiques de chunks/vecteurs : prouver
  couverture des occurrences, réutilisation du contenu et des vecteurs, absence d'appel à
  l'embarqueur, résultats équivalents et directions des relations conservées ; l'égalité
  brute des comptes avant/après n'est pas un invariant universel.
- L'ancien format doit être reconnu explicitement ; `index confirm` ne réalise pas une
  migration silencieuse. Les mesures M3–M5 restent à jouer ; aucune latence n'est acquise.

Témoins proposés : une fonction à deux positions et sous deux modèles ; parent/enfant/ancre
avec tombstone ; filtre utilisateur visant une occurrence masquée ; meilleurs voisins tous
hors vue avant filtrage ; arête vers une cible masquée ; migration sans embarquement neuf.
Le garde-fou des scopes seulement déplacés reste nécessaire. La conception historique contient
encore des phrases sur leur réembarquement contredisant le témoin existant : à retirer.

Arbre principal a confirmé la prise en charge des corrections et tient `code.rs` et
`templates/backends/code` pour le lot 1a. ED ne les édite pas simultanément.

## Chat : inventaire et prochains contrats

Sur les gabarits lus, l'atelier expose onze outils : attache/liste de worktrees, ouverture de
branche, commandes, avancement, lecture fichier/scope, trois écritures, demande d'accord.
Il n'expose pas `search_code`, `grep_files`, `list_files`, `outline`, `usages`, `impact`,
`callees` ou `wait_output`, présents dans le gabarit code (celui-ci ne propose pas `read_scope`).
Les tests et opérations git passent par `run_command`, sans outil dédié.

`OutilsDuChat::de` retire les commandes et écritures hors tâche quand le manifeste ne les
permet pas ; l'atelier déclare les deux permissions. Proposer un outil dans un gabarit ne
prouve donc pas à lui seul sa disponibilité dans une conversation : vérifier aussi la liste
réellement exposée au modèle, le dépôt cible, la porte et le résultat observable.

Le prochain relais des outils de recherche/navigation de l'atelier doit porter le dépôt et
la vue explicitement, sans repli silencieux vers un autre dépôt. Accord, refus, relance et
suivi de commande doivent conserver cette cible et rendre un reçu suivable. Archi tient le
parcours et l'UI, ED les contrats/gabarits/runtime ; l'inventaire UI reste à confronter avec archi.

Le défaut de fin sans livrable est confirmé dans `teammate.rs` : aucun plan vivant et un
`step_done` suffisent à clore sans examiner `remaining`. Corriger séparément, rouge d'abord :
`remaining` non vide relance un pas ; une demande explicite de report/question/accord ne se
clôt pas avant son livrable et la réponse attendue. Le petit cas sans plan et sans reste doit
continuer à se terminer. Le ticket reste ouvert ; aucun correctif de ce défaut dans le lot
réclamations.
