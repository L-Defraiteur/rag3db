# Rapport de session « rag3weaver archi », 11 novembre 2026

Cette page est écrite pour un agent qui reprend le périmètre sans mémoire de nos échanges.
**archi** est le nom de cette session de travail : elle tient l'IDE web de rag3weaver, l'atelier
et les hooks de Claude Code. Elle est orchestrée par une autre session, **c7** (qui distribue les
lots, donne le feu pour pousser et relit), aux côtés d'**ED** (la session « everything
declarative », qui relit le code), de **G** (la session « optimiseur », qui pose les binaires sur
les postes) et de **96** (la session « mémoire »).

Un **lot** est une unité de travail : une branche, ses témoins, sa passe, son push. Un **témoin**
est un test qui prouve une promesse : on l'écrit d'abord, on le voit échouer (« rouge »), puis on
corrige jusqu'à ce qu'il passe (« vert »). Une **passe** est la suite de vérifications jouée avant
un push. Le procédé est décrit dans la connaissance, page `02-knowledge-dump.md`.

## Les lots atterris sur master

Tous sont poussés sur `origin/master` du dépôt rag3db, entre le 9 novembre au matin et le
9 novembre au soir.

| hash | ce que fait le lot | branche d'origine |
|---|---|---|
| `8efd3eeb0` | un délai sur chaque client HTTP des tests de l'hôte ; ferme le ticket du 8 novembre « un test de l'hôte attend son hôte par un client sans délai », ouvre celui des cinq clients de production | `delai-des-temoins-de-l-hote` |
| `d23af9cd2` | l'atelier lit et écrit : `read_file`, `read_scope`, `replace_text`, `replace_lines`, `insert_lines`, `ask_permission`, la carte d'accord, le journal des décisions | `atelier-lire-ecrire` |
| `b3053bd51` | trois lignes pour Lucie (« À essayer dans ton atelier ») en tête de la page de l'atelier | docs |
| `be319fa75` | une commande mise en attente se décide sur la carte d'accord, et la décision relance le tour ; `accorder_ligne` n'accorde plus que la commande exacte (ticket fermé dans le lot) | `atelier-commandes-sur-la-carte` |
| `3e8c87aa5` | la carte d'un chemin relance aussi son tour | `atelier-chemin-relance` |
| `ab8272deb` | la jauge du contexte écrit son pourcentage, suit chaque appel au modèle, et survit au redémarrage de l'hôte | `jauge-du-contexte` |

Les lots plus anciens de cette session (les hooks de Claude Code, le 7 et le 8 novembre) sont
décrits dans `extension/rag3weaver/docs/07-novembre-2026-hooks-claude-code/`.

G a posé `d23af9cd2` sur le portable le 9 novembre à 13 h 42. Les lots suivants
(`be319fa75`, `3e8c87aa5`, `ab8272deb`) ne sont **pas encore posés** chez Lucie. c7 voulait les
regrouper avec le lot de la session « recherche ».

## Le lot en branche : l'IDE par le flux (reprise du 11 novembre)

- Branche de référence : `origin/cx-archi-ide-par-le-flux-2`, code `61066a2b3` ;
  rebasée après le garde-fou `1904e04e4` et le correctif mémoire `41526b29a`.
- Worktree sur les deux postes : `/home/lucied/.cache/rag3weaver-build/cx-wt-flux`.
  Le target de validation est `~/.cache/rag3weaver-build/flux` sur luciepc.
- Les anciens worktrees `wt-flux` et branches `ide-par-le-flux` /
  `archi-flux-validation-20261111` sont conservés ; aucune branche distante supprimée.
- La demande reste un IDE inactif sans requêtes, et un geste de chat qui ne produit que son
  `POST /ide/chat` ; états et fin du tour arrivent par le flux `etat`.
- La cause historique du code 137 est **inconnue** : pas de preuve OOM/SIGKILL retrouvée
  dans les journaux noyau consultés (9 novembre, 14 h 50–15 h 30). Aucun processus du target
  sur le portable ou luciepc n'était actif avant la reprise. L'ancien dossier `flux-passe`
  ne contient toujours qu'un journal et une compilation interrompue, sans verdict.

### Preuves et correction du compteur

Sur `a9c5a3108`, témoin flux vert (1/1, 67,85 s), puis à luciepc : features 5/5,
lib entière 2 094/0 en 56,02 s, hooks 2/2. Le premier e2e lectures a rougi faute de service
aux ports du portable ; avec les adresses luciepc `7878,7879,7880`, il passe 24/24.
Le navigateur complet reste rouge : 33/34, puis **5 rouges sur 5** du témoin
`la_barre_dit_l_usage_le_recapitulatif_et_l_archiviste` : la barre rend 497 jetons,
GET chat rend 0. Le compteur était ajouté seulement au message du flux ; GET attendait
encore le fichier asynchrone du journal.

La correction scelle `usage_du_chat` dans les ajouts du même journal à la fin du tour.
GET, flux et récapitulatif lisent la même projection : instantané des ajouts et bornes
avant liste des fichiers, déduplication par run, total scellé prioritaire sur `Consumed`.
Les rôles finis se relisent également avant leur fichier. L'ordre vient de l'identité du
run, avant les bornes de lecture, pas de l'ordre d'écriture des fichiers. Aucun polling,
aucun nouveau stockage. Un résumé absent n'est pas gardé en cache négatif.

Sur `a3a00ef75` : 16 tests chat hors navigateur verts (12 ignorés, joués à part),
puis **5 verts sur 5** du même témoin navigateur. Les preuves ajoutées couvrent deux tours,
leur transfert concurrent en ordre inverse, les ajouts tardifs, les anciennes traces et la
relecture par une nouvelle instance **après écriture** ; elles ne promettent pas la durabilité
lors d'un arrêt brutal avant l'écriture asynchrone du journal.

`061b0a483` ajoute l'ordre commun avant les bornes. Sa passe complète est **ROUGE lib** :
2 097 tests réussis, un échec, 59 ignorés, 145,09 s. Le témoin
`owed_vectors_are_caught_up_on_open_when_the_manifest_declares_it` expire après 120 s avec
606 morceaux encore dus. Les trois nouvelles preuves du chat passent dans cette lib.
Les étapes e2e et navigateur suivantes n'ont pas été jouées par ce lanceur.
ED tient le correctif des réclamations d'index en branche, pas encore sur master :
`bbba12fb2` sur `cx-ed-reclamations-rendues-5`. Sa piste explique comment des claims non
rendus peuvent laisser une dette après réouverture ; la cause de notre run n'est pas
prouvée. Aucun changement au test ni à l'index dans le lot flux. La lecture des piles a été
refusée par le système ; refus conservé, aucun contournement.

La passe rouge est conservée, sous `poste lourd` sur luciepc :

- lanceur `~/.cache/rag3weaver-build/cx-flux-complete-061b0a483.sh` ;
- sorties `~/.cache/rag3weaver-build/cx-flux-complete-061b0a483-20261111/` ;
- lib épinglée `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF` ;
- `CARGO_BUILD_JOBS=8`, paquets React exigés ; services luciepc `7878,7879,7880` ;
- étapes : check_features, script hooks, lib large, e2e hooks, e2e lectures, navigateurs.

Après rebase mémoire, `61066a2b3` est sauvegardé sous le nouveau nom `cx-archi-ide-par-le-flux-2`.
Un lanceur distinct vérifie les étapes hors lib (sans transformer le rouge en vert global) :
`~/.cache/rag3weaver-build/cx-flux-61066a2b3-hors-lib.sh`, sorties
`~/.cache/rag3weaver-build/cx-flux-61066a2b3-hors-lib-20261111/`.
Résultat de ces contrôles hors lib sur `61066a2b3` : features 5/5, script hooks vert,
e2e hooks 2/2 (8,07 s), e2e lectures 24/24 (11,22 s), navigateurs 34/34 (136,22 s),
zéro ignoré dans ces deux e2e et dans l'étape navigateur. Les deux suites e2e annoncent
chacune « 1 sur 1 demandées ». Verdict conservé :
`VERT hors-lib (lib bloquante non rejouée)` — **pas un verdict global vert**.
Tous les processus de ce lanceur ont fini.
Rejeu global requis après traitement du blocage ED et incorporation du master du moment.

Les journaux antérieurs restent distincts : `flux-passe-complete-20261111-luciepc-2`,
`flux-passe-reprise-services-20261111-luciepc`, `flux-barre-seul-20261111-{1..5}`,
`cx-flux-correction-20261111-*`, `cx-flux-correction-r2-20261111-*`.

### Reprendre

Lire le processus et le journal de la passe active avant toute commande cargo. Un seul cargo
par target ; ne pas déplacer un worktree actif. Après un résultat global vert, demander le
tour à l'orchestration. Toute arrivée de code impose rebase sur une nouvelle branche `cx-`
et rejeu court hors verrou de push ; les docs seules n'imposent pas de rejeu.
**Aucun tour de push master accordé.** L'ancien `wt-flux-push.sh` ne correspond plus aux chemins
ni aux verdicts courants : préparer et relire un lanceur conforme avant de l'utiliser.

## Lot suivant : garder les actions du chat après une panne

Branche `cx-archi-chat-erreurs`, worktree `cx-wt-chat-erreurs` sur les deux postes,
code `816fc4824`, dépendant du lot flux (pas encore sur master). Le premier blocage UI est
corrigé et rouge/vert prouvé : une erreur modèle après lecture ne supprime plus les actions,
et l'issue réelle du tour est portée par l'état poussé, y compris hors formulaire.
L'inventaire effectif atelier/dépôt/tâche est vert depuis le navigateur ; le trou de recherche
et de suivi de commande de l'atelier est transmis à ED en ticket. Types et bundle vérifiés.
Régression navigateur en cours ; aucun verdict global, aucun push master.
Détails, preuves, matrice et limites : `03-inventaire-parcours-chat.md`.

## Les lots nommés, pas commencés

- **Le découpage au survol de la jauge du contexte.** Au survol, la jauge dirait ce qui pèse
  dans le dernier appel au modèle : les instructions (le message système), les définitions des
  outils, l'historique, et dans l'historique les résultats d'outils. C'est estimé au prorata des
  caractères, puisque le fournisseur ne rend qu'un total. Il est nommé dans
  `extension/rag3weaver/docs/09-novembre-2026-atelier-lire-ecrire/02-la-jauge-du-contexte.md`.
  Ce raffinement passe désormais après le parcours de code complet dans le chat.
- **Les propositions et le fichier ouvert par le flux.** C'est le même moule que l'IDE par le
  flux, nommé dans sa page.

## Les pages de conception en attente

- `extension/rag3weaver/docs/09-novembre-2026-atelier-lire-ecrire/01-conception.md` (l'atelier lit
  et écrit, puis les commandes et les chemins sur la carte). Tout est sur master. La relecture
  d'ED se fera sur master : ED était bloqué depuis le 9 au matin sur une approbation de Lucie.
  Ses remarques feront un lot de suite. Les trois points que je lui ai soumis :
  - `accorder_ligne` resserré change aussi le oui du superviseur d'un équipier ;
  - la relance du tour est un message d'utilisatrice marqué « (carte d'accord) » ;
  - `Chat.tsx` suit désormais un tour qu'il n'a pas envoyé.
- `extension/rag3weaver/docs/09-novembre-2026-atelier-lire-ecrire/02-la-jauge-du-contexte.md`.
- `extension/rag3weaver/docs/09-novembre-2026-ide-par-le-flux/01-inventaire-et-forme.md`.

## Ce qui reste sur disque

Rien n'a été effacé pour ce passage.

- **Portable**, sous `/home/lucied/.cache/rag3weaver-build/` :
  - `wt-flux` : le worktree de la branche `ide-par-le-flux`, propre, à `a9c5a3108` ;
  - `flux` : son target cargo, 11 Gio ;
  - `wt-docs-archi` : mon worktree de documentation, sans branche (tête détachée) ;
  - `wt-flux-passe.sh` et `wt-flux-push.sh` : les scripts de la passe et du push du lot ;
  - les scripts `wt-*-passe.sh` et `wt-*-push.sh` des lots atterris, que l'on peut copier comme
    modèles.
  - Les autres worktrees de ce dossier (`wt-deplace`, `wt-docs-hooks-essai`, `wt-fuite3`,
    `wt-gabarits`, `wt-juge`, `wt-mf-96`, `wt-react5`) appartiennent à d'autres sessions.
- **Branches sur origin** laissées par mes lots, déjà fusionnées et à ne pas supprimer sans le
  mot de Lucie : `atelier-lire-ecrire` et `-2`, `delai-des-temoins-de-l-hote` et `-2`,
  `atelier-commandes-sur-la-carte` et `-2`, `atelier-chemin-relance`, `jauge-du-contexte`.
  Un suffixe `-2` est la branche rebasée, poussée sous un nom neuf parce qu'on ne force jamais
  un push.
- **luciepc** (l'autre poste) : aucun worktree ni target de cette session. Celui qui s'y trouve,
  `wt-lib-arbre`, n'est pas à moi.

## Ce qui est bloqué

- La relecture d'ED attend une approbation de Lucie.
- Le rouge connu de la CI de master,
  `daemon_host::worktrees::tests::owed_vectors_are_caught_up_on_open_when_the_manifest_declares_it`,
  attend aussi Lucie : le correctif est chez ED. Ce n'est pas à moi.

## Les tickets ouverts qui me concernent

Ils sont sous `docs/rag3weaver-archi/tickets/`, déplacés de `docs/tickets/` le 11 novembre.

- `2026-11-09-cinq-clients-de-production-attendent-sans-delai.md` : cinq clients HTTP hors des
  tests attendent un hôte sans délai. Il attend une décision de conception de Lucie.
- `2026-11-08-le-juge-du-rappel-coupe-sa-reponse-sous-400-jetons.md` : le juge du rappel des
  hooks a un plafond de 400 jetons, que la réflexion de Gemini consomme ; la réponse est coupée.
- `2026-11-07-search-structured-le-score-d-une-ligne-n-est-pas-comparable-d-un-signal-a-l-autre.md` :
  reste le seuil `recall_min_score` du hook, à mesurer.
- `2026-11-07-un-test-de-la-lib-qui-parle-au-service-d-embarquement-attend-sans-delai.md` :
  relevé par moi, sans porteur.

## Les trois prochains pas que je ferais

1. **Finir l'IDE par le flux.** Conserver le rouge 606 dus, coordonner le blocage avec ED,
   finir les contrôles hors lib, puis rebase et rejeu requis avant un éventuel tour de push.
2. **Le parcours réel du chat IDE.** Atelier, dépôt et tâche : outils réellement accessibles,
   recherche, lecture, édition, commandes/tests, sorties/diff, accords, arrêt et reprise.
   ED tient outils/gabarits et clôture des tâches ; archi tient navigateur/UI/flux/cartes.
3. Les raffinements de la jauge et les autres vues par flux viennent après ce parcours,
   conformément à `docs/orchestration/11-11-2026/05-cap-produit-vues-et-chat.md`.
