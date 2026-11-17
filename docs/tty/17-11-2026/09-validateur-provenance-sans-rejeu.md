# Correctif groupé du validateur R2, sans essai vert

Préparation C7 cbffc3122e après baseline14 sourceeeb, vrais10PASS4FAIL/0ERROR,
preuves224582 et ROindépendante. Même ticket anglais OPEN, réserves ajoutées
avant diff e30c138. Les huit assertionsd998 et les six nouvelles564393 sont
BYTEinchangées ; aucun CPUtest exécuté après ce rouge.

Le vrai results exige, quand une identité d'ELF est attendue, une provenance
pour ce genre de bloc. Chemin de catalogue et chemin source doivent désigner
l'ELF attendu ; la copie préservée exécutée a un chemin distinct. Empreinte du
catalogue, avant, après et copie doivent être identiques, SHA256 strict. Les
bytes réels du chemin source et de la copie sont relus par digest, primitive
existante, et doivent encore porter cette empreinte. La recette déclarative
seule ne suffit donc pas. Dans les témoins, digest lit un ensemble de bytes
virtuels, pas des empreintes promues silencieusement en observations réelles.

Le vrai feature_results compare TOUS les jeuxVERT à la liste ordonnée exacte :
plus de filtre qui abandonne une ligne étrangère. Les deux précontrôles connus
du script (tickets avec compte exact et « le lot ne touche pas l'analyse »)
sont reconnus par leur grammaire exclusivement AVANT les jeux ; ils ne sont
pas des jeux supplémentaires ni une permission d'ignorer d'autres lignesVERT.
Les anciens contrôles blocs/noms/filets/ignored, premierFAILED et should_panic
restent présents. Une fin8/8 ne masque plus un neuvième jeu.

## Limite physique explicite

Le producteur de capture d'exécution du canon n'est PAS livré ici. Le driver
ne fabrique pas une preuve d'exécution depuis le catalogue : ELF_PROVENANCE
reste vide en l'absence de ce raccord, et results refuse. Il faudra composer
la capture réelle au moment de l'exécution (copie effectivement jouée, bornes
avant/après, identité du contenu), notamment pour la lib reconstruite après
F8/touch. Un futur reçu devra aussi être lié à l'allocation/source/phase/END ;
le présent validateur de contenu ne prouve pas ces autres dimensions.
Aucun champ virtuel de test ne devient un grant ou une preuve physique.

Ce manque reste une condition avant activation dans le contrat INACTIF,
avec acteurPCLOCAL/politiqueTTY/identités/payload/PGSQL/release. Qv4 nouveau
est préparatoire, execution_ready=false conservé avant allocation/slot.
Qv2/Qv3 et leurs sources ne sont ni réécrits ni activés. Target et archives
rouges inchangés ; pas de Cargo/no-run/list/native/service/acteur/SQL/contact.

Vérification ici : AST et SHA des témoins inchangés, whitespace. Suite :
source et pins publiés, Qdésarmé, revue totale puis GOCPU14vert DISTINCT.
Une éventuelle réussite hermétique14 ne qualifiera ni capture physique ni
pipeline canonique complet, et n'accordera aucun GO3600.
