# Les listes scalaires de filtre produisent du Cypher sur PostgreSQL

- **État** : ouvert, erreur réelle atteinte après déblocage de l'ingestion 1a.
- **Famille** : produit
- **Gravité** : recherche dense refusée par erreur de syntaxe dans le masque d'admission.
- **Atteignable en service** : oui, Catalog::rechercher avec liste d'UUID sur PostgreSQL.
- **Touche rag3weaver** : oui, FilterParser et représentation dialectale de l'appartenance scalaire.
- **Pour** : arbre-principal, coordination recherche/orchestration pour le contrat commun.
- **Signalé par** : témoin privé SQL des versions et occurrences du lot 1a.

## Preuve

Source **60961bb24**, `cx-pg-graphe-gdb-vert.log` (nom de relance, pas verdict global) :
le blocage disparaît, les deux ingestions passent, les assertions deux versions/deux chunks
sont franchies. La première lecture publique dense échoue en 0,37 s : SQL42601 sur
`p."_uuid" IN $1` dans la recherche ContenuPg_Chunk. Test final **0/1**, aucun vert SQL complet.
La connexion est intacte ; l'erreur vient de la compilation du filtre, pas d'un format
modifié de fixture. Le masque canonique emploie FilterValue::List comme les filtres publics.

FilterParser, inchangé par 1a, construit IN directement pour List/In/NotIn. Les primitives
IR du dialecte SQL savent déjà émettre = ANY(param), mais cette règle n'est pas partagée
avec FilterParser. La syntaxe est distincte de la sérialisation UUID[] corrigée plus tôt.
Aucune garantie supplémentaire de typage des tableaux n'est déduite de cette preuve UUID.

## Suite bornée

Poser un rouge de compilation de filtre scalaire texte sur PostgresDialect ; factoriser
la représentation d'appartenance dans le dialecte, puis la consommer dans FilterParser
et les prédicats IR existants. Les trois chemins List/In/NotIn doivent rester équivalents,
le natif conserver IN, les paramètres et le masque avant top-k rester présents.
Rejouer le vrai parcours public SQL et la suite PG entière (transactions comprises).
Lien : [versions et occurrences](2026-11-11-deux-occurrences-identiques-reembarquent-le-meme-contenu.md).
