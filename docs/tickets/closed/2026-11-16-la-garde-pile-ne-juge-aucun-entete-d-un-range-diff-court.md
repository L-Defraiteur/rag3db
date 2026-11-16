# La garde pile ne juge aucun en-tête d'un range-diff court

- **État** : ouvert.
- **Famille** : produit
- **Gravité** : filet de push qui peut accepter sans juger les patches.
- **Atteignable en service** : non, garde de push uniquement.
- **Touche rag3weaver** : oui, outillage de qualification et de push.
- **Pour** : G / optimiseur, attribution explicite C7 ; signalement ED.

## Observation et reproduction sans mutation

Au push exactd395cb1f03f84c6abdfa023a0e3c7f87a7c0dab6, garde cache95a1d85779f0105a4c10d977064c548961b0b2661668d8df4b8ea6607428dfb5 égale dépôt. Mode PILE_AVANT=68590270e21d00285b29800bd9cdecf0cb921461 et PILE_TESTE=bd76f557928b5078f98a235f1cb0b5f78e7360d8. Le journal affiche range-diff jugé puis pile tenue, sans aucune ligne =/!/> commentée. Les contrôles indépendants avantpush et ROArchi/Mémoire ont établi le produit qualifié6915entrées exactes,8952masterprotégés et14patches= ; ce signalement ne substitue pas un résultat à ces audits ni ne réattribue la qualification produit.

Relecture APRÈSpush, sans Cargo/service/mutation : git range-diff --no-color --creation-factor=100 68590270e21d00285b29800bd9cdecf0cb921461..bd76f557928b5078f98a235f1cb0b5f78e7360d8 f83c6c4dd6a40200883c6325326f093ecdba3e23..d395cb1f03f84c6abdfa023a0e3c7f87a7c0dab6 produit15en-têtes, TOUS précédés d'un espace de padding. Le case de verifier_pile accepte seulement une ligne commençant directement par chiffre/tiret puis deux-points : zéro en-tête accepté. Première ligne réelle commence par « 1: » précédé d'un espace. La boucle peut donc arriver à pile tenue sans juger de patch ; elle ne vérifie pas non plus qu'elle a vu au moins un en-tête.

## Suite attendue

Témoin rouge CPU du VRAIparseur sur sortie indentée et sur zéro en-tête, puis correction et verts ; conserver les rejets de commits manquants, ajoutés horsdocs et patch changé. Aucune modification de la copie cache sans revue/autorisationC7 ; aucune réécriture des preuves du push. Porteur à attribuer explicitement par C7. Ticket publié avant transmission ; aucune correction ou nouvelle qualification lancée par ED.

## Reprise G

Lot `cx-optimiseur-stack-guard-16`, base d395cb1f03. Témoin CPU du vrai mode
`--pile`, avant correction ; caches et garde de nettoyage6f3 hors périmètre.
Publication de cette reprise avant tests.

## Correctif G publié, attente de revue et intégration C7

Branche `origin/cx-optimiseur-stack-guard-16`, reprise publiée4cc6caeb3a,
témoins publiésce6aa9ab60, correctionb74b85257b0d9363988519c63def98151c08ef8a.
SHA du script corrigé :
`fff9a4d9e20e739e5c8434b0a573ac2cfc00f6f37b45df500993d44d3a4b003d`.

Le vrai mode `--pile` a été vu rouge : trois défauts au premier passage,
puis six échecs sur six avec vérification de tous les en-têtes et placement du
commit code en première position, donc dans la zone indentée. Les fixtures
créent leurs dépôts privés ; Git produit réellement le padding de14/15commits.
Seuls les cas sortie vide/invalide/erreur substituent le producteur range-diff,
le parseur et les vérifications de la garde restent réels.

Le correctif demande `--no-patch` pour séparer les en-têtes des corps de patches,
vérifie le statut de range-diff, accepte l'alignement horizontal Git, exige le
format entier et refuse zéro en-tête. Les règles existantes `=`, `<`, `>` et `!`
restent inchangées : patches manquants, nouveau code et code modifié sont refusés.
Sept témoins CPU passent ; suite existante49verts, zéro rouge, code0 ; syntaxe
Bash et diff-check verts. Journaux conservés hors worktree :
`~/.cache/optimiseur-notes/stack-guard-16/{red.log,red-all-headers.log,green-final.log,green-existing-final.log}`.

Ticket maintenu ouvert pour intégration/revue et décision C7 sur installation.
Aucun cache modifié, aucune pose, aucun service touché, aucun rejeu produit ED,
aucune modification de la garde de nettoyage6f3.

## Candidat neuf demandé par C7

Branche `cx-optimiseur-stack-guard-16-candidate`, base exacte
`d395cb1f03f84c6abdfa023a0e3c7f87a7c0dab6`. Même correctif et témoins,
porteur G ; aucun déplacement ni doublon du ticket ED. Publication avant
qualification CPU et replay RO sur ED exact ; aucune mutation des caches.

### Replay RO ED exact : refus sémantique conservé

Candidat publié `e3360b5587`, source/tests identiquesb74b, 49+7CPU verts.
Audit Git des arbres `src`, `test`, `extension` et `tools/rust_api` : identiques
à masterd395 ; seules garde, nouveau témoin et présent ticket diffèrent.

Le replay du vrai `--pile 68590270e2 bd76f55792 f83c6c4dd6 d395cb1f03`
ne passe PAS : six `=` jugés, arrêt au septième en-tête, ajout non Markdown
`ab0e877375841935a63c3afd8525a84a8b0d72d1`. Il ajoute notamment
`docs/everything-declarative/16-11-2026/ties-qualification/driver.py` et
`launch.py`. La sortie brute comporte15en-têtes : six `=` et neuf `>` ;
ce résultat n'est pas quinze acceptations. La garde corrigée conserve le refus
préexistant des ajouts hors `.md`. Aucun assouplissement, réattribution des audits
produit indépendants ou nouveau rejeu produit. C7 informé du refus avant suite.

Preuves complètes `~/.cache/optimiseur-notes/stack-guard-16/candidate-e3360b5587/` :
`headers.log`, `historical.log`, `ed-exact.log`, `ed-exact-raw.log`, `verdict.json`.
Verdict global ROUGE pour ce replay, indépendamment des56témoins CPU verts.
En attente d'arbitrage C7 ; aucune installation ni cache modifié.

### Arbitrage C7 et candidat conservant les patches testés

C7 confirme que le replay ED négatif est attendu : APPbd76 ne contient pas les
neuf ajouts ultérieurs ; qualification ED indépendante APP+PILOT inchangée.
Le replay positif demandé b74→d736 refuse lui aussi correctement : le squash a
perdu le patch4cc6 du ticket. Rouge conservé dans `self-squashed-replay.log`.
Nouveau candidat `cx-optimiseur-stack-guard-16-r2` depuis b74, donc mêmes commits
testés et seuls compléments `.md` ensuite. Aucun assouplissement de la garde,
ni réécriture des branches publiées ou des anciennes preuves.
