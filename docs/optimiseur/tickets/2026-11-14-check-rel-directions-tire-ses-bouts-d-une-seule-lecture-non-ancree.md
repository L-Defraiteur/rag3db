# check_rel_directions tire ses bouts d'une seule lecture non ancrée

- **État** : ouvert (14 novembre 2026, 22 h 45), relevé d'hotfixs.
- **Famille** : produit
- **Gravité** : faux vert ; un « égales » de l'outil ne prouve pas qu'il ne manque rien.
- **Atteignable en service** : non, c'est l'outil du contrôle des deux sens de la bascule (procédure, § 8).
- **Touche rag3weaver** : non ; `tools/check_rel_directions/check_rel_directions.cpp`.
- **Pour** : optimiseur (G), avec coeur c++/hotfixs, qui a le relevé ancré prouvé.
- **Ouvert le** : 14 novembre 2026

## Ce que c'est

L'outil relève d'abord les sources et les destinations d'une table de relations par une seule requête non ancrée :
`MATCH (a)-[r:T]->(b) RETURN label(a), offset(id(a)), label(b), offset(id(b))` (l. 73). Le planificateur la joue dans un
seul sens. Si une source est perdue par ce sens-là, elle n'est jamais ancrée : sa relation n'est relue par aucun des
deux sens, même par celui qui l'a encore. Les deux relevés ancrés qui suivent ne voient alors que les bouts de cette
première lecture.

**Le cas prouvé par hotfixs**, sur code-principal, table LOCKS : `lockIndexForWrite` (Scope, lignes 1169–1177) →
`acquireLock`, à la ligne 1173. La relation est vraie dans le code (`node_table.cpp:1173` à 284cc00c0), et une requête
ancrée sur ce Scope la rend. Pourtant elle manquait aux deux relevés, celui de l'outil et le premier d'hotfixs, qui
reprenait la même lecture. Le sens arrière attribue cette ligne 1173 à `lockRowForWrite`.

**Conséquence** : les « aucun écart » de la bascule du 14 novembre au soir (memoire-longue et code-rag3db-depot au
portable) sont plus faibles que la page 04 ne le laissait croire. Ils disent qu'aucune relation lue d'un côté ne
manque de l'autre. Ils ne disent rien de ce qui manque à la première lecture.

## Le correctif

C'est le relevé d'hotfixs (`releve_sens2`, à luciepc dans `~/.cache/rag3db-locks/reconstruction-code-principal/`) :
les bouts se parcourent par **tranches de 500 offsets de chaque table de nœuds**, chacune ancrée sur son étiquette, et
non plus depuis une lecture de la relation. Le plan prouve le sens :
- `MATCH (a:Scope) WHERE offset(id(a)) in [o, o+500) MATCH (a)-[r]->(b)` donne `SCAN_NODE_TABLE Scope`, puis
  `SCAN_REL_TABLE` avec `Direction (a)-[r]->(b)` ;
- ancrée sur Symbol, la même tranche donne `Direction (b)<-[r]-(a)`.

Le rouge d'abord : une base jetable où une relation n'est lisible que par l'ancrage d'un sens, invisible à la lecture
non ancrée. Avec l'outil d'aujourd'hui, elle doit rendre « égales » ; avec le correctif, l'écart.
