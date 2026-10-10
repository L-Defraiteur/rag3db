# Le backend de code — un moteur, deux politiques

Deux manifestes du même moteur, qui ne diffèrent que par leur `workspace`
et leurs politiques d'outils :

- **`backend.json`** — la politique de **poste** : l'arbre de travail réel
  (`working_tree`) sous `workspace/`, l'édition sur le disque, les
  commandes derrière la porte en mode `approbation`.
- **`snapshot.json`** — la politique **cloud** : un instantané chargé en
  mémoire à l'ouverture (un dépôt cloné s'y copie), l'édition en mémoire,
  pas de commandes.

Le schéma de code (`File`/`Scope`) **n'est pas déclaré ici** : la vérité
vit dans le moteur (`register_code_schema`), enregistrée à l'ouverture dès
qu'un `workspace` est déclaré (`"index": "code"` le déclare ; absent, rien ne s'enregistre). Un schéma de
payload ne sait pas dire `Text`, une copie JSON dériverait ; les entités
**métier** d'un produit, elles, restent déclaratives.

Les deux manifestes pointent des bases **distinctes** (`code-poste` /
`code-cloud`) : deux backends lancés ensemble ne doivent jamais être deux
écrivains sur une même base — on ne sait pas faire.

**Pas d'index initial ici** : `search_code` sur une base vide ne rend rien
tant que les verbes d'amorçage (`estimate`/`index`, ou `sync_source`) ne
sont pas attachés — seuls les fichiers déjà édités par `edit_file` sont
réindexés au fil de l'eau. Les verbes arrivent avec l'API d'indexation en
deux temps.

Le schéma du workspace se **nomme** (`"index": "code"`) : la vérité d'un
schéma vit dans le moteur, un dossier de documents nommera le sien.

Question ouverte : les descriptions d'outils et le prompt sont en français,
les rendus du moteur en anglais (« File written; … ») — un agent lira les
deux ; à trancher un jour, pas ici.

Préparer : créer `workspace/` (ou pointer `root` ailleurs), poser
`RAG3WEAVER_EMBED_SERVICE` ou l'adresse du démon dans `embeddings.address`.
Les verbes d'amorçage `estimate`/`index` arrivent avec l'API d'indexation
en deux temps (doc du 3 octobre, « indexer ce dépôt ») ; en attendant, un
fichier édité est réindexé à l'édition.

La tuyauterie s'éprouve sans modèle : `scripts/test_backend_code.py`.

## Le signal creux — une option, pas le défaut (4 octobre 2026)

**Position de Lucie** : le creux appris sur du texte faiblit sur les
jetons hors vocabulaire (les identifiants), et hors domaine il peut
faire pire que BM25 — « une option, bien faite », la parité avec ce que
propose Qdrant, mais pas la préconisation par défaut tant que les
expériences ne l'ont pas tranchée (la session optimiseur en propose, le
banc les jouera). Le manifeste d'exemple ne l'active donc PAS. Pour
l'activer, deux clés : `models.sparse` (bge-m3 par le service) **et**
`workspace.index_signals` qui monte `Scope` en `bm25+vector+sparse` — un
signal de plus sur une entité se déclare, il ne découle pas du modèle
tout seul. Le poids `sparse:0.4` des `default_weights` reste déclaré :
il ne pèse rien sans signal. La fusion le pèse par les
`default_weights` mesurés (`bm25:0.45, vector:0.55, sparse:0.4` — banc du
4 octobre : phrases 0,340 → 0,369, identifiants 0,850 → 0,900 sur granite
dense + creux bge).

**Ce que le creux coûte** : un second embarquement (bge-m3) de chaque
morceau à l'indexation — au banc, la passe complète de `src/`
(~7 600 scopes, dense + creux par le service) tient dans les mêmes
minutes que la passe dense seule, le service absorbant les deux ; sur un
dépôt entier, compter le même ordre de grandeur que les vecteurs. Le
plein texte reste cherchable d'abord ; le creux entre dans la dette comme
les vecteurs (`sparse_missing` est compté) — son reflet dans
`index_state_for` est en discussion avec la session embarquements.

**Une base déjà indexée sans creux** : déclarer le signal ne casse rien —
les morceaux existants ont leur `sparse_missing`, la dette les rattrape
au fil des passes d'indexation, et la fusion ne compte pas un signal
absent comme un zéro : une liste creuse vide ne pèse pas, les autres
signaux répondent seuls en attendant.

**Le cloud** : pas de creux déclaré dans `snapshot.json` pour l'instant —
le service d'embarquement du cloud n'est pas défini, et le creux est un
gain de qualité, pas une condition de marche. Le déclarer quand l'infra
cloud existera tient en deux clés.

## Les verrous applicatifs — `workspace.locks_via` (11 octobre 2026)

L'analyse relève seule les verrous pris sur un **champ mutex** (gardes
RAII C++, `.lock()` / `.read()` / `.write()` sur un champ déclaré mutex ou
RwLock) : la relation LOCKS du scope vers le symbole `Classe::champ`. Un
gestionnaire de verrous applicatif, lui, ne verrouille aucun champ : il
**s'appelle**. Le dépôt déclare ses fonctions de verrou :

```json
"workspace": { "source": "working_tree", "root": "…", "index": "code",
               "locks_via": ["acquireLock", "acquireLocks", "lockRowForWrite"] }
```

Un scope qui appelle l'une d'elles reçoit LOCKS vers le symbole de la
fonction (genre `lock`, ligne du site), que l'appel soit résolu dans le
fichier ou au rendez-vous ; la correspondance se fait par le nom nu. Sans
`index: "code"`, la clé est refusée au chargement. **LOCKS par appel,
marque `via`** : `via = "appel"` pour un verrou posé par une fonction
déclarée, `via = "champ"` pour une garde sur un mutex ; une base d'avant
lit la marque nulle. `impact` et `callees` les rendent dans la même
section, « Verrous pris sur le chemin ».

**L'exemple de rag3db** (le gestionnaire de verrous de la transaction) :
`acquireLock`, `acquireLocks`, `lockRowForWrite`. Ne pas y mettre
`lockKeyOf` ni `lockKeyOfRow` : elles **calculent la clé** d'un verrou
(`pkVector.getAsValue(pos)->toString()`), elles ne le prennent pas — les
déclarer ferait dire à `impact` que `NodeTable::update` « prend »
`lockKeyOfRow`. Sur `rag3db/src`, ces trois déclarations ajoutent 9 LOCKS
aux 169 relevés sur des mutex ; `callees` de `NodeTable::update` rend
`lockRowForWrite` (au départ) → `acquireLock` → `acquireLocks`.
