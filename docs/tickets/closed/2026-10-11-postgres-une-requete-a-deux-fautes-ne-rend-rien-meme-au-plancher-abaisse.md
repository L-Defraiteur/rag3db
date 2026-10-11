# PostgreSQL : une requête à deux fautes ne rend rien, même au plancher de rappel abaissé — le banc des seuils est rouge à sa première exécution

- **état** : corrigé (11 octobre 2026) — le plancher voyage avec la requête (`SET LOCAL` dans sa transaction, sur une seule session) ; témoins `e2e_postgres::le_plancher_de_rappel_tient_sur_chaque_session_du_pool` et `ou_vit_la_frontiere_entre_le_vrai_et_le_bruit`
- **gravité** : réponse fausse (un rappel attendu ne vient pas, selon la session du pool reçue)
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

## Tranché par la mesure directe (11 octobre 2026, dans le conteneur)

Mesuré sur une seule session (`pg8000`, même conteneur), contre le texte de Clavecin
tel qu'il est indexé (« clavecin baroque sautereau registre ») :

| requête | `similarity` | `word_similarity` | `<%` au défaut (0,6) |
|---|---|---|---|
| sauterau baroqe | 0,3000 | **0,5789** | faux |
| sautereau baroque | 0,5000 | 1,0000 | vrai |
| navigaton astronomiqe (Sextant) | 0,5000 | 0,7200 | vrai |

`pg_trgm.similarity_threshold` = 0,3, `pg_trgm.word_similarity_threshold` = 0,6 par défaut.
Après `SET pg_trgm.word_similarity_threshold = 0.3` **sur la même session**,
`'sauterau baroqe' <% 'clavecin baroque sautereau registre'` est **vrai**.

**L'attente du banc est juste ; c'est un défaut.** Le score réel (0,5789) est au-dessus du
plancher abaissé, et l'opérateur l'admet à 0,3. Le dialecte n'utilise pas `%` (il est déjà
à `<%` / `word_similarity`, la bonne question) : la cause est **le pool**.
`PostgresConnection` est un pool `deadpool` ; `PostgresSearchBackend::poser_le_plancher`
joue `SET pg_trgm.word_similarity_threshold = 0.3` **une fois** (`Once`), donc sur une seule
session du pool. Les requêtes suivantes partent sur n'importe quelle session, et les autres
gardent 0,6. Même classe que « la transaction d'un paquet part sur deux sessions du pool ».
Le banc souffre du même mal : ses deux `SET` (0,6 puis 0,3) ne visent qu'une session chacun.
Son 0 à 0,6 est donc juste par accident, et le bruit proche qui monte à 0,3 vient des
requêtes tombées sur la session où le SET a porté.

En service : sur PostgreSQL, le plancher de rappel de 0,3 n'est tenu que sur une session du
pool. Une requête à deux fautes tombe ou non selon la session qu'elle reçoit — une réponse
fausse (« rien ») qui dépend du hasard du pool.

**Pour le fermer** (au choix de qui le prend, aucun n'est fait) :
- poser le seuil sur **chaque** session du pool : `options=-c pg_trgm.word_similarity_threshold=0.3`
  dans la configuration des connexions, ou un crochet à la création d'un objet du pool ;
- ou le porter dans la requête : `word_similarity(q, t) >= $plancher` à côté de `<%`, qui
  garde l'index GIN tant que la session est au plancher ou plus bas ;
- et le banc pose ses deux planchers par le même chemin que le dialecte, pas par un `SET`
  de session.

Le témoin existe déjà : `ou_vit_la_frontiere_entre_le_vrai_et_le_bruit` doit passer au vert.

## Corrigé (11 octobre 2026, session embarquements)

La forme demandée par l'orchestration : **le plancher voyage avec la requête, pas
avec la session.**

- `DbConnection::execute_with_local_settings(sql, params, réglages)` : les réglages
  partent sur la même session et dans la même transaction que l'instruction. Pour
  PostgreSQL, `set_config(nom, valeur, true)` (la forme fonction de `SET LOCAL`),
  dans la transaction épinglée de l'appelant s'il y en a une, sinon dans une
  transaction à elle sur une session du pool (`BEGIN`, réglages, instruction,
  `COMMIT`), qui rentre au pool sans rien garder. Le défaut du trait refuse en le
  disant : un réglage ignoré en silence rendrait une réponse fausse sans erreur.
- Pourquoi pas l'épinglage de `begin` tel quel : il est partagé par toute la
  connexion. Deux recherches concurrentes se seraient disputé la même session
  épinglée, et l'une aurait commité sous l'autre.
- `PostgresSearchBackend` : le `Once` et le `SET` de session sont retirés. Le
  plancher est un champ du backend (`PLANCHER_RAPPEL` = 0,3 par défaut,
  `with_recall_floor` pour un autre), envoyé avec chaque recherche. `<%` reste
  dans la requête et l'index GIN sert toujours.
- Le banc pose ses deux planchers par le même chemin (un backend par plancher).

Témoin rouge d'abord, sur le code d'avant : 4 fils × 3 recherches, puis 12
successives ; **7 sur 24 ne rendaient pas Clavecin**. Après : 24 sur 24, et
`e2e_postgres` passe en entier (21 sur 21). Le banc dit maintenant à 0,6 « rien »
pour la vraie raison (le plancher de PostgreSQL), plus par accident ; à 0,3,
Clavecin à 0,8955, et le vrai se sépare du bruit dans ]0,8012 ; 0,8876[.
