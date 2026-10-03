# Le service d'embarquement sur l'autre poste

3 octobre 2026 — session embarquements.

Lucie : « on pourrait, pour ne pas se bloquer de trop, utiliser mon GPU qui
sert pas à seat1 sur l'autre PC, faire servir un modèle d'embedding depuis
là-bas ». C'est fait : trois démons `rag3weaver-embeddings` tournent sur
`luciepc`, joignables d'ici par trois tunnels ssh, et une variable suffit pour
qu'une suite ou un backend s'en serve à la place de la carte de ce poste.

## 1. Ce qui tourne, et où

| | modèle | écoute sur luciepc | port local d'ici (tunnel) |
|---|---|---|---|
| service 1 | granite-278m (768 d) | `127.0.0.1:7878` | `127.0.0.1:7979` |
| service 2 | bge-m3 (1 024 d, dense + creux) | `127.0.0.1:7879` | `127.0.0.1:7980` |
| service 3 | granite-107m (384 d) | `127.0.0.1:7880` | `127.0.0.1:7981` |

Trois services parce qu'un démon sert un modèle : le banc et l'agent de code
demandent granite, les suites e2e ordinaires demandent bge-m3 dense et creux,
le banc de qualité compare aussi granite-107m.

Sur luciepc, rien n'écoute hors de la boucle locale. Le code vient d'un
worktree à part, `~/git_workspaces/rag3db-service`, détaché sur
`origin/master` (`6d6e5588d`) ; l'arbre `~/git_workspaces/rag3db` de là-bas
(branche `mtg-experiments`, fichiers modifiés) n'a reçu qu'un `git fetch`.
**On n'y commite rien et on n'y pousse rien** : l'identité git globale de ce
poste est l'adresse professionnelle. Les poids sont ceux de
`~/.cache/rag3weaver/` là-bas, que le démon trouve seul.

## 2. La carte

luciepc a deux Radeon AI PRO R9700 et un iGPU Intel, et deux sièges.

- `card0`, PCI `0000:07:00.0` : **la carte de `seat1`**, l'écran de jeu.
  udev la marque `ID_SEAT=seat1`, `loginctl seat-status seat1` la liste comme
  son seul périphérique DRM, son HDMI est actif. On n'y touche pas.
- `card2`, PCI `0000:04:00.0` : l'autre R9700, rattachée à `seat0`, aucun
  connecteur actif. **C'est elle que les démons prennent**, par
  `RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0` (wgpu énumère dans l'ordre PCI :
  bus 04 avant bus 07).

Vérifié à chaque lancement par la VRAM : au chargement de granite-278m,
`card2` est passée de 1 426 à 4 986 Mio et `card0` n'a pas bougé ; au
chargement de bge-m3, `card2` a pris 4 433 Mio et `card0` 17. Pour refaire la
vérification :

```bash
cat /sys/class/drm/card{0,2}/device/mem_info_vram_used
```

Le régime là-bas est `plein`, écrit : la carte n'affiche rien, il n'y a rien
à ménager.

## 3. Lancer, arrêter (sur luciepc)

Le shell de luciepc est fish : d'ici, passer les commandes par
`ssh lucied@luciepc bash -s < script`.

Bâtir (3 min 25 s à froid, `-j8` pour ménager le poste) :

```bash
cd ~/git_workspaces/rag3db-service/extension/rag3weaver
cargo build --release --bin rag3weaver-embeddings --features daemon,burn-embedder -j8
```

Lancer les trois, détachés :

```bash
BIN=~/git_workspaces/rag3db-service/extension/rag3weaver/target/release/rag3weaver-embeddings
RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0 RAG3WEAVER_REGIME=plein RAG3WEAVER_EMBED_MODEL=granite-278m \
  setsid nohup $BIN --adresse 127.0.0.1:7878 > ~/.cache/rag3weaver/service-embeddings.log 2>&1 < /dev/null &
RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0 RAG3WEAVER_REGIME=plein RAG3WEAVER_EMBED_MODEL=bge-m3 \
  setsid nohup $BIN --adresse 127.0.0.1:7879 > ~/.cache/rag3weaver/service-embeddings-bge-m3.log 2>&1 < /dev/null &
RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0 RAG3WEAVER_REGIME=plein RAG3WEAVER_EMBED_MODEL=granite-107m \
  setsid nohup $BIN --adresse 127.0.0.1:7880 > ~/.cache/rag3weaver/service-embeddings-granite-107m.log 2>&1 < /dev/null &
```

Chacun annonce dans son journal la carte prise et « à l'écoute sur … » après
quelques secondes de chauffe.

**L'adresse d'écoute est l'argument `--adresse`**, pas une variable : le
binaire ne lit pas `RAG3WEAVER_EMBEDDINGS_ADDR` (le knowledge dump de
l'optimiseur dit le contraire, c'est une erreur ; cette variable n'est lue
que par `tests/common`).

Arrêter, proprement :

```bash
kill -TERM $(pidof rag3weaver-embeddings)      # les trois
ss -ltnp | grep -E ':78(78|79|80)'             # qui tient quel port, pour n'en arrêter qu'un
```

Pour mettre le service à jour : `git fetch origin` dans
`~/git_workspaces/rag3db`, `git checkout --detach origin/master` dans
`rag3db-service`, rebâtir, arrêter, relancer.

## 4. Les tunnels (sur ce poste)

```bash
ssh -f -N -o ExitOnForwardFailure=yes -o ServerAliveInterval=30 -L 127.0.0.1:7979:127.0.0.1:7878 lucied@luciepc
ssh -f -N -o ExitOnForwardFailure=yes -o ServerAliveInterval=30 -L 127.0.0.1:7980:127.0.0.1:7879 lucied@luciepc
ssh -f -N -o ExitOnForwardFailure=yes -o ServerAliveInterval=30 -L 127.0.0.1:7981:127.0.0.1:7880 lucied@luciepc
curl -s http://127.0.0.1:7979/sante    # dit le modèle servi, la dimension, la précision
curl -s http://127.0.0.1:7980/sante
```

Les ports locaux ne sont pas 7878 exprès : c'est l'adresse du démon local que
les suites lancent et remplacent.

## 5. S'en servir d'ici

**Une variable : `RAG3WEAVER_EMBED_SERVICE`**, une adresse ou plusieurs
séparées par des virgules.

```bash
export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981
```

Le client demande à chaque adresse son identité et prend celle qui sert le
modèle voulu. Si aucune ne le sert, c'est un refus qui dit ce que chacune
sert — pas des vecteurs d'un autre modèle dans un index, et pas un repli
silencieux sur la carte d'ici, que la variable existe pour épargner. Le
client **s'attache et rien d'autre** : il ne lance, n'arrête ni ne remplace
jamais un service désigné par cette variable.

> La variable vit dans le code depuis la branche `regime-carte-partagee` :
> une session bâtie d'avant sa fusion ne l'honore pas.

Qui l'honore :

- **Les suites e2e** (`tests/common`) : `BGE_M3`, `GRANITE_278M`,
  `GRANITE_107M` passent par le service quand la variable est posée. La
  batterie complète du 3 octobre : 374 tests verts, 0 rouge, en 20 minutes.
- **Le banc** : il prend ses modèles par ces mêmes statics, donc il suit sans
  rien de plus.
- **Les backends** (`rag3weaver-backend`) : dans `embeddings`, laisser
  `address` vide avec `provider: daemon` ; l'adresse vient alors de la
  variable, choisie par `model`. Une `address` écrite dans le manifeste
  garde le dernier mot. Les scripts Python (`test_backend_persistence`,
  `_snapshot`, `_lifecycle_batch`, `test_migration_v8`) vident l'adresse du
  gabarit quand la variable est posée.
- **Une ingestion par le code** : `DaemonEmbedder::from_service("granite-278m")`
  rend `None` si la variable n'est pas posée, sinon le client ou le refus.

Qui reste sur la carte d'ici, variable posée ou non, parce que c'est
l'embarqueur, le démon ou la vitesse de ce poste qu'ils éprouvent :
`e2e_burn_*`, `e2e_demon_embeddings`, `e2e_mesure_ingestion_code`,
`e2e_banc_bge_m3`. La liste et sa raison sont dans `tests/common`
(`SUITES_LOCALES`) et en tête de chaque suite. Les modèles que le démon ne
sert pas (MiniLM, les relecteurs, l'OCR) restent locaux aussi.

**À ne pas faire** : pointer `RAG3WEAVER_EMBEDDINGS_ADDR` sur un tunnel.
Cette variable-là désigne *notre* démon local ; `tests/common` y remplace un
démon d'une autre construction en lui envoyant `/quitter` — il arrêterait le
service distant.

**Deux arbres qui jouent des e2e en même temps** se disputent le démon
local de `127.0.0.1:7878` : chacun trouve celui de l'autre « périmé » et le
remplace (arrivé le 3 octobre, entre cette batterie et celle de l'arbre
principal). Les suites restées locales en lancent un ; donner à chaque arbre
son port : `RAG3WEAVER_EMBEDDINGS_ADDR=127.0.0.1:7890`.

Il existe une autre voie, sans rien de tout cela : un service d'embarquement
compatible OpenAI, par `HttpEmbedder` (`provider: compatible` dans le
manifeste), quand le poste ne suffit pas et qu'on préfère un service tiers.

## 6. La preuve

- **Parité** : 32 phrases (trois courtes, vingt-neuf morceaux de 900
  caractères de code) embarquées par granite-278m ici, sur l'iGPU du Z13, et
  là-bas, sur la R9700, toutes deux en Flex32. Écart absolu maximal
  **4,1e-5**, médian 3,5e-5, cosinus minimal 0,99999993. Deux cartes
  différentes en demi-précision : l'écart attendu, pas zéro.
- **Débit là-bas, vu d'ici par le tunnel** : 2 048 morceaux de 1 200
  caractères du code de la crate, par requêtes de 256, en 16,4 s — 125
  morceaux par seconde, **148 000 caractères par seconde**, sérialisation
  JSON et tunnel compris. Non mesuré dans les mêmes conditions sur l'iGPU
  d'ici : la mesure aurait figé l'écran une à deux minutes.

## 7. Après un redémarrage

**De luciepc** : les démons ne se relancent pas seuls. Refaire le § 3
(lancer), vérifier la carte (§ 2), puis d'ici vérifier `/sante` — les
tunnels, eux, sont tombés avec la machine : refaire le § 4.

**De ce poste** : les démons tournent toujours là-bas ; refaire seulement
les tunnels (§ 4) et reposer la variable (§ 5).

**Un tunnel tombé** se voit à `curl` qui refuse la connexion, ou à une suite
qui échoue sur « aucun service ne sert … ne répond pas » : relancer la ligne
`ssh -f -N …` correspondante.

## 8. Ce qui reste ouvert

- Les démons ne sont pas des services systemd : un redémarrage de luciepc
  demande la relance à la main. À faire si l'usage dure.
- Trois modèles chargés tiennent environ 10 Gio sur les 31 de la carte.
- Une enveloppe d'embarqueur doit relayer `distant()` : celle des tests ne
  le faisait pas, et le client cadençait ses rafales pour une carte qui
  n'était pas la sienne (`e2e_charge_ingestion` : 1 019 s, puis 123 s).
- Le service distant tourne le code de master, sans le régulateur de rafale
  de la branche : sans effet là-bas, la carte n'affiche rien.
