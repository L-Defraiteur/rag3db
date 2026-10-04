# Une propriété de chaîne d'une relation se lit fausse dans le sens direct après un point de reprise

- **État** : ouvert — trouvé par la session du cœur C++ (rag3db-e3), correctif annoncé ; ce ticket est écrit du côté rag3weaver, à compléter par le cœur C++ (cause, recette, commit)
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (la marque `resolution` des arêtes d'usage, que lisent les filtres de Liens)

## Ce que c'est

Un point de reprise qui suit la mise à jour d'une propriété de chaîne d'une
relation peut fausser cette propriété, **dans le sens direct**, sur d'autres
relations de la même table ; le sens inverse reste juste. Vu une fois sur
sept reprises par l'arbre principal (4 octobre, soir) : 140 arêtes `CONSUMES`
internes à des fichiers neufs lues `resolution = nom` dans le sens direct au
lieu de `fichier`.

## Ce que ça fait à rag3weaver

Le filtre des arêtes devinées (`edge_field=resolution, edge_guessed=nom`) lit
la propriété sur l'arête du saut :
- **Liens** suit les deux sens ; le saut sortant lit le sens direct — une
  arête sûre lue « nom » y est tue à tort ;
- **« dépend de »** (voisinage sortant) : même exposition ;
- **impact** et **impact d'un fichier** marchent en entrant (`direction=incoming`) :
  ils lisent depuis la cible, sûrs ;
- **usages** lit les arêtes directes depuis la définition (entrant) : sûr.

## Témoin

Pas de recette minimale ici (à compléter par le cœur C++). Côté rag3weaver :
compter `MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) RETURN r.resolution, count(*)`
contre la même requête écrite depuis la cible, avant et après un point de
reprise qui suit une mise à jour de `resolution`.

## Pour le fermer

Le correctif du moteur ; puis la comparaison des deux sens ci-dessus, à zéro
écart.
