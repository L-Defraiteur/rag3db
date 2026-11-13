# Passation week-end — hotfixs du cœur C++, 13 novembre 2026

## Instruction de reprise prioritaire

Passation demandée par Lucie pour limite d’usage. **Aucun nouveau calcul, build,
run, chargement de modèle, nettoyage ou push master autorisé par ce document.**
Les anciens délais et accords one-shot NE SE RÉACTIVENT PAS au week-end.
Commencer par la lecture ; demander le périmètre suivant à l’orchestration.
L’arbre de Lucie `/home/lucied/git_workspaces/rag3db` n’a pas été modifié ici.
Ses modifications de sous-module et son dossier non suivi restent à elle.

## Livré et conclusion à conserver

1. Les deux rejeux exacts des relations sous la lib corrigée complète
   `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`
   ont été vus verts : d796, 797 lignes/31 écritures/0 erreur ; item2,
   2 lignes/1 écriture/0 erreur ; aucun écart des deux sens et codes0.
   Résultat transmis au cœur pour son ticket d’index complet. Ne pas annoncer
   un nouvel index utilisateur réalisé : il s’agissait de copies privées.
2. Diagnostic de quarantaine documenté, puis témoin structurel privé préparé,
   compilé et exécuté dans les bornes expressément relues. Résultat : après
   annulation partielle de TA et libération de ses verrous, BEGIN/CHECKPOINT
   publics refusent nommément la base ; le COMMIT forcé de TB déjà ouverte
   atteint pourtant une fois la fabrique interne de checkpoint.
3. La sentinelle interrompt AVANT construction de Checkpointer. **Aucun
   checkpoint réel, réouverture, preuve de corruption ou innocuité générale.**
   Le code0 signifie diagnostic observé, pas quarantaine correcte.
4. Le cœur a relu et consolidé les docs/preuves sur master ; les sources et
   scripts privés du témoin restent sur leur branche, sans intégration moteur.
   Revue à lire :
   `docs/coeur-cpp/12-11-2026/21-relecture-quarantaine-et-suite-sur-papier.md`.
   Point essentiel : TB traverse sa publication avant la fabrique ; interdire
   uniquement l’écriture de checkpoint ne prouverait pas un refus avant publication.
5. Service privé7882 : usage transmis à codeparsers, puis rendu ; arrêts/retours
   du banc opérés par optimiseur, ensuite arrêt distinct pour audit de résidence.
   Il est ARRÊTÉ, sans restauration automatique ; ne pas utiliser ses anciens PID.

## WIP possédé, branches distantes et worktrees

Dépôt distant `origin` : `L-Defraiteur/rag3db`. Les deux branches récentes ont été
relues avec `git ls-remote` pendant cette passation ; leurs worktrees sont propres.

| Poste / worktree | Branche distante | SHA complet sauvegardé | État |
|---|---|---|---|
| portable `/home/lucied/git_workspaces/cx-hotfixs-temoin-quarantaine` | `origin/cx-hotfixs-temoin-quarantaine` | `e289163bfc3055421fda47cf0a370cd46d376f17` | Source test, scripts, preuves et rapport ; propre ; pas de nouveau run |
| portable `/home/lucied/git_workspaces/cx-hotfixs-recette-quarantaine` | `origin/cx-hotfixs-recette-quarantaine` | `32fc739a8e0ff9e4948dfc83ee9980b46f20b635` | Plan général antérieur, propre ; aucune variante autorisée |
| portable `/home/lucied/git_workspaces/cx-hotfixs-passation-week-end` | `origin/cx-hotfixs-passation-week-end-20261113` | commit contenant ce rapport, à relever avec `git rev-parse HEAD` | Branche de passation neuve depuis master `3a5080d93bdaeba05fd333b25bdd0565b49ad18b`, sans code |

Ne pas rebaser ces worktrees de preuve, ne pas y modifier la source compilée.
Le lot récent n’a pas de worktree Git propre à hotfixs sur luciepc : compilation
standalone dans les stagings privés ci-dessous. Les sources moteur0511 et
l’archive appartiennent au cœur ; ne pas y intervenir.

### Lots anciens hérités, non repris par cette session

Leur propreté locale et les refs distantes ci-dessous ont été relues pendant la
passation. Ils sont déjà sauvegardés, pas de code local non poussé identifié.
Le rapport du 11 novembre décrit leurs motivations ; ses états de branches
peuvent être anciens : les valeurs suivantes sont celles relues aujourd’hui.

| Worktree portable | Ref distante | SHA complet relu |
|---|---|---|
| `~/git_workspaces/rag3db-verrou` | `origin/versement-verrou-2` | `4510f4631a5b38d508fb8d81dec2252366883b5c` |
| `~/git_workspaces/rag3db-g5` | `origin/g5-statistiques` | `e43ebb920649dc031a51fd60b848ea15c8deb522` |
| `~/git_workspaces/rag3db-tmpb` (détaché) | `origin/agent/tests-bases-dans-tmp` | `fe16dad5b921ae9574d705ad758d44bc920752c4` |
| `~/git_workspaces/rag3db-reindex` | `origin/temoin-deux-sens-reprise` | `dfc0ec8b34e2dd25b9a6746c002ff86de54683a0` |
| `~/git_workspaces/rag3db-etude` (détaché) | `origin/trace-exacte-9bae` | `2b73c4ff77e07bad24ca8c50a72d72591099c260` |

Autre ref historique relue : `origin/reindex-complet-temoin` à
`da1f9b1b6c0e66732653aba8224229e70cd2519e` ; cinq tests voisins jamais rouges,
pas une preuve du défaut corrigé. Rien à atterrir automatiquement.

À luciepc, contrôles lecture seule :
- `~/git_workspaces/rag3db-dbg` à `dfc0ec8b34e2dd25b9a6746c002ff86de54683a0` ;
  seuls non-suivis observés : `follows.csv`, `user.csv`, `user.parquet`, déjà
  mentionnés par le rapport historique ; laissés en place, pas du code WIP poussé.
- `~/git_workspaces/rag3db-equipier` à `07050ff83c1eb399cefb6e7136d8aa4c1dc75889`,
  status propre ; chantier historique partagé, aucune prise en charge nouvelle.

Les autres worktrees anciens sont préservés ; aucune suppression/prune/GC.

## Tickets et porteurs

Ticket actif de ce chantier, **OUVERT**, hotfixs avec cœur C++ :
`docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-12-verifier-la-quarantaine-d-une-annulation-partielle-pour-les-transactions-deja-ouvertes.md`.
Source du constat et revue du contrat dans ce ticket ; ne pas dupliquer.
Lié mais distinct du ticket du cœur
`docs/coeur-cpp/tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md`.
Le regroupement de plages INSERT_INFO a été retiré comme piste pour TwentyWriters
par le cœur ; pas d’opt-in NodeTable validé. Ne pas reprendre cette optimisation
comme si elle avait été prouvée ou autorisée.

Autres tickets hotfixs encore à lire, aucun traitement nouveau ici :
- `docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-08-le-create-vector-index-concurrent-lit-une-ligne-au-dela-de-la-colonne-en-cache.md` ;
- `docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-08-le-nombre-de-lignes-hors-du-graphe-d-un-index-vectoriel-lisible-sans-ecrire.md` ;
- `docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-07-un-rang-deplace-pendant-un-create-n-est-pas-toujours-rattrape-premier.md` ;
- `docs/coeur-cpp-hotfixs-tickets/tickets/2026-10-25-la-ci-de-master-est-rouge-depuis-le-18-octobre-cassert-dans-geo-et-format-clang.md` ;
- `docs/coeur-cpp-hotfixs-tickets/tickets/2026-10-10-ouverture-sans-une-extension-dont-une-table-depend.md`.

Coordination7882 : ticket banc dix fiches
`docs/banc/tickets/2026-11-11-mesurer-le-juge-local-avec-dix-fiches.md` ;
audit résidence terminé et clos dans
`docs/tickets/closed/2026-11-12-revoir-les-services-d-embarquement-residents.md`.
L’arrêt d’audit n’est pas un échec de restauration du banc.

## Preuves, erreurs conservées et NON JOUÉS

Docs/preuves disponibles sur `origin/cx-hotfixs-temoin-quarantaine` :
`docs/coeur-cpp-hotfixs-tickets/12-11-2026/04-resultat-structurel-quarantaine.md`,
`preuves/compile-01`, `preuves/compile-02`, `preuves/compile-03`,
`preuves/runtime-01`, `preuves/runtime-02`.

- Compile01 ROUGE : namespace `common` ambigu et `visible_func` non qualifié ;
  correction de qualification seule autorisée, puis compilation02/lien/ELF conformes.
- Runtime01 INCONCLUSIF/échec : maxDBSize `-1ull` refusé avant toute requête.
  Correction exclusive vers sentinelle `-1u` relue/autorisée, compilation03 réussie.
- Runtime02 : `internal-path-reached`, enfant/parent/wrapper0, un seul nouvel
  essai autorisé, callback one-shot/identité/offset attendus, TA inactive/locks0,
  contrôles publics refusés, fabrique interne1. Ce n’est pas un test prouvant la
  correction du moteur et ne doit pas être rebaptisé « quarantaine verte ».
- NON JOUÉS : checkpoint réel, réouverture/durabilité, lecteurs, auto/différé,
  verrous gardés et attentes synchronisées, correction produit/passe complète.
- Les passes des lots historiques B/G5/temporaires ne sont pas relancées et ne
  sont pas déclarées vertes par cette passation.

Luciepc, tous les chemins suivants existent encore (lecture seule pendant la
passation), sous `~/.cache/rag3db-hotfixs-notes/` :
- `cx-quarantaine-0511-structurel/` : première compilation refusée ;
- `cx-quarantaine-0511-structurel-02/` : compilation02, premier runtime refusé ;
- `cx-quarantaine-0511-structurel-03/` : compilation03 et runtime observé,
  base conservée dans `runtime-aymvkbea/artifacts/released-az2teC/`.

Empreintes runtime02 :
- source `14d99084010e0b8f5f530f2faa5926ca724382d44ab9deca24a31545baf71afd` ;
- binaire `37ccee63d54555552a4541bf6f2aa8f55c1b97210e0b1d43f1332a1c28dcddd7` ;
- rapport `9f780e3f07427d65bb6eb1acc21a0b583b2e6785b1c40b05e9ef70faf28a4432` ;
- lanceur `a110d161f027b67f2464112058fde9250c1ebf801bbe0b2b557d888c21fb1e6f`.
Les anciens JSON approved_to_run=true sont des reçus d’exécutions consommées,
PAS des accords réutilisables. Le binaire ne se lance pas pour vérifier qu’il existe.

Preuves de rejeu des deux sens à luciepc :
`~/.cache/rag3db-tickets-notes/reprise-codex-11nov-29a7/{d796,item2}.log`.
Les bases et traces historiques restent à leurs chemins du rapport du11nov ;
aucune suppression ni nouvelle copie de base en passation.

## Ressources et contraintes durables

Aucun calcul ou run actif pour ce chantier ; lecture de processus pendant la
passation : aucun programme des stagings de diagnostic observé. Les preuves de
fin du dernier essai donnent enfant récolté, cgroup supprimé, unité inactive/dead,
MainPID0. Aucun verrou lourd/mesure réservé à reprendre par cette session.
Ne pas toucher aux unités/processus des autres sessions.

7882 ne présente plus de port à l’écoute au relevé de passation. Recette conservée
par optimiseur dans `~/.cache/optimiseur-notes/audit-residence-12nov/recette-retour.json`
(selon son message de fin d’audit ; pas relue ici). **Pas de retour automatique.**
Dernier état communiqué par orchestration : 278m7878 et BGE/relecteur/OCR7879 chauds ;
107m7880 arrêté, instance disabled mais pas masked, démarrage manuel possible
quand un travail autorisé exige ce modèle, sans substitution du modèle d’index.
Ne pas déduire une autorisation de charger un modèle pour cette passation.

Card2 est une ressource de travail normale pour les travaux autorisés : coordination
sur capacité/VRAM/consommateurs, pas quota arbitraire ; préserver seat1. Une mesure
comparative conserve sa fenêtre MESURE ; les appels ordinaires ne demandent pas
une exclusivité GPU générale. Le modèle/version/réglage d’un index ne se substitue pas.

Identité git Lucie gmail, aucune attribution ; nouveaux noms branches/worktrees
`cx-`. Pas de force-push ni suppression distante. Tout nouveau travail complète
un ticket avant transmission. Master attend procédure et mot d’orchestration,
mais **aucun push master pour la présente passation**, même docs seules.
Conserver les worktrees, paquets, sources et bases privées ; avant retrait futur,
archive fidèle des administrations hors cible et garde commune VERT, sans forcer
un refus ni ignorer les dépendances de sous-module.

## Trois prochains pas, lecture d’abord

1. Relire le ticket actif, résultat04 et revue cœur21 sur master du moment ;
   confirmer avec l’orchestration le contrat refus AVANT publication et les
   départs/réveils après quarantaine. Aucun patch natif implicite.
2. Relire les refs WIP et leurs preuves depuis les worktrees conservés. Pour
   une suite autorisée, créer un nouveau `cx-` depuis la base choisie ; ne pas
   rebaser ou nettoyer un arbre de preuve. Rechoisir les artefacts/limites et
   accords liés aux nouveaux SHA si une exécution devient autorisée.
3. Faire choisir l’ordre par orchestration : contrat de quarantaine sur papier
   ou ancien lot B/G5/temporaires. Avant toute passe, revalider ticket, source,
   lib et accès ; les vieux scripts et délais ne donnent aucun GO automatique.

Commandes sûres de lecture/reprise documentaire :

```bash
cd /home/lucied/git_workspaces/cx-hotfixs-temoin-quarantaine
git status --short
git log -1 --format='%H %s'
git ls-remote origin refs/heads/cx-hotfixs-temoin-quarantaine refs/heads/cx-hotfixs-recette-quarantaine
cat docs/coeur-cpp-hotfixs-tickets/12-11-2026/04-resultat-structurel-quarantaine.md
# Revue cœur présente sur master actuel, pas nécessairement dans ce vieux WIP :
git show origin/master:docs/coeur-cpp/12-11-2026/21-relecture-quarantaine-et-suite-sur-papier.md
```

Les scripts runtime et scripts historiques de push ne figurent volontairement
pas comme commandes de reprise à exécuter : leur autorisation n’est pas acquise.
