# Un backend lâché après un point de reprise échoué ne se rouvre plus

- **État** : ouvert (14 novembre 2026), relevé par archi sous sa passe de la fermeture qui tient ;
  le premier commit rouge trouvé par bissection
- **Famille** : produit
- **Gravité** : perte de service — après un point de reprise échoué, l'hôte lâche le backend et le
  prochain appel ne peut plus l'ouvrir : chaque lecture et chaque écriture échoue, jusqu'au
  redémarrage de l'hôte. Les données validées sont en base ; ce n'est pas une perte de données.
- **Atteignable en service** : oui, à tout point de reprise qui échoue sous un hôte (disque plein,
  erreur d'E/S) : le chemin « la base doit être rouverte » de l'hôte ne rouvre plus.
- **Touche rag3weaver** : oui (`src/rag3db_connection.rs`, le registre des écrivains)
- **Pour** : arbre principal (1e9bf5089e)

## Ce qu'on voit

`e2e_demon_hote::une_base_a_rouvrir_ne_lache_que_son_backend` rougit à la l. 280, « A rouvert lit
ce qui était validé ». C'est l'assertion de surface : l'aide `fiche()` du test rend `None` sur tout
`ok: false` de `get_fiche` et avale l'erreur. Instrumentée (une ligne, après le `host.call` de
`fiche()`) :

    eprintln!("DIAG get_fiche({titre}) sur {} : {r}", manifest.display());

elle dit, sur le `get_fiche(A, "avant")` qui suit la réouverture :

```
{"error":"connection error: …/rouvrir-a-…/base.rag3db is already open for writing in this process — 1 référence(s) à l'instance précédente tenue(s) après 10s : un catalogue, un index plein texte ou une tâche de fond la retient encore","ok":false,"transient":false}
```

Le journal juste avant : « la base doit être rouverte (Runtime exception: A checkpoint of this
database failed, … (injecté par le crochet de test)) — le backend est lâché », puis « [backend]
arrêt au Drop : la base doit être rouverte ».

## La bissection

Le test seul (`./run_e2e.sh --test e2e_demon_hote une_base_a_rouvrir_ne_lache_que_son_backend`),
au portable, lib `0c41f9fe6`, `RAG3WEAVER_MOTEUR_ANCIEN=1` :

- dernier vert archivé au poste : le 13 novembre à 08 h 01 (lib `29a7bc52a`) ;
- vert à master `4f6f80ef4` (13 nov., 8 h) et à `966a4d903` ;
- rouge à `6a63c50ca`, et à tout ce qui suit (`2a1f52c21`, `42c84cb77`, `a646bf215`, master de ce
  soir) ; rouge aussi sous la lib `29a7bc52a` et sous `e93710f36` : ce n'est pas la lib.

Entre `966a4d903` et `6a63c50ca`, un seul commit de code : `1e9bf5089e` (13 nov., 16 h 51),
« fix(raccord-physique) : l'écrivain du raccord passe par le registre des écrivains, et les photos
d'octets attendent la fin de la fermeture ».

## Ce qui reste à savoir

Ce qui tient la référence après le lâcher d'un backend sur un point de reprise échoué : le registre
OPEN_FOR_WRITING, ou une tâche (plein texte, planificateur) que la fermeture en échec ne rend pas.
La lecture d'arbre principal (14 novembre, 20 h) : le crochet de test lui-même. `FailedCheckpointHook`
tient un `Arc<ReopenState>`, que le test garde pour toute sa durée, et le registre attend que la base
**et** son `ReopenState` soient tombés : 10 s, puis le refus. Le correctif en cours (branche
`arbre-principal/crochet-faible`) : le crochet tient un `Weak`. Réserve : c'est `2f6c346439` (17 h 07)
qui ajoute le `ReopenState` à l'attente, et il n'est **pas** un ancêtre de `6a63c50ca`, déjà rouge ;
à `1e9bf5089e` seul, ce que le crochet tient alors (la base, ou une structure qui la tient) est à
vérifier par le même correctif.

## Ce qui le fermerait

La réouverture qui réussit après un point de reprise échoué ; le témoin
`une_base_a_rouvrir_ne_lache_que_son_backend` vert ; et `e2e_demon_hote` dans le verdict d'une
bascule de lib (ce rouge y a dormi depuis le 13, la bascule ne jouant que e2e_code et e2e_lectures).
