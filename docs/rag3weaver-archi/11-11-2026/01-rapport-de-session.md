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

## Lot commun prêt : l'état fidèle du chat par le flux

- Branche de référence : `origin/cx-archi-ide-par-le-flux-chat-4`.
- Source exactement jouée : `bc3a1d7797856abf89ef838e6720622ea5c393aa`, rebasée sur
  projection `6c4b5ad0c0f757ee17905c1643dd8b39174ccaa9`, après ED réclamations `91e5f760b`.
- Worktree sur les deux postes : `~/.cache/rag3weaver-build/cx-wt-chat-erreurs` ;
  target de validation à luciepc : `~/.cache/rag3weaver-build/flux`.
- **Passe complète VERT**, terminée à luciepc ; aucun processus restant du lanceur.
  **Pas encore de push master : tour à obtenir de l'orchestration.**

Le lot réunit les états poussés du chat et leur fidélité : aucun polling au repos dans le
témoin, compteur commun pour flux/GET/récapitulatif, actions conservées après une panne et
issue réelle du tour publiée, y compris quand il part hors du formulaire. Aucun outil ni
capacité de recherche ajouté. L'inventaire navigateur atelier/dépôt/tâche est documenté dans
`03-inventaire-parcours-chat.md` ; le manque de recherche et de suivi de commande de l'atelier
reste un ticket ED, pas une livraison de ce lot.

### Passe finale et provenance

Dossier : `~/.cache/rag3weaver-build/cx-flux-chat-bc3a1d779-prive-r2-20261111/`.
Lanceurs : `cx-flux-chat-bc3a1d779-prive-r2-{passe,push}.sh`, sous le même dossier de build
à luciepc. Le push n'a pas été exécuté. `source-head`, `master-au-depart`, `lib-epinglee`
et `postgres-identite.txt` attestent la source, la base, la lib et l'identité SQL.
Le master vu au départ est `b798a299e86091033d15fbcb2491088734611806` (docs après projection).

| étape | résultat réel |
|---|---|
| `check_features` | VERT 5/5 |
| script d'installation des hooks | VERT |
| lib, jeu large | 2 100 réussis, 0 échec, 61 ignorés, 39,07 s |
| `e2e_hooks_claude_code` | 2/2, 0 ignoré, 7,59 s, 1 suite sur 1 |
| `e2e_lectures` | 24/24, 0 ignoré, 9,79 s, 1 suite sur 1 |
| `e2e_postgres` | 34/34, 0 ignoré, 7,84 s, 1 suite sur 1 |
| navigateurs | 36/36, 0 ignoré, 132,10 s |
| TypeScript et bundle `--check` | VERT, vérifiés après installation des dépendances |

Les 61 ignorés de la lib sont comptés comme ignorés, pas ajoutés aux tests réussis ;
l'étape navigateur joue ses témoins séparément. Aucun `SAUTÉ` dans les trois e2e.
Le jeu large est `rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm`.
Lib : `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.
`CARGO_BUILD_JOBS=8`, services luciepc `7878,7879,7880`, paquets React exigés.
PostgreSQL privé : `rw_archi_flux_chat_bc3a_20261111`, compte `rag3weaver` ; contrôle initial
une connexion, URL et identifiants non affichés. Ne pas jouer `e2e_postgres` sur la base partagée.

### Les rouges et leurs preuves restent conservés

- Code 137 historique du 9 novembre : **cause inconnue** ; aucune preuve OOM/SIGKILL retrouvée
  dans la fenêtre consultée. `flux-passe` n'a pas de verdict ; aucun doublon local/distant
  au début de la reprise. La tentative de piles de la lib du 11 a été refusée ; aucun contournement.
- `a9c5a3108` : navigateur 33/34, puis compteur rouge **5/5** (barre 497, GET 0).
  Le total était ajouté seulement à la trame. Correction : `usage_du_chat` scellé dans les
  ajouts du journal à la fin du tour ; projection commune pending/durable dédupliquée par run,
  total scellé prioritaire sur `Consumed`, ordre logique avant bornes, pas de cache négatif.
  Les rôles finis se relisent avant leur fichier. Ciblé passé **5/5**, puis régression globale.
- `061b0a483` : lib rouge 2 097/1, 145,09 s ; rattrapage expire à 606 vecteurs dus après 120 s.
  Après intégration ED, **ce témoin précis est OK** dans les deux libs sur `bc3a1d779` :
  38,89 s puis 39,07 s pour la suite. L'absence du défaut sur la base corrigée ne démontre
  pas rétroactivement la cause exacte de l'ancien run.
- Premier lancement commun interrompu à `check_features`, avant tests SQL, pour remplacer
  la base partagée par une base privée ; répertoire `cx-flux-chat-bc3a1d779-20261111`.
- Première complète privée : features, lib 2 100/0 (38,89 s) et hooks 2/2 verts, puis refus
  avant lectures car target 34 Go > 32. Verdict rouge conservé dans
  `cx-flux-chat-bc3a1d779-prive-20261111`. Mesure : 22 Go de caches incrémentaux.
  Aucun processus target avant nettoyage des seuls `debug/incremental` ; target ramené à
  13 Go, puis 14 Go pendant la passe finale. Aucun plafond relevé. Anciennes preuves externes
  intactes. La complète r2 était déjà lancée lors de la précision permettant une reprise
  des seules étapes manquantes ; ce rejeu supplémentaire est déclaré, pas caché.
- La première vérification TypeScript a croisé `npm ci` et échoué sur dépendances absentes.
  Rejouée séquentiellement après installation : VERT ; aucun défaut source masqué.

Anciennes branches et worktrees conservés : `ide-par-le-flux`, `cx-archi-ide-par-le-flux`
et suffixes `-2`, `-3`, `cx-archi-chat-erreurs`. Aucun force-push ni suppression distante.
Les anciens journaux détaillés restent sous les noms consignés dans l'historique du rapport.

### Reprendre / intégrer

Avant toute nouvelle commande cargo, vérifier les processus et le propriétaire du target.
La passe finale est terminée ; ne pas la rejouer sans changement ou nouvelle inquiétude.
Le script de push exige VERT, vérifie les changements depuis la source jouée et les arrivées
sur master, refuse un rejeu sous le verrou et sauvegarde tout rebase sous un nouveau nom.
Il ne s'exécute qu'au tour accordé, puis lit les trois contrôles et nettoie son worktree/target,
en conservant tous les journaux. Les modifications de rapport après la passe sont des docs seules.
Après push, consigner le hash et la nouveauté réellement livrée ; prévenir orchestration et ED.

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

## Ce qui reste en attente

Le lot commun n'a plus de rouge de validation : le rattrapage historique est vert sur la base
ED. Il attend uniquement son tour d'intégration. Les anciens signalements du 9 novembre
(approbation de relecture ED, rouge CI du rattrapage) sont historiques ; ne pas les reprendre
comme état actuel sans vérification. Les capacités encore absentes de l'atelier ont leur ticket
et leur futur lot ED, distincts de la livraison du flux et des erreurs.

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

1. **Intégrer le lot commun flux/chat**, passe complète verte ci-dessus, au tour de l'orchestration.
   Le rouge historique 606 dus reste conservé ; le témoin est maintenant vert sur la base ED.
2. **Le parcours réel du chat IDE.** Atelier, dépôt et tâche : outils réellement accessibles,
   recherche, lecture, édition, commandes/tests, sorties/diff, accords, arrêt et reprise.
   ED tient outils/gabarits et clôture des tâches ; archi tient navigateur/UI/flux/cartes.
3. Les raffinements de la jauge et les autres vues par flux viennent après ce parcours,
   conformément à `docs/orchestration/11-11-2026/05-cap-produit-vues-et-chat.md`.
