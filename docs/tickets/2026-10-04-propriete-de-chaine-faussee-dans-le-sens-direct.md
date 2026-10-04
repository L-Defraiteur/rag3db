# Une propriété de chaîne d'une relation se lit fausse dans le sens direct après un point de reprise

- **État** : ouvert — cause trouvée par le cœur C++ (rag3db-e3), correctif en passe ; écrit du côté rag3weaver, à compléter par le cœur C++ (recette, commit)
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (la marque `resolution` des arêtes d'usage, que lisent les filtres de Liens)

## Ce que c'est

Un point de reprise qui suit l'ajout ou la mise à jour de relations peut
fausser une propriété de chaîne **très dupliquée** (`resolution`, `usage`,
`kind`) d'autres relations de la même table. Cause trouvée par le cœur C++ :
la relecture des chaînes au point de reprise, dans le code d'origine. **Selon
la région réécrite, c'est l'un ou l'autre rangement (sens direct ou inverse)
qui est faux**, et le planificateur choisit lequel une requête lit : aucun
sens n'est sûr par principe. Vu par l'arbre principal (4 octobre, soir) :
140 arêtes `CONSUMES` internes à des fichiers neufs lues `resolution = nom`
au lieu de `fichier`.

Le dégât reste sur disque : une base qui a reçu des ajouts ou des mises à
jour de relations après un premier point de reprise est suspecte, et se
réindexe une fois le correctif passé.

## Ce que ça fait à rag3weaver

Tout ce qui lit `resolution`, `usage` ou `kind` sur une arête : les filtres
d'arête (Liens, impact, impact d'un fichier), la mention « (par le nom) »
d'`usages`, le genre d'usage, et le genre des rendez-vous. Aucune de ces
lectures n'est sûre en attendant le correctif.

Les mesures de la session codeparsers du 4 octobre (banc des relations,
sondes des marques) viennent d'index neufs, construits en mémoire à chaque
passe ; qu'un point de reprise y soit survenu en cours d'ingestion n'est pas
établi. À rejouer après le correctif, sur une indexation de zéro, avant d'y
fonder une décision.

## Témoin

Le seul contrôle valable : comparer, relation par relation, la même
propriété lue par les deux formes de requête (depuis la source et depuis la
cible), après un point de reprise qui suit des mises à jour de relations.
Recette et commit : cœur C++.

## Pour le fermer

Le correctif du moteur ; une réindexation de zéro ; la comparaison des deux
formes ci-dessus, à zéro écart ; puis le banc des relations rejoué.
