# L'oracle cherche la cible au lieu du nom réellement mentionné

- **État** : ouvert — lecture du code et vérification des 415 lignes historiques le 11 novembre 2026 ; aucun changement d'oracle dans le lot des exports
- **Gravité** : preuve incomplète (un faux peut devenir muet sans interrogation du LSP)
- **Atteignable en service** : non (oracle de mesure seulement)
- **Touche rag3weaver** : oui (outillage de validation des arêtes)
- **Famille** : produit

- **Pour** : session codeparsers
- **Signalé par** : session codeparsers ; reprise et preuves dans [le bilan du 11 novembre](../11-11-2026/04-bilan-navigation-et-provenance.md)

## Preuve et cause

La sonde A/B fournit `b.name` comme champ nom de l'arête. Dans
`scripts/oracle_lsp.py`, la boucle cherche `a['nom']` sur la ligne source,
puis interroge le LSP seulement si ce texte y apparaît. Le verdict commence
à muet. Une cible zone de fichier, ou une classe exportée dont le nom diffère
de l'alias importé, peut donc être rendue muette sans question LSP.

Vérification à luciepc : les 415 lignes marquées liaison de
`~/.cache/codeparsers-notes/require-mesure-2/verdicts-ajoutees.tsv`
ont été comparées à leurs lignes sources dans `oracle/arbre`, avec la même
regex que l'oracle : 415 tokens absents, zéro source absente, zéro token présent.
Le rapport ancien qui les décrit comme un silence du serveur ne démontre donc
pas ce silence. Il démontre une absence de jugement.

L'instance FS corrigée vers sa zone et un alias Alias vers une classe Outil
rencontrent la même limite. Une baisse des faux accompagnée de nouveaux muets
ne prouve pas un gain de précision.

## Invariant requis pour le fermer

Conserver séparément l'identité de la mention (texte et position source) et la
cible proposée (fichier et plage). Interroger la mention réelle et juger la cible
par les positions rendues, sans substituer son nom à celui de la mention.
Une question non envoyée doit se distinguer d'un LSP interrogé sans réponse.
Poser des témoins alias et zone, puis rejouer les mesures concernées avec les
entrées corrigées, en conservant les anciennes preuves et les seuils.
Cette correction d'outillage est distincte du lot de navigation ; aucune
modification de l'oracle n'est autorisée implicitement par ce ticket.

## Complément : la ligne ne désigne pas chaque occurrence

L'examen du producteur d96ed3264/a730a9b montre une seconde perte :
IdentifierReference porte le nom source et une colonne optionnelle ; UsageSite
n'en garde que la ligne et la nature. usage_properties conserve ensuite le
minimum des lignes du groupe. Lire Scope.key et ses bornes améliore
l'attribution au propriétaire sans restituer les occurrences supprimées.
Sur huit usages des index nodejs/sync, les dépendances existent toujours avec
une autre ligne représentative : les retraits de tuples ne prouvent pas une
perte de rappel. Le complément de 23 questions a été joué, avec ses réponses
brutes conservées ; il ne ferme pas ce ticket général.
Voir [le bilan et la chaîne de provenance](../11-11-2026/04-bilan-navigation-et-provenance.md).

## Action bornée proposée et portage

Porteur : codeparsers. Diagnostic terminé dans le bilan db0dbfcbb ; le défaut
et la suite de validation restent ouverts. Prochaine action proposée à
l'orchestration : capture de provenance avant perte sur les mêmes 23 questions,
avec versions/options figées, clés de scopes, tokens et positions vérifiés,
puis réutilisation des réponses LSP existantes. Ne pas attribuer les arêtes
à partir du résultat LSP lui-même. Cette proposition ne déclenche aucune
nouvelle campagne, schéma ou refonte générale sans cadrage supplémentaire.
Reste : valider le périmètre, réaliser la capture, rendre explicites les cas
non attribuables, et satisfaire les critères généraux de fermeture ci-dessus.
