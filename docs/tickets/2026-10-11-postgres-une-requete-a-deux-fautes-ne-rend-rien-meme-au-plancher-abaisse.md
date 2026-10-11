# PostgreSQL : une requête à deux fautes ne rend rien, même au plancher de rappel abaissé — le banc des seuils est rouge à sa première exécution

- **état** : ouvert (11 octobre 2026, première exécution réelle d'`e2e_postgres` sur le conteneur de Lucie)
- **gravité** : réponse fausse (un rappel attendu ne vient pas) ; ou un banc dont l'attente est fausse — à trancher
- **atteignable en service** : sur un backend PostgreSQL, oui ; rag3db n'est pas concerné
- **touche rag3weaver** : oui (le moteur de texte de PostgreSQL et ses seuils)

## Ce que c'est

`e2e_postgres::ou_vit_la_frontiere_entre_le_vrai_et_le_bruit` est le seul rouge des
vingt tests de la suite : 19 passés, 1 échoué, à sa toute première exécution (la suite
n'avait jamais tourné, faute de conteneur sur les postes).

Le banc mesure la frontière entre de vraies requêtes et du bruit. Au plancher de rappel
par défaut (0,6), puis abaissé à 0,3. **Une requête dégradée, « sauterau baroqe » (deux
fautes, attendu `Clavecin`), ne rend rien aux deux planchers**, alors qu'à 0,3 le bruit
proche, lui, remonte (jusqu'à 0,8012). Le banc affirme qu'une fois le rappel ouvert, le
bruit proche reste sous la pire vraie requête : avec un 0,0000 en face, l'assertion tombe.

Les cinq autres requêtes dégradées passent (0,8876 à 1,0).

## Ce n'est pas l'échappement des noms

Rejoué sur le commit **juste avant** l'échappement (`60d1afe00^`), même lib : même rouge,
même requête, même 0,0000. Le défaut, ou l'attente fausse, est antérieur à l'échappement
et au `Select` PostgreSQL.

## La recette minimale

```
RAG3WEAVER_PG=… cargo test --features …,postgres --test e2e -- --ignored \
  e2e_postgres::ou_vit_la_frontiere_entre_le_vrai_et_le_bruit
```

Conteneur `pgvector/pgvector:pg17`, port 5433 (la recette en tête d'`e2e_postgres`).

## Ce qu'il faut pour le fermer

Trancher entre deux lectures, que le banc ne sépare pas :
1. **le plancher ne s'applique pas à cette requête** : deux mots, chacun mal orthographié ;
   la similarité par trigrammes de chaque mot passe peut-être sous le seuil propre de
   `pg_trgm` (`similarity_threshold` de la session, ou l'opérateur `%`), qui ne suit pas
   le plancher de rappel du catalogue — et c'est un défaut ;
2. **l'attente du banc est fausse** pour une requête à deux fautes de cette longueur, et
   c'est elle qui se corrige, en le disant.

Le banc imprime déjà sa mesure entière ; la lire contre le texte SQL émis pour cette
requête départage.
