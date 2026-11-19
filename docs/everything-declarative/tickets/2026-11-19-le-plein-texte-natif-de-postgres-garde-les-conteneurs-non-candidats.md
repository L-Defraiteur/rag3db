# Le plein texte natif de postgres garde les conteneurs non candidats

- **État** : ouvert (19 novembre 2026)
- **Famille** : produit (recherche)
- **Gravité** : sur postgres, un namespace ou un module replié reste candidat au plein texte, et peut repasser devant une
  classe du même nom, le défaut de 2.12.
- **Atteignable en service** : sur toute base de code postgres ; aucune base de Lucie n'y est aujourd'hui.
- **Touche rag3weaver** : oui (le plein texte natif de `PostgresDialect`, la recherche `PostgresSearchBackend`)
- **Pour** : everything declarative
- **Signalé par** : ED, au lot de 2.12 (branche `container-not-candidate`), avec l'accord de c7.

## Ce que c'est

La règle de 2.12 (`ChunkingConfig::not_candidate`) retire une ligne non candidate de deux endroits : ses morceaux,
donc ses vecteurs, et son document du plein texte lucivy. Le plein texte de postgres est natif (`tsvector`), tenu
par la base sur la ligne elle-même : rien ne l'écrit à part. Le saut d'`InsertRecordNode` et le geste des bases
existantes n'y ont donc pas d'effet. Sur postgres, la règle ne retire que les morceaux.

## Ce qu'il faudrait

Que la recherche plein texte native ne voie pas une ligne non candidate. Par exemple :
- une colonne générée qui vaut le texte ou NULL selon la règle ;
- ou un index partiel `WHERE` la règle tient.

La règle devrait alors se traduire en SQL depuis `NotCandidate`. Le témoin, rouge d'abord, est celui de 2.12 joué sur
`e2e_postgres`.
