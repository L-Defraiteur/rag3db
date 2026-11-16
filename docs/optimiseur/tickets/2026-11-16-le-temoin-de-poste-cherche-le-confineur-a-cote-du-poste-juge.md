# Le témoin de `poste` cherche le confineur à côté du `poste` qu'il juge

- **État** : ouvert (16 novembre 2026) — à G, dans son prochain lot
- **Famille** : produit (outillage : `extension/rag3weaver/scripts/test_poste.sh`)
- **Gravité** : faux rouge, au seul témoin. Joué contre la copie du cache, un cas rougit sans défaut de `poste`.
- **Atteignable en service** : non
- **Touche rag3weaver** : oui (`scripts/test_poste.sh`)
- **Pour** : G (optimiseur)
- **Signalé par** : G, à la recopie du cache du 16 novembre, 2 h 20

## Ce qu'on voit

`test_poste.sh ~/.cache/rag3weaver-build/poste` rend 34 OK sur 35. Le cas « le seuil de poste est celui du confineur »
lit `confiner_validation.py` dans `$(dirname "$POSTE")`. Le cache n'en a pas, le seuil du confineur est donc « absent ».
Joué depuis le dépôt, le cas est vert, et la valeur copiée au cache (`VALIDATION_MEMORY_MAX=34359738368`) est la bonne.
`test_passe_epinglee.sh` a le même pli pour `required_suites.sh` : contourné en posant la table au cache, alors qu'une
vraie passe la lit dans l'arbre du lot.

## Ce qu'il faudrait

Que les deux témoins lisent le confineur et la table dans le dépôt du témoin lui-même (`$(dirname "$0")`), et non à côté
du script qu'ils jugent. Le témoin de `poste` doit être vert contre la copie du cache, et la table ne doit plus être
posée au cache.
