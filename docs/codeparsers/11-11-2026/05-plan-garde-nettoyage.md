# Plan avant code : garde commune de retrait de worktree

Ticket porteur :
[incident de nettoyage](../tickets/2026-11-11-le-nettoyage-du-parent-invalide-un-worktree-de-sous-module.md).
Session : codeparsers ; revue orchestration requise avant implémentation.
Aucun changement de Rust, de l'ancien garde ou de ses métadonnées.

## Résultat de la lecture ciblée

Le dépôt administratif de rag3db-brancher n'existe plus sur luciepc. Le clone
principal du sous-module distant subsiste, avec ses propres références et son
seul worktree ; ce n'est pas une sauvegarde de l'ancien index/reflog du garde.
La lecture est enregistrée hors garde dans
`~/.cache/codeparsers-notes/cx-passe-require-3/lecture-restes-git-incident.txt`,
SHA-256 `ad641c5da8683a39dc4e78191771a6da803517fcf6f309772b4fb1bf75405417`.

Sur le portable, un autre dépôt du sous-module conserve
`refs/heads/garde-de-match` vers
`672a0d5012f716da2742eeb1bf03e38450c345cd`, arbre
`a5521a9c8e9177f2e2a8bd1d4f2938a5c7cf7c86`. Ses reflogs contiennent commit,
publication et checkouts de cette version. Artefact hors garde :
`~/.cache/codeparsers-notes/cx-incident-cleanup/references-autre-clone.txt`,
SHA-256 `57da02ec808c3334f820d90475057af0624cd150c821f0a4c574fdce864d7d11`.

Un journal antérieur à luciepc,
`~/.cache/codeparsers-notes/push-sm-codeparsers-garde-pointeur-3.txt`, indique
« codeparsers master → 672a0d5 », avec contrôles du lot f3a115887.
SHA-256 `8a911e979508274db87440b74eb414dc96061c8c106f9f931c116335c3993dff`.
Il atteste une publication historique de cette version. Il ne certifie pas
l'état du garde juste avant le nettoyage. Aucune copie antérieure de son index,
de toutes ses références locales ou de ses reflogs n'a été identifiée dans
ces lectures ciblées. Ne pas reconstruire ces éléments à partir d'une hypothèse.

## Frontière et garanties proposées

L'unité à protéger est l'administration Git effectivement supprimée, distincte
des fichiers du worktree. Un parent propre peut détenir l'administration d'un
sous-module dont un autre worktree dépend. Le contrôle doit donc examiner les
dépendances de cette administration, pas seulement parcourir les fichiers du
parent ou tester son status.

La garde commune est une commande de **lecture**, immédiatement avant le
retrait explicite. Elle ne supprime, ne répare, ne prune ni ne réinitialise rien.
Elle refuse le partage et l'ambiguïté. Une réussite ne certifie pas l'absence
de writer non coopératif : le retrait exige encore la maîtrise de ses propres
processus et la coordination existante. Pas de kill, verrou nouveau ou prétendue
transaction entre Git et tous les writers du poste dans ce lot.

## Fichiers et interface proposés

| Fichier | Rôle |
|---|---|
| `extension/rag3weaver/scripts/garde_worktree.py` | Commande commune, bibliothèque standard Python, inspection seulement. |
| `extension/rag3weaver/scripts/test_garde_worktree.py` | Fixtures Git locales temporaires, aucun accès aux vrais dépôts. |
| `docs/procedures/push-d-un-lot.md` | Appeler la garde avant git worktree remove ; retirer le retrait inconditionnel de l'exemple. |
| Ticket de l'incident et présent plan | Preuves, périmètre accepté, résultats et limites. |

Interface proposée :

```text
python3 scripts/garde_worktree.py --depot <dépôt> --worktree <chemin> --rapport <json hors cible>
```

Aucun paramètre force ou réparation. Sorties : 0=contrôles structurels acceptés,
2=retrait refusé (dépendance, état incertain, données locales non préservées),
1=erreur d'utilisation ou d'inspection. Le JSON nomme les administrations,
leurs chemins canoniques, propriétaires, worktrees dépendants et motifs du
verdict. Il ne contient ni contenus source ni secrets. Un rapport demandé
ne doit pas se trouver dans la cible ou son administration supprimable.

Inspection via Git avec `GIT_OPTIONAL_LOCKS=0` : identification canonique
worktree/git-dir/git-common-dir, vérification que la cible est un worktree
secondaire enregistré (refuser racine principale, chemins étrangers et liens
ambigus), inventaire des sous-modules initialisés et de leurs administrations
imbriquées, et lecture `worktree list --porcelain` de chaque dépôt découvert.
Les entrées Git ne sont pas déduites d'un nom de dossier usuel seulement.
Les métadonnées .git/modules doivent être considérées même pour un sous-module
désinitialisé ou déplacé ; une administration résiduelle non attribuable refuse.
Aucune traversée de .vault ou lecture de .env.

Refuser dès qu'une administration contenue dans celle que le retrait détruit
est commune à un autre worktree, ou qu'une entrée ne peut pas être résolue de
façon certaine (gitfile cassé, worktree manquant/enregistré, symlink ou sortie
Git inexploitable). Refuser les états sales/non sauvegardés : status du parent
ne suffit pas, ceux des sous-modules concernés comptent aussi. L'inventaire
n'autorise pas la destruction des références uniques : les références locales
utiles doivent être accessibles hors des administrations supprimables ; sinon
refus et préservation explicite à cadrer, jamais push/bundle automatique.
La méthode précise de certification des références est à revoir avant code,
notamment HEAD détachée, reflogs non publiés et sous-modules privés. Un résultat
non certifiable doit refuser plutôt que considérer les seules branches origin
comme preuve de conservation exhaustive.

## Témoin local proposé

Créer dans TemporaryDirectory des dépôts locaux et un parent lié avec son
sous-module initialisé. Depuis ce dépôt de sous-module, ajouter un autre
worktree hors parent. Y poser une référence locale et une modification de
fichier, puis capturer avant le contrôle : HEAD, refs, hash d'index, reflog,
hash des fichiers suivis et non suivis de la fixture. La garde doit refuser
le retrait du parent à cause de la dépendance administrative. Relire ensuite
le même inventaire et comparer ; toutes les données et références doivent
être intactes et Git encore opérationnel. Ne pas appeler un retrait réel
si la garde refuse, et ne jamais utiliser codeparsers-garde comme fixture.

Ajouter un cas privé sans dépendant, un gitfile cassé et une administration
résiduelle ambiguë pour vérifier les décisions, sans multiplier les cas qui
recopient simplement les branches du code. Le cas nominal doit avoir ses
références préservées hors administration détruite. Les essais restent sans
réseau, sous TemporaryDirectory, avec autorisation du protocole file limitée
aux commandes de construction de la fixture. Nettoyer uniquement ces fixtures.

## Point de revue

Valider d'abord cette interface et la règle de préservation des références
avant implémentation. Puis jouer les témoins locaux et revoir le branchement
dans la procédure commune. Le ticket reste ouvert ; la publication des seules
rectifications 8df1266cd n'a ni livré cette garde ni restauré les preuves perdues.
