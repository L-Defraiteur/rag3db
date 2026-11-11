# Le nettoyage du parent invalide un worktree de sous-module

- **État** : ouvert — incident après livraison require 2e910930d ; restauration effectuée sans preuve comparative initiale suffisante ; revue et garde à faire
- **Famille** : produit
- **Gravité** : risque de perte de références ou d'historique local ; intégrité avant/après non entièrement établie
- **Atteignable en service** : non ; nettoyage des worktrees de développement
- **Touche rag3weaver** : oui (procédure de nettoyage des lots ; aucun défaut fonctionnel require établi par cet incident)
- **Pour** : session codeparsers, revue avec l'orchestration
- **Signalé par** : orchestration après [l'attestation de nettoyage](../11-11-2026/01-rapport-de-session.md), correction de la qualification donnée en 1d347071a

## Constat et interventions déjà exécutées

À luciepc, après le push 2e910930d et ses trois contrôles, suppression de
rag3db-brancher par git worktree remove --force. L'ancien codeparsers-garde
était rattaché au dépôt du sous-module administré sous ce parent. Son accès
Git échoue ensuite : « gitfile ne pointe pas vers un dépôt valide ».

L'intervention déjà faite a déplacé son fichier .git dans les preuves, lancé
git init dans le répertoire existant, ajouté origin, puis tenté un fetch du
SHA court 672a0d5 (refusé par le serveur). Un fetch origin complet et
`git reset --mixed 672a0d5` ont ensuite réussi. Git status --short est vide.
Il n'y a pas eu de checkout/reset --hard, ni de suppression du répertoire
source. Ces commandes ne remplacent pas une comparaison avant/après.

Aucune nouvelle intervention dans le garde ni suppression de restes avant
revue. Cette consigne n'est pas une autorisation de lancer une nouvelle
réparation ou d'implémenter la garde ci-dessous.

## Preuves avant : disponibles et manquantes

Le rapport historique docs/codeparsers/11-11-2026/01-rapport-de-session.md,
section « Les autres worktrees de luciepc », mentionne codeparsers-garde au
commit 672a0d5, ancien clone, « rien n'y attend ». C'est un état de reprise,
pas un relevé d'intégrité juste avant le nettoyage.

Le status propre relu avant retrait concernait rag3db-brancher, pas son autre
worktree de sous-module. Aucun inventaire immédiat des fichiers/hash, de
l'ensemble des références locales, reflogs ou writers du garde n'a été
conservé avant suppression. **Absence de writer et identité des fichiers
avant/après non établies.** Aucun relevé git rev-parse --git-common-dir avant
retrait n'est disponible ; le lien administratif ci-dessous prouve le
rattachement, sans remplacer cet inventaire.

Le .gitfile sauvegardé pointe vers :
`/home/lucied/git_workspaces/rag3db/.git/worktrees/rag3db-brancher/modules/extension/rag3weaver/codeparsers/worktrees/codeparsers-garde`.
La suppression de l'administration du parent a donc atteint celle de cet
autre worktree. Les branches distantes du lot sont conservées et a730a9b est
accessible sur origin/cx-codeparsers-default-metadata ; cela ne prouve pas
que toutes les anciennes références locales du garde étaient publiées.
Pas de sauvegarde préalable de cet ancien dépôt administratif établie.

## Preuves après et limites

La commande reset --mixed vers 672a0d5 réussit ; les contrôles status ensuite
ne rendent aucune ligne. Cela établit la propreté Git actuelle vis-à-vis de
ce commit, pas l'identité avec tous les fichiers antérieurs (dont ignorés),
ni la conservation de références/reflogs locaux non publiés. Le clone
récupère l'historique et les références distantes par fetch origin ; faute
d'inventaire initial, leur équivalence à l'ancien dépôt reste inconnue.

Les artefacts suivants sont conservés sur luciepc dans
`~/.cache/codeparsers-notes/cx-passe-require-3/` :

| Artefact | SHA-256 | Portée |
|---|---|---|
| ancien-codeparsers-garde.gitfile | 27beb3a14d411bc6ae10e075371afd3f70fdf9e2f530918da5fc5ea64496412b | Ancien lien administratif, sauvegardé après invalidation. |
| reparation-ancien-codeparsers-garde.log | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 | Fichier vide : reset/status sans sortie, pas un inventaire ni une comparaison. |
| nettoyage-push.txt | c45e470aa1b6fd7ec2fd08c34aa22e5bd25962a9ae5de08fdde9496e21d0c80d | Liste des worktrees et ressources retirés. |
| controles-push.txt | 7b60b51a91795301952f6c8bbb00d4a05c3dc090692ec2bf70fe553528311b63 | Trois contrôles de livraison require ; distincts de l'intégrité du garde. |

Les sorties des commandes de réparation et contrôles sont aussi dans la
conversation de la session. Aucun hash initial des sources ne peut être
reconstruit honnêtement à partir du seul état final.

## Travail attendu et critères de fin

1. Revue de l'incident et des inconnues par l'orchestration, sans nouvelle
   intervention dans le garde avant ce cadrage ; déterminer si des preuves
   antérieures permettent de borner davantage les références ou fichiers.
2. Poser une garde de nettoyage avant tout prochain retrait de parent :
   inventorier les administrations et worktrees de ses sous-modules,
   détecter les liens vers un autre worktree, puis refuser le retrait tant
   que leurs données, états, références utiles et usages actifs ne sont pas
   vérifiés et préservés. Une propreté du seul parent ne suffit pas.
3. Définir et jouer un témoin parent/sous-module/autre worktree : retrait
   refusé avant invalidation ; les fichiers, HEAD et références de l'autre
   worktree restent accessibles. Choisir le mécanisme avec la procédure
   commune, sans effacement automatique de l'autre worktree.
4. Consigner le résultat réellement prouvé, garder les incertitudes non
   levées explicites et déplacer ce ticket dans closed seulement lorsque
   la revue et la garde sont traitées, ou avec une raison d'abandon explicite.

Ce ticket est distinct du ticket require corrigé et des tickets oracle et
homonyme. Aucun verdict fonctionnel ou LSP n'est requalifié par l'incident.

## Lecture ciblée et plan soumis avant code

Les rectifications 8df1266cd sont sur master, trois contrôles lus. Aucun
nouveau changement dans le garde. L'autre clone du portable conserve
672a0d5012f716da2742eeb1bf03e38450c345cd, son arbre et la référence garde-de-match ;
un journal antérieur atteste sa publication. Cela ne reconstitue pas l'ancien
index, ses références locales exhaustives ou ses reflogs. Administration du
parent supprimé absente ; aucun snapshot de ces éléments identifié dans la
lecture ciblée. Preuves et SHA-256 dans
[le plan avant code](../11-11-2026/05-plan-garde-nettoyage.md).

Plan proposé : garde_worktree.py en lecture seule, test_garde_worktree.py
sur fixtures temporaires et appel dans la procédure commune avant retrait.
Contrat : refuser partage/ambiguïté ; ne pas confondre propreté des sources,
propriété des administrations et préservation des références. La méthode de
certification des références utiles reste un point explicite de revue avant
code. Aucun nouveau mécanisme n'est implémenté par la rédaction de ce plan.

## Garde implémentée, en attente de revue

Amendement de l'orchestration appliqué : pas de certification sémantique de
références, préservation fidèle de toute administration détruite par le lanceur,
contrôle en lecture seule via --archive ; partage refusé même avec copie.
Scripts garde_worktree.py et test_garde_worktree.py ajoutés, procédure commune
branchée avant le retrait. Huit témoins locaux passent, sans réseau ni dépôt
réel ; refus partagé prouvé avec données/index/refs/reflogs inchangés dans la
fixture. La garde ne copie, ne pousse, ne répare ni n'efface ; aucun nouvel
accès au garde réel. Voir le plan 05 pour interface et limites. Ticket laissé
ouvert jusqu'à revue ; aucune nouvelle garantie rétrospective sur l'incident.

### Vérification sur les deux postes

Huit tests passent aussi à luciepc sous poste lourd (0,700 s), depuis deux
scripts copiés dans ~/.cache/codeparsers-notes/cx-garde-nettoyage/ ; sans
compiler Rust, ouvrir un dépôt réel ou toucher l'ancien garde. La relecture
a rendu explicite le refus en cas d'erreur de parcours de fichiers : aucune
branche inaccessible n'est silencieusement considérée vide. Journaux et hashes
des versions testées conservés dans ce dossier, à actualiser si le script change.

### Revue : racines protégées

Le défaut relevé dans la revue 6dac58aea est couvert : avant le premier appel
Git ou l'ouverture d'un fichier, tous les arguments dépôt/cible/rapport et
administration/copie sont contrôlés lexicalement, puis canoniquement ; un
composant .env ou .vault refuse, même suivi de .. ou atteint par un alias.
Les chemins Git découverts sont contrôlés aussi. Une zone protégée présente
dans l'arbre refuse avant sa visite et avant status. Les six nouveaux témoins
ont vu cinq échecs sur l'ancienne version (inspection trop précoce), puis
passent après correction ; 14 tests au total. Les cas protégés vérifient par
instrumentation l'absence d'appel Git/lecture d'inventaire, et le cas .env
interdit aussi scandir de cette zone et status. Aucune donnée réelle de ces
zones n'est utilisée ; seules des fixtures temporaires sont construites.

Quiescence toujours à la charge du propriétaire ; archives obligatoirement
hors cible et administration détruite. La garde ne copie ni ne rend atomique
le retrait ultérieur. L'incertitude historique index/refs/reflogs de l'ancien
garde demeure ; ce ticket reste ouvert même après livraison de la garde.

### Garde validée pour livraison, incident historique toujours ouvert

Code final 91c386388bba471fe48a03c99a2a6577009e7c6b, SHA-256 du script
`ef52d5901f3bfc39fa63e4aa72843b7204967b57cc61e713a8cf58b5cbc8e9fb`.
Les 14 témoins passent aux deux postes et ont été rejoués par l'orchestration
(14/14, 0,698 s). Logs luciepc :
`~/.cache/codeparsers-notes/cx-garde-nettoyage/tests-racines-protegees.log`,
SHA-256 `d6aa32b30592102464efecbd5c00e1b5d2a083dbb820efaa8a02af764f546684`.
Revue finale acceptée et tour de push accordé pour scripts/procédure seulement,
sans rejeu Rust requis. La garde est livrée dans ce lot, sans restauration
exhaustive revendiquée : index, références et reflogs historiques du garde
restent non certifiés ; le ticket demeure **ouvert** sur cette partie.

Le retrait des deux worktrees de ce suivi utilisera cette garde, avec copies
fidèles d'administrations privées et rapports exclusifs hors cibles sous
`~/.cache/codeparsers-notes/cx-garde-nettoyage-livraison/` sur le portable.
Commande prévue : `python3 garde_worktree.py --depot <dépôt> --worktree <cible>
--archive <git-dir privé> <copie hors cible> --rapport <rapport neuf hors cible>`.
Les copies, rapports, branches distantes et traces forensiques seront conservés.
Aucun accès supplémentaire à l'ancien codeparsers-garde, aucun GC/prune.

### Livraison effective et arrêt du nettoyage par la garde

Master c04c62021d0d90a29ad0b1d23de22934f49e7526 poussé sous poste push,
verdict VERT explicite, trois contrôles relus : status propre, aucun rebase,
origin/master égal à HEAD. Le bandeau CI pré-push signalait un rouge global
antérieur depuis 21:17 UTC ; ce bandeau n'est pas le résultat des 14 témoins
Python relus pour ce lot et n'en change pas les preuves.

Première utilisation pour retirer le worktree de livraison : refus ROUGE
sur le répertoire lié tools/rust_api/rag3db-src présent dans les sources.
Archive administrative comparée fidèle, mais le contrôle conservateur des
répertoires liés interdit le retrait. Aucun worktree ni branche de ce suivi
n'a donc été supprimé ; aucune tentative en force, modification de lien ou
changement du script pour contourner le refus. L'autre worktree documentaire
est conservé également, sans tentative identique supplémentaire.

Commande exécutée :
`python3 ~/.cache/codeparsers-notes/cx-garde-nettoyage-livraison/garde_worktree.py
--depot /home/lucied/git_workspaces/rag3db
--worktree /home/lucied/git_workspaces/cx-codeparsers-garde-nettoyage
--archive /home/lucied/git_workspaces/rag3db/.git/worktrees/cx-codeparsers-garde-nettoyage
~/.cache/codeparsers-notes/cx-garde-nettoyage-livraison/admin-cx-codeparsers-garde-nettoyage
--rapport ~/.cache/codeparsers-notes/cx-garde-nettoyage-livraison/rapport-cx-codeparsers-garde-nettoyage.json`.
Le shell a développé les chemins ~ avant l'appel ; copie et rapport sont
conservés. Empreinte d'inventaire validée dans ce rapport :
7f7a73fd1688c7cc67e94c120fedd74326de5438b71752a59f8efb5e5c2125ed (11 entrées).
La copie est l'état de l'administration au premier contrôle, pas celui après
la présente mise à jour documentaire ; ne pas la réutiliser comme preuve fraîche.

Reste à cadrer avec l'orchestration : distinction entre un lien source que
le retrait ne ferait que délier et une dépendance administrative qui serait
cassée, puis éventuel témoin et correctif de la garde. Pas de code supplémentaire
avant cette revue. Le volet historique index/refs/reflogs reste non certifié,
indépendamment du refus de nettoyage et de la garde livrée.

### Témoins bornés avant le correctif suivant

Le témoin du lien source suivi vers parent est rouge : la garde conservatrice
le refuse ; le cadrage autorise désormais une feuille 120000 inchangée, sans
visite de sa cible. Le témoin separate-git-dir est également rouge pour la
bonne raison : administration placée sous parent/donnees, suivie dans les
sources du parent (status propre), servant deux worktrees externes ; la version
livrée retourne **VERT à tort**. Aucun retrait réel n'a été appelé. Les 17 tests
(dont ces deux rouges) ont été joués uniquement sur fixtures temporaires.
Il faut reconnaître les candidats administratifs hors des seuls chemins .git,
les attribuer par Git et refuser leur partage même avec archive. Ce constat
reste rattaché à cet incident et ne requalifie pas l'état historique du garde.

### Correctif borné : liens source et administrations sans nom .git

La garde distingue maintenant le lien source du stockage administratif. Un
lien source suivi dans l'index en mode 120000, stade 0, dont readlink égale
le blob d'index, est une feuille : lstat/readlink sont enregistrés, aucune
descente ni ouverture de sa cible. Le status propre reste requis, notamment
pour refuser une modification déjà staged. Un lien modifié ou non suivi refuse.
Racines, administrations, copies et .git liés restent refusés ; composants
protégés et données ignorées/non préservées restent refusés aussi.

Les candidats HEAD+(config/commondir/objects) sont repérés dans toute la zone
source supprimable, pas seulement sous les administrations déjà connues ;
Git doit les attribuer. Le témoin separate-git-dir vu faussement vert avant
correctif est désormais ROUGE pour partage, sans et avec archive fidèle,
fichiers administratifs inchangés. Aucun retrait réel de fixture n'a servi
à découvrir le défaut ; aucun dépôt réel n'a été interrogé par ces tests.

19 témoins passent localement : 14 précédents, lien source vers parent sans
visite ni mutation de cible, lien modifié, lien non suivi, administration
séparée partagée, racines liées refusées. Quiescence à la charge du propriétaire,
archives hors cible ; pas de reprise du nettoyage réel avant nouveau tour.
Le volet historique index/refs/reflogs de l'ancien garde reste non certifié.

### Revue finale du correctif et seconde livraison

Correctif 8e9ceff85db54987027cb3f88979c2ccbaa268e8 relu ; 19 témoins verts sur
le portable, à luciepc sous poste lourd (1,127 s), et rejoués par l'orchestration.
Script SHA-256 dca13abb7a84cf3c1419f728697837a6b1b9b20e34e34c144c9fbf835252f628.
Preuves luciepc : cx-garde-nettoyage/tests-liens-administrations-separees.log
(SHA-256 8f4cfb4fadf601b35960a69a9a66ef82fa8f130df05329256db7bc73589a43ad)
et hashes-liens-administrations-separees.txt. Tour explicite accordé pour ce
correctif ; prévention livrée dans ce lot, volet historique toujours **ouvert**.

Une seconde tentative d'auto-contrôle utilisera exclusivement la version livrée,
des copies administratives neuves et des rapports exclusifs sous
~/.cache/codeparsers-notes/cx-garde-nettoyage-livraison-v2/ sur le portable.
Les premiers rapports/copies/refus restent conservés séparément, sans écrasement.
Aucun retrait si le contrôle est rouge, aucun changement d'état entre archive
et retrait, aucun accès à l'ancien garde ni GC/prune.

### Prévention livrée et nettoyage final vérifié

Master reçoit 65aa01328456439ed0389e9c2a6a60e966c0ae23 sous poste push,
if VERT et trois contrôles lus : propre, aucun rebase, origin/master=HEAD.
La garde livrée (SHA dca13...f628 complet ci-dessus) a contrôlé les deux
worktrees du suivi avant retrait : deux VERT, une archive fidèle privée et
un lien source suivi inspecté comme feuille par worktree. Retraits ensuite
réalisés sans force, aucune modification entre archive et retrait ; branches
locales cx-codeparsers-garde-nettoyage et cx-codeparsers-incident-cleanup supprimées,
chemins absents vérifiés. Branches distantes et preuves conservées.

Artefacts sur le portable, sous
~/.cache/codeparsers-notes/cx-garde-nettoyage-livraison-v2/ :

- pousser.sh : commande exacte `python3 garde_worktree.py --depot R --worktree T
  --archive ADMIN COPIE --rapport NEUF`, puis retrait conditionné à son succès ;
- push-controles.txt et nettoyage.txt : livraison et retraits ;
- admin-cx-codeparsers-garde-nettoyage et admin-cx-codeparsers-incident-cleanup :
  copies fidèles conservées hors cibles, non modifiées après contrôle ;
- rapport-cx-codeparsers-garde-nettoyage.json : VERT, SHA-256
  e85242767f1d9f8e8de0d19f0bc4e2bb591f7bedc94d14c23cfb301089152f54 ;
- rapport-cx-codeparsers-incident-cleanup.json : VERT, SHA-256
  365988edcdd60c5214be48f92558fb8b73771f735a958f5d9638bd0c6c2f9981.

La relecture locale de l'orchestration est 19 tests/0 échec en 0,995 s, preuve
~/.cache/orchestration-notes/guard-review/tests-8e9ceff85.log ; la durée approximative
initialement transmise n'est pas retenue. Premier rapport ROUGE et premières
copies restent conservés séparément. Aucun accès supplémentaire au garde ancien,
aucun GC/prune, aucune restitution exhaustive d'index/refs/reflogs revendiquée.
Ce ticket reste **ouvert** sur l'état historique non certifié ; la prévention
et son nettoyage livrés ne ferment pas ce volet. Après le présent bilan MD,
veille demandée, aucun nouveau lot activé.
