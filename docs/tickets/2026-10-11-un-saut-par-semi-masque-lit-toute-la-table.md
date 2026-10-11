# Un saut par semi-masque lit toute la table quand le masque est petit et éparpillé

- **État** : ouvert (11 octobre 2026) — au cœur C++, avec le ticket des fils (même famille : le planificateur ne regarde pas la cardinalité) ; ouvert par la session embarquements à la demande de l'orchestration
- **Gravité** : coût (l'outil `usages` prend 0,4 à 1,5 s sur un index entier au lieu de quelques dizaines de millisecondes)
- **Atteignable en service** : oui, à chaque saut vers une grande table de nœuds (usages, impact, liens, sur un dépôt réel)
- **Touche rag3weaver** : oui (tous les sauts des outils ; contournement à mesurer côté dialecte)

## Ce que c'est

Sur l'index entier de rag3db (copie de `code-rag3db-depot`, 7 310 fichiers), un appel de
`usages` ne fait que six instructions, mais chacune prend 24 à 470 ms, même celle qui ne
rend aucune ligne ; le coût suit la taille de la table, pas le travail. Le saut le plus
coûteux, le rendez-vous de `update` :

```
UNWIND $uuids AS u MATCH (d:Symbol {_uuid: u})<-[r:MENTIONS]-(m:Scope) RETURN u, m._uuid, m.name, …
```

Son plan (`PROFILE`, réduit à `m._uuid, m.name`) :

| opérateur | lignes sorties | temps |
|---|---|---|
| `PRIMARY_KEY_SCAN_NODE_TABLE` (d) | 1 | 0,08 ms |
| extension `MENTIONS` (r) | 117 | 0,21 ms |
| `SEMI_MASKER` | 117 | 0,01 ms |
| `SCAN_NODE_TABLE` Scope (m), sous le masque | **71 657** | 6,9 ms |
| `HASH_JOIN_PROBE` | 117 | 5,2 ms |

Le masque retient 117 nœuds, éparpillés dans toute la table `Scope` : presque chaque
bloc du balayage en contient un, et le balayage lit tout. Avec les neuf colonnes que
rend `usages`, ce saut prend 420 à 470 ms par appel ; l'ensemble d'`usages`, 404 ms
(`update` sous `src/storage/table/node_table`) à 1 499 ms (`update` sans chemin), en
médiane (`e2e_mesure_impact::ou_va_le_temps_d_usages_sur_l_index_entier`, lib
`ae3d781aa49f`, `poste mesure`).

## Ce qu'il faut pour le fermer

Pour un masque de quelques centaines d'offsets, une recherche par offset (la ligne
lue directement à son offset, colonne par colonne) doit remplacer le balayage sous
masque ; le seuil se décide sur la part du masque dans la table. Témoin : la mesure
ci-dessus, `usages` à quelques dizaines de millisecondes sur l'index entier, avec les
mêmes lignes rendues. Page :
`extension/rag3weaver/docs/11-octobre-2026-embarquements/06-ou-va-le-temps-d-un-aller-retour.md`.
