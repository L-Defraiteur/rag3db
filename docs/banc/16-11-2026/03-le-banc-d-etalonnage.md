# Le banc d'étalonnage du débit d'embarquement : la conception (16 novembre 2026)

Le ticket est `docs/banc/tickets/2026-11-16-le-banc-d-etalonnage-du-debit-d-embarquement.md`. Le mot de Lucie
(15 novembre, 21 h) : « traduire par un bench d'avance à quoi correspondent 6 secondes vs combien de lots à
embarquer ». Cette page dit ce qui existe, ce que le banc mesurera et comment l'hôte s'en servira. Elle se
soumet à c7 et à embarquement.

**Accord de c7 (16 novembre, 11 h 3x)** : les pas 1 à 4 du § 8 ; le pas 5 attend la clé d'embarquement. Le § 2
est une conception, **à relire par Lucie**.

## 1. Ce qui existe sur master

Relu dans le code de master le 16 novembre (chemins sous `extension/rag3weaver/src/`).

**Deux débits, qui ne se parlent pas.**

| | le débit de l'embarquement | le débit de l'attente de 6 s |
|---|---|---|
| unité | caractères par seconde (`estimate.rs`, `Rate`) | millisecondes par fichier (`catalog/retard.rs`) |
| clé | `embedding_rate:{modèle}:{origine}`, dans `_catalog_meta` (`catalog/progress.rs`, `rate_key`) | `index_rate_ms_per_file`, une par catalogue (`catalog.rs`, `INDEX_RATE_KEY`) |
| origine | `"service"` ou `"local"` (`embedder_origin`) : **pas l'adresse, pas la carte, pas le poste** | la cellule (`Scope`) en mémoire |
| écrit par | la fin d'un `index` qui embarque 512 morceaux ou plus (`note_measured_embedding_rate`, `index_nodes.rs`), ou une sonde (`probe_embedding_rate`) | chaque lot de réindexation (`observe`, moyenne glissante 0,7 et 0,3) |
| mesure | le temps de toute la boucle de dette : réclamation, appel et écriture. Les caractères sont estimés : morceaux × taille moyenne | le temps où le verrou du catalogue est tenu pour le lot : lecture, analyse, écriture, plein texte |
| lu par | `Estimate` (l'outil `estimate`) et le « reste environ N min » (`IndexProgress::line`) | `wait_for_a_lagging_index` (`backend.rs`) : attendre sous `search.index_wait_s`, ou répondre « index pas prêt, ~N s » |

**Le constat qui compte** : l'attente de 6 s **ne compte pas l'embarquement**.
- Dans `dataflow/reindex_nodes.rs`, un fichier sort du registre de retard (`lag.done`) dès que son texte est
  écrit.
- Son embarquement vient après, hors du verrou (`embarquer_le_retard_hors_verrou`), et n'est pas dans le temps
  observé.
- Une recherche qui attend « l'index » reçoit donc le texte à jour, mais peut manquer les vecteurs des morceaux
  neufs le temps de leur appel au service. Le commentaire du code le dit : « les morceaux neufs manquent à la
  recherche vectorielle le temps de cet appel ».

**Ce que les services annoncent** (`/sante`, relu à luciepc le 16 novembre) : le modèle, la dimension, les lots
conseillés (granite-278m `[128, 512]`, bge-m3 `[32, 512]`), `max_chars_per_text` (12 000) et les formes de
chauffe. **Aucun débit.**

**Les traces qui donnent déjà un débit après coup** :
- la ligne `[montre] puis {n} morceau(x) embarqué(s) hors du verrou : {ms} ms` (`reindex_nodes.rs`), une passe
  d'au plus 512 morceaux ;
- la provenance des vecteurs au journal d'index (`dire_la_provenance`, `index_nodes.rs`) : « provenance des
  vecteurs : service 127.0.0.1:7879 : N morceau(x) ». Elle n'a pas de durée.

**Le découpeur** : 1 500 octets par morceau au plus (`chunker.rs`, `max_size`). Le lot d'envoi est borné par
`LotBudget` (`embedder.rs`), à partir du conseil du service, de `RAG3WEAVER_EMBED_CHAR_BUDGET` et du régime.

## 2. Ce que 6 secondes valent : le modèle proposé

Le temps avant que la vue soit prête, pour une recherche :

```
attente = fichiers en retard × ms par fichier            (le texte : ce qui existe)
        + morceaux dus × caractères par morceau ÷ débit   (les vecteurs : ce qui manque)
```

- **Le premier terme** existe et reste tel quel.
- **Le second** est ce que le banc fournit. Le débit est celui **du service qui servira la vue, depuis le
  poste où tourne l'hôte**, et non une valeur par modèle.
- **« 6 s en lots »** se lit alors : `lots = 6 × débit ÷ (caractères par morceau × morceaux par lot)`. Le banc
  rend ce nombre par service, par poste et par taille de lot, sous la forme d'une table (§ 4).

Une recherche en bm25 seul n'a besoin que du premier terme. Une recherche avec le signal vector a besoin des
deux. Le choix de compter ou non le second dans l'attente est la question 1 (§ 7).

## 3. Le banc

**Le jeu, figé** :
- des textes réels : 4 096 `ScopeContent_Chunk._text` tirés d'une copie de B′
  (`banc-parite/migration-bprime-20261116`, l'index neuf du 16 novembre), avec une graine fixe ;
- leur distribution de longueurs est celle du dépôt, avec une médiane d'environ 670 caractères ;
- un second jeu de morceaux pleins, de 1 500 caractères (la taille du découpeur), pour la borne haute ;
- les deux jeux sont écrits dans un fichier JSON, avec leur sha256, hors du dépôt (le texte vient du code de
  Lucie).

**La mesure, par le vrai client** : le banc passe par le client de service de rag3weaver, celui que l'hôte
utilise, avec son découpage en lots (`LotBudget`, `stable_batches`), et pas par des `curl` bruts. Sinon il
mesurerait un autre chemin que celui de l'index. Il prend la forme d'un test ignoré du crate,
`tests/bench_embedding_rate.rs`, qui écrit un JSON :
- **les services** : 7878 (granite-278m) et 7879 (bge-m3). Le 7880 (granite-107m) est froid à cette date : le
  banc ne le réveille pas, il le note ;
- **les chemins** : à luciepc, en local (`127.0.0.1:787x`) ; depuis le portable, par les tunnels
  (`127.0.0.1:797x`). Un embarqueur en processus sur le CPU, seulement si embarquement le juge utile (question 3) ;
- **les tailles de lot** : 1, 8, 32, 128 et 512 textes, bornées par le lot conseillé du service ;
- **le protocole** :
  - une passe de chauffe à part, non comptée ;
  - puis trois répétitions par point ;
  - le régime courant (`RAG3WEAVER_GPU_DUTY` et `RAG3WEAVER_EMBED_CHAR_BUDGET` non posés) est relevé et noté ;
  - aucune autre passe lourde n'embarque sur card2 pendant la mesure : `poste mesure`, annoncé à c7 ;
- **les sorties** : morceaux par seconde, caractères par seconde et lots par seconde, en p50 et p90 ; le temps
  du premier lot ; et les morceaux que 6 s embarquent.

**La preuve de l'artefact, avant tout chiffre** :
- `/sante` relu : modèle, dimension, `max_chars_per_text`, chemin de l'exécutable ;
- le sha256 du binaire de service ;
- le commit du client.

Une mesure dont le service n'est pas celui annoncé ne compte pas.

## 4. Ce que le banc rend

Une table par poste et par service :

| poste → service | modèle | lot | morceaux/s (p50) | caractères/s (p50) | morceaux en 6 s | lots de 512 en 6 s | fichiers moyens du dépôt en 6 s |
|---|---|---|---|---|---|---|---|
| luciepc → 127.0.0.1:7878 | granite-278m | 128 | à mesurer | | | | |
| portable → tunnel 7979 | granite-278m | 128 | à mesurer | | | | |
| … | | | | | | | |

La dernière colonne est celle que Lucie lira : les morceaux embarqués en 6 s, divisés par le nombre médian de
morceaux d'un fichier du dépôt. Ce nombre médian est relevé sur la même copie de B′.

**La vérification contre l'index réel** :
- le débit du banc se compare à celui des lignes `[montre] puis N morceau(x) embarqué(s) hors du verrou` d'une
  réindexation réelle sur le même service ;
- il se compare aussi à `embedding_rate:{modèle}:service`, que l'index a noté. Ce dernier compte aussi les
  écritures et une taille moyenne de morceau, donc il doit être plus bas ;
- un écart de plus de 30 % se dit, avec sa cause.

## 5. Où le débit s'écrit

La décision de l'orchestration du 14 novembre : le banc écrit au même endroit que l'index, une seule source pour
l'hôte. La clé actuelle (`embedding_rate:{modèle}:service`) confond tous les services d'un même modèle et tous
les postes. La proposition, **à décider par embarquement**, qui tient la table par service :
- la clé `embedding_rate:{modèle}:{adresse}@{poste}`, par exemple `granite-278m:127.0.0.1:7878@LuciePC` (décidée par embarquement, § 7) ;
- `note_measured_embedding_rate` et le banc l'écrivent tous les deux ; `known_embedding_rate` la lit d'abord, puis
  se rabat sur l'ancienne clé ;
- le banc ne touche au catalogue qu'une fois cette clé décidée. D'ici là, il rend son JSON.

## 6. Comment arbre lira le reste

Pour le point 7 de la vision (une vue par worktree), l'estimation par vue :
1. les fichiers en retard de la vue, dans le registre de retard, × ms par fichier : ce qui existe ;
2. les morceaux dus de la vue, par le compte de `count_marqueur_manquant` restreint à ses fichiers, × la taille
   moyenne de ses morceaux ÷ le débit du service de la vue (§ 5) ;
3. la somme se compare à `search.index_wait_s` ;
4. le débit observé corrige celui du banc au fil des passes, par la même moyenne glissante que `observe`. Le
   banc donne la valeur de départ, et l'hôte ne garde pas de constante écrite à la main.

C'est à arbre et à recherche de dire si ce découpage leur convient. Le banc fournit les débits, pas le code de
l'estimation.

## 7. Les questions, numérotées (aucune ne bloque la page)

1. **À c7 et recherche** : l'attente de 6 s doit-elle compter les vecteurs dus, quand la recherche utilise le
   signal vector ? Aujourd'hui elle ne les compte pas. **Réponse de c7 : oui.** Avec le signal vector, l'attente
   compte les vecteurs dus, car une réponse sans ceux des morceaux neufs n'est pas complète. En bm25 seul, le
   premier terme suffit. c7 le porte à Lucie et à recherche.
2. **À embarquement** : la clé par adresse et par poste du § 5, ou une autre forme ?
   **Réponse d'embarquement : oui, `embedding_rate:{modèle}:{adresse}@{poste}`.**
   - Une activation a l'adresse du fournisseur pris : la carte, le service de repli ou l'activation de repli.
   - Le tunnel compte à part, et c'est voulu : `127.0.0.1:7979@lucie-tablette` et
     `127.0.0.1:7878@LuciePC` sont le même service, mais pas le même débit vu du client.
   - Un embarqueur en processus s'écrit `local@{poste}`.
   - L'ancienne clé se lit en repli et ne s'écrit plus.
   Le banc écrit sous cette clé dès que sa lecture est sur master
   (`docs/embarquement/tickets/2026-11-16-le-debit-d-embarquement-par-service-pour-le-seuil-de-6-s.md`).
3. **À embarquement** : faut-il mesurer aussi un embarqueur CPU en processus, celui d'un poste sans service ?
   **Réponse d'embarquement : pas maintenant.** Aucun manifeste en service ne l'emploie, et les tests n'ont pas de
   vrai repli CPU (`2026-11-15-un-repli-cpu-des-tests-sans-vulkan.md`). Il se mesurera le jour où un manifeste
   déclarera `provider: local`, sous `local@{poste}`.
4. **À c7** : le 7880 froid à mesurer ou non. Le réveiller tient la card2 ; le banc ne le fait pas sans mot.
   **Réponse de c7 : non.** Le 7880 reste froid et se note.

## 8. Le plan, après accord

| pas | quoi | taille |
|---|---|---|
| 1 | le jeu figé, et son sha256 | 1 h |
| 2 | `tests/bench_embedding_rate.rs`, avec son témoin rouge d'abord : un faux service à débit connu (`tests/common/fake_sse` ou un faux serveur d'embarquement) dont le banc doit retrouver le débit à 5 % près | 2 h |
| 3 | la mesure, à luciepc et depuis le portable, sous `poste mesure` | 1 h 30 |
| 4 | la vérification contre `[montre]` et la clé de l'index, puis la table de cette page | 1 h |
| 5 | l'écriture au catalogue, une fois la clé décidée par embarquement | dans son lot |
