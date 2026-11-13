# Passation tty — 13 novembre 2026

## État et périmètre

Session en veille. Aucun nouveau calcul ni lot produit autorisé. Les anciens
créneaux et délais ne se réactivent pas à la reprise. Aucun push master demandé
ou effectué par tty ; l'orchestration rassemble les passations.

Le terminal livré avant cette reprise est décrit dans
`docs/tty/11-11-2026/01-rapport-de-session.md`. Aucun travail tty-4, mesure ou
thème n'a été démarré. L'unique lot repris est une régularisation documentaire
du défaut de compilation minimale, désormais corrigé par d'autres lots.

## Branche, commits, postes et travail restant

- Dépôt distant : `origin`, `github.com:L-Defraiteur/rag3db`.
- Branche locale et distante : `cx-docs-tty-features`.
- Worktree portable : `/home/lucied/git_workspaces/cx-docs-tty-features`.
- Pointe avant cette passation, relue égale au distant :
  `e593c67a2a1756bd6a3a6bdf4ec8048b290ad1f3`.
- `0882f0a86` contient le déplacement du ticket seul : une mise en index
  incluant son ancien chemin a échoué. Le commit publié n'a pas été réécrit.
  `e593c67a2` complète l'État corrigé, les preuves, le rapport et le dump.
  Intégrer le lot complet, jamais le déplacement isolé.
- Arbre propre avant rédaction de cette passation. Aucun WIP possédé ailleurs
  dans cette reprise. Le commit contenant cette passation est sa nouvelle
  pointe ; le retrouver avec `git log -1 origin/cx-docs-tty-features`.
- Aucun worktree ni target créé sur luciepc par cette reprise. L'absence
  d'anciens worktrees y vient du rapport du 11 novembre, pas d'un nouvel audit
  distant. Aucun inventaire distant lancé pour cette passation.
- Reste : revue et coordination de l'intégration documentaire par
  l'orchestration. Pas de travail produit supplémentaire autorisé.

## Ticket et preuves

Ticket porteur tty :
`docs/tickets/closed/2026-11-07-la-lib-rag3weaver-ne-se-batit-pas-sans-rag3db-native.md`.
Sa clôture est publiée sur la branche, pas annoncée intégrée à master.

Critères relus : ouverture native protégée dans `base_de_verification`
(`9231f07dc`), test natif de record_nodes qualifié (`c74e3310d`), helper
Backend::call_tool sous test et feature native ; jeux vide et daemon présents
dans la garde canonique (`5a4049b9e`). Le contrôle est `check --tests`, incluant
la bibliothèque normale ; il ne prouve pas une exécution des tests minimaux.

Preuve existante publiée par `cdcc032df` :
`docs/recherche/13-11-2026/13-preuves-finales-union-garde-target.md`,
APP exact `9ec421069320b2f0cca4099fa27f83059e944de9`.
Archives relues directement sur le portable :
`~/.cache/rag3weaver-build/cx-preuves-garde-target-union-v2-final/client/features.log`
avec VERT sans-feature, VERT daemon, CHECK_FEATURES VERT 8/8 ; journaux
`client/features/sans-feature.log` et `daemon.log` terminés avec succès,
18,97 s et 11,28 s. Manifeste publié dans
`docs/recherche/13-11-2026/garde-target-union-final-preuves/manifest.json`.
Ces archives appartiennent à la qualification Recherche ; ne pas les nettoyer.

Contrôles réellement joués par tty : lecture sources et journaux, garde tickets
VERT 410 en-têtes (125 ouverts,285 fermés), `git diff --check` propre.
NON JOUÉS par tty : Cargo, rustc, tests Rust, e2e, navigateur, mesures et GPU.
Le rouge historique minimal rc101/112,69s du 12 novembre est conservé dans le
ticket ; aucun test n'y avait tourné. Les réussites ultérieures ne le réécrivent pas.

## Ressources et contraintes

Aucun service, processus de calcul, verrou poste ni target acquis par tty
pendant cette reprise. Aucun nettoyage effectué ni à poursuivre implicitement.
Worktree documentaire et preuves préservés. Aucun arrêt d'un processus tiers.

Ne pas modifier l'arbre de Lucie ni ses sous-modules. Branches et worktrees
neufs préfixés cx-. Identité git Lucie Defraiteur, adresse Gmail personnelle ;
aucune attribution ajoutée. Aucun amend publié, force-push ni suppression distante.
Avant tout retrait futur : procédure courante, archives administratives fidèles,
garde_worktree fraîche verte et quiescence ; ne pas improviser un nettoyage.
Tout nouveau travail doit avoir un ticket unique ; celui-ci n'autorise aucun lot.
Les propositions historiques tty-4, tty-5 et thèmes attendent un objectif établi.

## Trois prochains pas à la reprise

1. Lire cette passation et le ticket fermé, puis relire les règles courantes :
   `git fetch origin master cx-docs-tty-features`
   et `git log -3 origin/cx-docs-tty-features`.
2. Revoir le lot documentaire complet, sans compilation :
   `git diff origin/master...origin/cx-docs-tty-features -- docs/tty docs/tickets`
   puis, dans le worktree, `python3 extension/rag3weaver/scripts/check_tickets.py --lot origin/master`.
3. Recontacter l'orchestration pour le statut d'intégration du lot complet et
   son nouveau cap. Aucun push master ni reprise automatique d'un ancien délai.
   Si aucun objectif n'est donné, rester en veille ; ne pas créer de travail.
