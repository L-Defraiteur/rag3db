# 10 — Journal du 27 après-midi : ce qui a été lancé, et ce qui est revenu

Le compte rendu détaillé de la session : commandes exactes, paramètres, retours
bruts. Il existe parce que **la moitié des trouvailles de cet après-midi
viennent d'une mesure qui ne collait pas**, et qu'une mesure sans son chiffre
n'est qu'un souvenir.

Les documents [04](04-la-session-tient-l-invite.md) à
[09](09-le-terminal-a-plusieurs.md) disent le *pourquoi*. Celui-ci dit le
*quoi*, avec les nombres.

---

## 1. Le disque : 586 Go, dont 580 dans `target/`

**Déclencheur** — « tu peux regarder ce qui est lourd sur mon disque dans
rag3db ? »

```sh
du -sh . ; du -sh --exclude=.git .[!.]* * | sort -rh | head -20
```

```
586G  .
582G  extension     2,7G  build      319M  dataset     32M  third_party
```

```sh
du -sh extension/rag3weaver/target/*/
```

```
571G  debug/     9,2G  release/     2,1M  sparse-dump/     296K  environment/
```

```sh
du -sh target/debug/*/
```

```
448G  deps/     113G  incremental/     8,5G  build/     1,5G  examples/
```

Regroupement par famille et par âge :

```
librag3weaver-*.a :  45 fichiers,  81 Go   (~1,8 Go pièce)
binaires e2e_*    : 367 fichiers, 285 Go

  0 jour  114,7 Go  137 fichiers
  1 jour   81,2 Go  105 fichiers
  2 jours  88,9 Go  125 fichiers
```

```sh
df -h /home
```

```
/dev/nvme0n1p2  3,7T  3,4T  259G  94% /home
```

**Diagnostic** — cargo suffixe chaque artefact d'une empreinte et ne supprime
jamais les anciennes. **Environ 100 Go par jour** ; les 259 Go restants
tenaient deux jours et demi.

### Le tri à blanc, puis le ménage

Groupement par `(famille, extension)`, on garde le plus récent de chaque :

```
familles     : 75
on garde     :   75 fichiers,  31,6 Go
on supprime  : 2209 fichiers, 366,7 Go
```

Exécution après la fin de la passe (garde-fou : le script refuse de tourner
pendant une passe) :

```
incremental/ : 124.3 Go
deps/        : 2209 fichiers, 366.7 Go — gardé 31.6 Go (75 familles)
total libéré : 491.1 Go
```

```
101G  target/
/dev/nvme0n1p2  3,7T  3,2T  514G  87% /home
```

**Ce qui a été exclu du ménage, et pourquoi** : toutes les dépendances externes.
Serde, burn, tokio et les cinq cents autres sont stables ; les effacer coûterait
une reconstruction complète de l'arbre pour ne rien gagner. C'est exactement là
que la règle naïve « supprime ce qui est vieux » fait mal.

---

## 2. Les cartes graphiques : quatre mesures identiques, et pourquoi

**Déclencheur** — « vraiment l'impression que l'embedding est ce qui slow down
mon pc ».

### 2.1 L'inventaire

```sh
vulkaninfo --summary | grep -E "GPU[0-9]:|deviceName|deviceType"
```

```
GPU0: DISCRETE_GPU   AMD Radeon AI PRO R9700 (RADV GFX1201)
GPU1: DISCRETE_GPU   AMD Radeon AI PRO R9700 (RADV GFX1201)
GPU2: INTEGRATED_GPU Intel(R) Graphics (ARL)
```

```sh
for c in /sys/class/drm/card*-*/status; do ... done
```

```
card0-HDMI-A-3 : branché      card2-DP-6 : branché      card2-HDMI-A-2 : branché
```

`card1` (Intel, pilote `i915`) **ne pilote aucun écran** — et n'expose aucun
`gpu_busy_percent`, ce qui a rendu ma première mesure aveugle de son côté.

```sh
ps -eo args | grep llama-server
```

```
llama-server -m …/Huihui-Qwen3-Coder-30B-A3B-Instruct-abliterated.i1-Q6_K.gguf \
  --device Vulkan1 -ngl 99 --port 8080 -c 131072 --jinja --flash-attn on \
  --cache-type-k q8_0 --cache-type-v q8_0 --cache-ram 2048
```

### 2.2 La mesure qui ne voulait rien dire

```
défaut : 32 s | TOTAL 4 passed | (rien annoncé)
gpu:0  : 10 s | TOTAL 4 passed | (rien annoncé)
gpu:1  :  9 s | TOTAL 4 passed | (rien annoncé)
igpu:0 : 10 s | TOTAL 4 passed | (rien annoncé)
```

Les 32 s du premier passage étaient la compilation. Et « rien annoncé » était
le vrai signal :

```sh
grep -rn "for_role\|BurnRole::" src/ | grep -v burn_device.rs
```

```
(rien)
```

**`BurnDevice::for_role` n'était appelé par personne.** Les quatre variables
d'environnement ne faisaient rien, et les quatre mesures étaient quatre fois la
même exécution. L'exemple `burn_throughput.rs` codait même sa carte en dur
(`DiscreteGpu(0)`), donc la comparaison y était impossible par construction.

### 2.3 La mesure propre

Une seule suite, **tous les rôles épinglés** — sinon on mesure ce que fait le
rôle qu'on a oublié de fixer, ce qui m'est arrivé une fois :

```sh
export RAG3WEAVER_BURN_DEVICE_{EMBEDDER,RERANKER,OCR,LLM}=$D
./run_e2e.sh --summary --test e2e_burn_embedder
```

Occupation maximale des deux cartes AMD, échantillonnée toutes les 200 ms :

| | card0 | card2 |
|---|---|---|
| repos | 0 % | 6 % |
| `gpu:0` | 2 % | **99 %** |
| `gpu:1` | **98 %** | 67 % |
| `igpu:0` | 26 % | 6 % |

**Donc** : `gpu:1` = card0 = la carte de Qwen ; `gpu:0` = card2 = celle qui
pilote les deux écrans de travail. Le défaut tombait sur `gpu:0`.

### 2.4 Le CPU, qui n'était pas coupable

```
16:07:59  cpu global 20 %   e2e_burn_embedder  158 %
16:08:01  cpu global 17 %   e2e_burn_embedder  187 %
16:08:03  cpu global 13 %   e2e_burn_embedder  177 %
```

**1,1 à 1,9 cœur sur 24.** L'hypothèse CPU ne tient pas ; c'est la préemption du
GPU d'affichage qui donne l'impression que la machine rame.

### 2.5 Ce que valait l'iGPU

`examples/burn_throughput.rs`, en **release**, BGE-M3 :

```
gpu:0
 batch  seq     tok/s      ms/doc          batch  seq     tok/s
     1   32       210       152.6              1  128      1353
     4   32       991        32.3              4  128      4128
    16   32      2698        11.9             16  128      7417
    64   32      7550         4.2             64  128      5507
                                               1  512      2629
                                               4  512      6210
                                              16  512      5879
                                              64  512      5378
gpu:1  : crête 7628 tok/s        igpu:0 : 148 → 241 → 121 tok/s
```

**L'iGPU est environ 60× plus lente, et son débit *empire* quand le lot
grandit** — elle est limitée par la bande passante de la RAM système qu'elle
partage. Ma recommandation de l'y mettre était mauvaise ; elle a été retirée.

### 2.6 Le chiffre qui décide de la borne

Les trois crêtes tombent au même endroit :

```
64 × 32  = 2048 jetons → 7550 tok/s
16 × 128 = 2048 jetons → 7417 tok/s
 4 × 512 = 2048 jetons → 6210 tok/s
```

Et au-delà, ça **redescend** : 5 507 à 8 192 jetons, 5 378 à 32 768.

---

## 3. La session : le facteur 24

```sh
cargo test --lib agent::tests::dix_tours -- --nocapture
```

```
[dix tours] sans absorption 900180 caractères, avec 37567
```

Dix tours, neuf résultats de 20 000 caractères, `Stale { max_chars: 2_000,
after_turns: 2 }`. Le témoin est dans le même test que la mesure : un chiffre
seul ne dit rien, deux chiffres mesurés dans la même minute disent tout.

**Réserve inscrite au [doc 08 §11](08-le-compteur.md)** : c'est mesuré en
caractères envoyés, pas en jetons facturés.

---

## 4. Le coût d'un schéma, rendu exécutable

```sh
cargo test --lib le_defaut_hybrid -- --nocapture
```

```
[coût] Symbol (HYBRID)  : 3275 lignes, 3275 chunks, 3275 embeddings, 6550 documents plein texte
[coût] Symbol (déclaré) : 3275 lignes, 3275 documents plein texte
```

Les 6 550 documents plein texte contre 3 275 : l'index vit sur la table
**parente**, et les chunks s'y ajoutent parce que `fulltext_on_chunks` est vrai
par défaut. C'est le piège qui a coûté une nuit, ici en un chiffre.

---

## 5. Trois guetteurs qui tournaient dans le vide

```sh
ps -eo pid,etime,args | grep "until ! pgrep"
```

```
PID  787772   20 h 58 min   until ! pgrep -f "cargo test --release --no-run"
PID 1503746      41 min     until ! pgrep -f "menage.py"
```

**Chacun se trouvait lui-même** : `pgrep -f` compare à la ligne de commande
complète, et celle du guetteur *contient* le motif cherché. Le premier tournait
depuis la veille.

Et ma vérification était fausse pour une raison voisine : `pgrep -f "[c]argo"`
ne se voit pas lui-même — le crochet protège de ça — mais il voyait la ligne de
commande du zombie, qui contenait `cargo test --release --no-run` en toutes
lettres. D'où trois messages où j'ai annoncé « la suite tourne encore » alors
que rien ne tournait.

Un troisième cas, plus tard, de la même famille : le motif de fin de passe
`^  TOTAL` attrapait aussi `  TOTAL : 186.9 ms`, une ligne de profilage. Le
guetteur s'arrêtait au milieu.

> **Trois fois le même défaut** : conclure depuis une correspondance
> approximative au lieu de vérifier la chose elle-même.

---

## 6. Le journal de passe, qui s'effaçait

Symptôme :

```
  e2e_idempotent_registration     21 passed, 1 FAILED
  TOTAL                          275 passed, 1 FAILED
```

Et **aucun moyen de savoir lequel** : `run_e2e.sh` gardait son journal dans un
`mktemp` détruit par un `trap … EXIT`, et seulement dans la branche
`--summary`.

Après correction (`target/e2e-last.log`, les deux branches, le résumé écrit
dedans lui aussi, et les échecs nommés) :

```
grep -E "^test .* FAILED|panicked at" target/e2e-last.log
```

```
thread 'kb_vector_search_survives_migration' panicked at
  tests/e2e_idempotent_registration.rs:830:8
```

Trois commandes pour la cause. Sans le journal, une demi-heure de relance.

**La cause** : mon refus « signal sémantique sans champ de contenu » ne
connaissait qu'une des deux façons d'avoir du contenu. `Note` déclare
`content_for: ["knowledge"]` — son texte part dans les chunks de la base, pas
dans les siens. `has_kb_participation()` existait déjà, juste à côté de la
fonction que j'avais prise.

---

## 7. Le traceur qui dédupliquait en silence

```
assertion `left == right` failed
  left: 16      (entités Trace dans le catalogue)
 right: 18      (événements que le graphe dit avoir enregistrés)
```

`Trace` n'a **pas de `hashsafe`** : son uuid dérive de tous ses champs. Et
`Consumed` tombait dans le cas générique du traceur — résumé « Consumed »,
détail vide. Trois consommations d'un même run étaient donc littéralement
identiques, et ont fusionné en une ligne.

**Dette nommée** : ce n'est pas propre à cet événement. N'importe quels deux
événements identiques disparaissent l'un dans l'autre.

L'assertion qui l'a attrapé compare *ce que le graphe dit avoir écrit* à *ce qui
existe*. C'est le genre qu'on croit redondant.

---

## 8. Le sélecteur de carte, vérifié par une passe entière

```sh
RAG3WEAVER_BURN_DEVICE_{EMBEDDER,RERANKER,OCR}=gpu:1 ./run_e2e.sh --summary
```

```
progression : 14/34 suites, 0 échec(s), card2 (tes écrans) à 3 %
progression : 28/34 suites, 0 échec(s), card2 (tes écrans) à 6 %
TOTAL                          277 passed
```

**Contre 98-100 % le matin même.** C'est la vérification du correctif : pas un
message qui s'affiche, une carte qui reste libre pendant trente minutes de
tests.

---

## 9. Les trois mécanismes construits et jamais branchés

C'est le motif de la journée, et il n'aurait été trouvé par aucune relecture.

| Mécanisme | Écrit le | Découvert par |
|---|---|---|
| `BurnDevice::for_role` + 4 variables | 27 au matin | quatre mesures de cartes rendant le même temps |
| `Postures::describe_for` (bloc d'attentes) | 26 au soir | en cherchant où l'injecter — personne ne l'appelait |
| borne de lot du dense et du sparse | — | le dual du **même nœud** la respectait déjà |

Et un quatrième, plus discret : `FlushConfig::embed_batch_size` est déclaré,
testé pour sa sérialisation, et appliqué nulle part — doublon mort de
`gpu_batch_size`.

---

## 10. Les tests, au fil de l'après-midi

```
812 → 817 → 819 unitaires        277 E2E sur 33 suites
```

Trois passes complètes arrêtées en cours de route, deux par toi, une par moi
pour ne pas attendre quarante minutes sur deux échecs déjà corrigés.

**Une passe incohérente est pire qu'une passe absente** : j'ai compilé trois
fois pendant qu'une passe tournait, et comme elles partagent `target/`, les
suites tardives ne testaient plus le même code que les premières. Le piège est
désormais écrit au cookbook.

---

## 11. Les commits

```
81ca594e1  feat(trace): un participant est une identité, pas une session
3fdee9d3f  docs(27 août): le compteur, et le terminal à plusieurs
5adac3d33  perf(embed): borner les lots par le texte, pas par le nombre
cef6d6b14  fix(burn): le choix de carte existait, personne ne l'appelait
20e04670d  feat(meter): compter ce qui est consommé…
ea166079c  build(e2e): le journal survit, les échecs sont nommés, le ménage passe avant
c0077b504  fix(config): alimenter une base de connaissances compte comme du contenu
f2bdbdc7d  feat(config): le langage de déclaration apprend le coût et l'état
c76e9ee87  docs(27 août): la session, la réputation, le Tamagotchi…
b8c23a1d8  feat(session): absorber ce qui n'a plus à être payé…
```

---

## 12. Ce qui reste ouvert

- Le **balayage fin du seuil** — 8 192 caractères reposent sur trois points.
- La **comparaison llama.cpp**, avec sa réserve : il ne rend que le dense, notre
  chemin calcule dense *et* sparse dans le même forward.
- Le **relevé du compteur** — `Meter::describe()` existe et personne ne
  l'affiche. Ma propre décoration.
- `Trace` sans `hashsafe`.
- Et une passe complète sur le **fil nommé** et le **domaine**, écrits après la
  dernière verte.
