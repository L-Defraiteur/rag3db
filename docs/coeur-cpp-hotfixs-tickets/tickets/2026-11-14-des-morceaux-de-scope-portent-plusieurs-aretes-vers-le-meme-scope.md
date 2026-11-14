# Des morceaux de Scope portent plusieurs arêtes vers le même Scope

- **État** : ouvert (14 novembre 2026, hotfixs, vu par la sonde des orphelines sur la base migrée par 1b ;
  arbre principal avait le même compte)
- **Famille** : produit
- **Gravité** : à départager. Ce ne sont pas des orphelines : chaque arête relie un morceau vivant à son
  Scope vivant. Mais un même morceau est rattaché jusqu'à sept fois à son Scope, ce qui fausse tout
  compte ou tout parcours qui ne dédoublonne pas (une remontée du morceau vers le Scope rend le Scope
  autant de fois).
- **Atteignable en service** : oui, dans la base de dépôt de Lucie (copie du 13 novembre)
- **Touche rag3weaver** : oui (l'écriture des morceaux de Scope et de leur arête `Scope_CHUNKED_FROM`)
- **Pour** : à attribuer par c7 (le chemin des morceaux de l'ancienne forme, celle d'avant les versions)

## Ce qu'on voit

Dans `code-rag3db-depot`, sur la copie de l'instantané `avant-835d5298f` (13 nov., 20 h 07, hôte
arrêté), puis sur sa migration par 1b (`~/.cache/rag3weaver-build/1b-copie/migree-20261114-1641/`, lib
0c41f9fe6), avec les mêmes chiffres dans les deux :

| mesure | valeur |
|---|---:|
| morceaux Scope_Chunk | 127 195 |
| arêtes `Scope_CHUNKED_FROM` | 129 400 |
| morceaux à plus d'une arête | **1 291** (2 à 7 arêtes chacun ; 3 496 arêtes à eux seuls, soit 2 205 de trop) |
| morceaux dont les arêtes visent des Scope différents | **0** : toujours le même Scope |
| arêtes dont le Scope n'est pas le `_parent_uuid` du morceau | 0 |

Exemples (morceau, Scope, nombre d'arêtes) :
- `80a0b597-e4a5-c15f-5096-b64cdae25f01` → `461389fc-04f9-940b-d73c-655233bef0cd`, 3 ;
- `b5eceae9-fa98-5e1d-f36d-b7d20ad588bc` → `ead79a64-f2bd-b60e-4d40-1068fce05d66`, 7 ;
- `a78419ac-0421-5257-2c96-bbea415d7ab7` → `2067ebf2-6642-001a-f519-0d44f1e52cda`, 5.

**Les autres bases et tables sont propres.** code-lourd et code-principal (copies de
`avant-835d5298f`) n'ont aucun doublon, ni dans `Scope_CHUNKED_FROM` ni dans `File_CHUNKED_FROM`. Dans
le dépôt migré, `File_CHUNKED_FROM`, `Library_CHUNKED_FROM`, `ScopeContent_CHUNKED_FROM`,
`ScopeContent_DERIVED_FROM` et `MENTIONS` (comptées par paire de bouts) n'en ont pas non plus.

## Pistes à lire

- La base du dépôt est celle qu'un hôte a réindexée au fil de nombreux redémarrages ; c'est aussi elle
  qui accumule 8 809 documents plein texte orphelins (ticket
  `docs/recherche/tickets/2026-11-14-un-scope-efface-du-graphe-garde-son-document-plein-texte.md`). Une
  redécoupe qui recrée l'arête par un CREATE au lieu d'un MERGE, quand le morceau existe déjà, donnerait
  cette forme : même morceau, même Scope, une arête de plus à chaque passage.
- Scope_Chunk a aussi 21 287 places vides sous son offset maximal (des morceaux effacés).

## Ce qu'il faut pour le fermer

1. Trouver le chemin qui ajoute l'arête sans voir celle qui existe.
2. Un témoin rouge : redécouper un Scope déjà découpé, et compter une arête par morceau.
3. Le correctif, puis la décision de Lucie sur les bases existantes : dédoublonner, ou laisser la
   migration vers la forme à versions remplacer cet ancien index.
