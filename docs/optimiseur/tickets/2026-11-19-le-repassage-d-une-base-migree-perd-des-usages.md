# Le premier passage de l'écrivain sur une base migrée perd des usages

- **État** : ouvert (19 novembre 2026), relevé à la répétition complète de la bascule vers les versions
- **Famille** : produit (l'écrivain de rag3weaver)
- **Gravité** : bloque la bascule des trois bases de code ; `usages`, l'outil de mesure d'impact, rendrait moins de sites
- **Atteignable en service** : oui, au premier index de toute base passée par 1b puis les générations
- **Touche rag3weaver** : oui

## Ce qui s'est passé

Répétition complète au portable, le 19 novembre au soir, sur une copie reflink de l'instantané
`avant-versions-20261119/code-rag3db-depot` (la vraie base, avant 1b). Hôte isolé (port 7992) bâti sur a83e234fc4,
qui porte le correctif du tampon (ed-rewrite-bounded-memory), et lib 6e2476d41.

- 1b en 845 s, générations, réouverture sans avertissement.
- Repassage jusqu'à « indexation terminée » en 20 min, RSS maximal de l'hôte 9,1 Gio : le tampon tient.
- Comparaison à la base de production, relevée au même moment, sur le même arbre (b79878da9b) :
  - search_code (10 requêtes, top 10) : recouvrement 96,8 %, même premier résultat 10/10 ;
  - `usages` identiques pour acquireLock, FileSource et lockRowForWrite ;
  - **`usages Catalog` : 774 sites en production, 733 sur la copie repassée, aucun site en plus.** Les 41 manquants
    sont tous sous `extension/rag3weaver/tests/` (e2e_generic_search ×7, e2e_copy_liens ×5, e2e_entites_derivees ×3,
    e2e_mesure_impact ×2, e2e_cloud_code_agent ×2, structured_payloads ×2, e2e_postgres ×2, …). Ce sont de vrais
    usages de l'arbre, par exemple `e2e_arret_brutal.rs:476` « let mut catalog = Catalog::new(boxed, … ».
    `invisible_usages` passe de 1 à 0.

Le critère de c7 (usages identiques) est manqué : pas de bascule.

## Les pièces

Au portable, `~/.cache/rag3weaver-build/mesures/repetition-versions-code-rag3db-depot-20261119-194739/` :
- la copie repassée, fermée et intacte (`copie/`), et son manifeste aux chemins absolus (`backend.json`) ;
- les journaux de 1b, des générations et du repassage ;
- `memoire.tsv` ;
- `usages-prod-maintenant.json` et `usages-apres.json` (format json, limit 200).

## Pour le correctif

Un témoin rouge d'abord : une base migrée, le premier index de l'écrivain, puis `usages` d'un type construit par
`Type::new(` dans un fichier de tests, qui doit garder ses sites. Le défaut est à chercher dans ce que le premier
passage réécrit des arêtes d'usage (MENTIONS, CONSUMES) pour les fichiers de tests.
