# 06 — Un composant React est un gabarit : les décisions prises

*18 octobre 2026, la nuit. Lucie dormait ; l'orchestration m'a demandé de trancher par la voie
générique et de noter ici chaque décision pour son réveil. Le chemin vient de l'étude de
codeparsers (`../10-octobre-2026-18h20/codeparsers/07-un-composant-react-est-un-gabarit.md`).
Le lot 1, le moteur `tsx` (swc : TSX → JSX automatique → React Refresh), est à codeparsers ; les
lots 2 à 4 sont à moi.*

Chaque décision dit ce qui est choisi, pourquoi, et ce qui reste ouvert. Une décision qui ne
convient pas se reprend : rien n'est publié hors du dépôt.

## Lot 2 — la déclaration, le magasin, la résolution

1. **Un composant se déclare comme un nœud scripté.** `components/<nom>.component.json` à côté
   de son source (`.tsx`, `.jsx`, `.ts`, `.js`), découvert comme `nodes/`, vérifié à chaque
   chargement.
   - Les props sont des **ports d'entrée**, les événements des **ports de sortie**, avec le même
     JSON Schema que les nœuds.
   - **Un port de sortie doit s'appeler `on…`** : un composant ne rend pas de valeur, il émet des
     événements. Refusé sinon, en le nommant.
   - *Pourquoi* : le contrat se vérifie comme entre deux nœuds, et le niveau 2 (le composant
     déclaré par un graphe de clic) n'a rien de nouveau à apprendre.
2. **`libs.json` est indexé par nom de paquet.** Une entrée par paquet, avec sa version exacte,
   ses fichiers et leur blake3, sa table `exports`, son `main` et ses `dependencies`.
   - **Deux versions d'un même paquet ne s'écrivent pas** : une seule instance de React est
     garantie par la forme du fichier, sans règle à part.
   - *Pourquoi* : `react`, `react-dom` et `react/jsx-runtime` doivent partager l'état interne de
     React. Une règle « singleton » pour certains paquets serait une liste à tenir ; la forme
     rend l'erreur inexprimable.
   - *Ouvert* : deux versions d'une même lib qui ne partage pas d'état seraient refusées aussi.
     Aucun besoin connu aujourd'hui.
3. **Les versions sont exactes**, dans `libs.json` comme dans les `libs` d'un composant. Une
   dépendance du `package.json` en `^19.2.0` est rangée en `19.2.0`.
   - *Pourquoi* : un magasin par contenu n'a rien à résoudre ; une plage dirait « n'importe
     laquelle de celles-là », et le hash ne le pourrait pas.
4. **Le magasin est `<backend>/.libs/<blake3>`**, un fichier par contenu.
   - Il est caché, donc hors des déclarations : `list_declarations` saute déjà les dossiers en
     `.`, et un composant ne voit pas mille fichiers de React dans son atelier.
   - Il voyage avec le backend : l'application empaquetée part avec ses libs, hors réseau.
5. **Le magasin se remplit par une commande seulement** : `rag3weaver-backend libs add
   <backend.json> <paquet déballé>`.
   - La commande lit le `package.json`, hache et copie chaque fichier, et écrit l'entrée de
     `libs.json`.
   - Rien ne télécharge : un paquet se déballe d'abord (`npm pack react@19.2.0`, puis `tar`).
     C'est la règle de la vision : « téléchargé sur commande explicite seulement ».
   - *Ouvert* : une option qui va chercher le paquet au registre npm, sur la même commande, si
     Lucie la veut. Je ne l'ai pas faite pour ne pas mettre le réseau dans ce lot.
6. **La résolution est une table, jamais le disque** (`Components::resolve`) :
   - un spécifier relatif d'un composant mène à un autre composant ;
   - un relatif d'un fichier de lib mène à un fichier de la même lib (`.js` et `/index.js`
     essayés) ;
   - un spécifier nu mène à la lib que l'importeur **déclare** (les `libs` du composant, les
     `dependencies` d'une lib), puis par ses `exports`, avec les conditions `browser`,
     `development` ou `production`, `import` et `default`.

   **Les conditions d'`exports` se prennent dans notre ordre** (`browser`, le mode, `import`,
   puis `default` en dernier), et non dans l'ordre de l'objet comme le fait Node. Notre lecture
   du JSON range les clés : « default » passait avant « development », et le mode
   développement prenait le fichier de production. Le témoin l'a vu rouge d'abord. L'écart avec
   Node ne compte que pour un paquet qui mettrait, par exemple, `import` avant `browser`.

   Ce qui n'est pas déclaré est refusé en nommant l'importeur et le paquet. Un composant
   n'importe que ce qu'il déclare, même si une autre partie du backend l'a.
7. **Les vérifications au chargement** : la version des libs d'un composant contre `libs.json`,
   chaque dépendance déclarée à sa version, chaque fichier présent au magasin, chaque schéma de
   port valide. Le backend refuse de se charger en les nommant toutes, comme pour `nodes/`. La
   vérification des octets eux-mêmes (le hash recalculé) se fera au service (lot 3), là où on
   les lit.

Les lots 3 (servir les modules, la page d'aperçu) et 4 (le sujet `components` du flux, le
registre de modules, React Refresh) suivront ici, avec leurs décisions.
