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
