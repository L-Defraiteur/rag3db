# Sous des écritures de BLOB, la mémoire résidente du processus dépasse nettement le tampon du moteur

- **État** : ouvert (11 octobre 2026, seconde session cœur C++)
- **Gravité** : mémoire (le processus tient plus que le tampon qu'on lui a donné ; rien n'est
  perdu ni faux)
- **Atteignable en service** : oui, dès qu'un processus écrit de gros BLOB (le magasin de blobs
  du plein texte de rag3weaver, l'index creux)
- **Touche rag3weaver** : oui. Le tampon du moteur sert de plafond quand on dimensionne un poste
  ou un service ; ce plafond n'est pas tenu.

## Ce que c'est

Un tampon du moteur de 2 Gio n'empêche pas le processus de monter à 3,2 à 5 Go de mémoire
résidente quand il écrit de gros BLOB. Une part de la mémoire est tenue hors du tampon.

## Les chiffres

Sonde Rust `extension/rag3weaver/tests/sonde_tampon_et_blobs.rs` (la connexion seule et du Cypher
brut : une table `(clé, BLOB)`, des lots `UNWIND … MERGE … SET` de 32 Mo de blobs de 4 Mo),
`RAG3DB_BUFFER_POOL_SIZE=2147483648`, 3 Go écrits :

| moteur | mode | résultat | mémoire résidente au plus haut |
|---|---|---|---|
| `e3daa2836` (4 octobre) | `remplace` | passe, 8 Go en 621 s | 5,0 Go |
| master `5c5feb527` (11 octobre) | `remplace` | passe, 3 Go en 195 s | 3,4 Go |
| segments de chaînes bornés (11 octobre) | `ajoute` | passe, 3 Go en 65 s | 3,2 Go |
| segments de chaînes bornés (11 octobre) | `remplace` | passe, 3 Go en 70 s | 3,7 Go |

Une sonde C++ du même jour (des transactions de 8 blobs de 4 Mo, un point de reprise après
chacune) voit la mémoire **du tampon** retomber à 6 Mo après chaque point de reprise : ce qui
dépasse n'est donc pas compté par le tampon.

## Ce qui est tenu hors du tampon, d'après la lecture (non mesuré)

- Les valeurs des paramètres et des vecteurs du processeur de requêtes (les blobs de la requête
  `UNWIND $items` vivent dans les `ValueVector`, dont le dépassement va au tas, pas au tampon).
- La table de dédoublonnage des chaînes du dictionnaire (`DictionaryChunk::indexTable`, des index
  de 4 octets, sur le tas).
- Côté rag3weaver, la copie des blobs en Rust avant leur passage au moteur.
- L'allocateur : les grosses allocations rendues au tas ne reviennent pas forcément au système
  (fragmentation, seuils de `malloc`).

Aucune de ces pistes n'est mesurée : la première chose à faire est de répartir la mémoire
résidente (heaptrack ou `malloc_info` sur la sonde C++).

## Recette minimale

La sonde Rust ci-dessus. En C++ : la sonde jetable
`~/.cache/rag3db-tickets-notes/blobs_checkpoint_probe_scratch_test.cpp`, avec un relevé de la
mémoire résidente (`/proc/self/status`) à côté de celui du tampon.

## Témoin

Aucun : c'est une mesure. Il faudrait borner la mémoire résidente d'un processus de test (un
plafond de `cgroup`, ou une comparaison RSS contre le tampon avec une marge).

## Pour le fermer

Savoir ce qui est tenu hors du tampon, puis le borner ou le compter dans le tampon. Ou bien, si
c'est inévitable (les paramètres d'une requête), le dire dans la documentation du
dimensionnement : prévoir le tampon plus la taille des lots écrits.
