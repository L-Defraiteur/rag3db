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
