# Rapport de session — optimiseur (moteur burn, démon, modèles, forks)

**Nuit du 1er au 2 octobre 2026.** Session reprise sur le nouveau poste
(`lucie-tablette`, iGPU Radeon 8060S), sous les consignes de la session
d'orchestration. Arrêt demandé par Lucie ; rien n'est envoyé à tracel-ai ni au
support GitHub. Le [02](02-knowledge-dump.md) dit tout ce qu'il faut savoir
pour reprendre ; ce document dit ce qui a été fait et où ça s'est arrêté.

## 1. Ce qui a été fait

### Les sept PR amont, vérifiées contre l'amont

Lucie est d'accord pour proposer nos correctifs Flex32 à tracel-ai, à
condition qu'ils n'aient pas déjà corrigé. Vérifié sur leurs branches
principales du 1er octobre (burn `8942d406`, cubek `fbed329f`, cubecl
`a6321cd4`) : **les sept défauts sont toujours là**, aucune issue ni PR ne les
couvre (burn#4778 décrit la même famille que le masque d'attention, sur
l'autre axe, déjà corrigé). Tableau, textes des PR en anglais et correctifs :
[`docs/optimiseur/2-octobre-2026-00h15/`](../../optimiseur/2-octobre-2026-00h15/01-les-sept-pr-amont.md).

Sept branches locales, un commit chacune au-dessus de leur `main`, en anglais,
sans mention d'IA, signées de l'identité personnelle. Elles vivaient dans le
scratchpad ; elles sont maintenant **à l'abri** :

- `docs/optimiseur/2-octobre-2026-00h15/patches/` : un `.patch` par branche,
  à jour de ce soir, avec `BASES.md` (le commit de leur `main` sur lequel
  chacun s'applique) ;
- `.vault/forks/pr-{burn,cubek,cubecl}.bundle` (hors git), vérifiés.

### L'état de chaque vérification

Une compilation lourde à la fois, `target` à part, `-j8`, sur l'iGPU.

| # | PR | dépôt | état ce soir |
|---|---|---|---|
| 1 | Vulkan déclare Flex32 | cubecl | **vérifiée**. Test ajouté (le type est-il déclaré ?) : rouge sans la ligne, vert avec. `cargo fmt`, `clippy --tests` propres. Suite SPIR-V entière de `cubecl-wgpu` : 797 tests, 0 échec. Constat pour le texte : leurs 169 tests Flex32 passent déjà sans la ligne — les noyaux tournent, seule la déclaration manquait. |
| 2 | `float_from_data` accepte Flex32 | burn | **non vérifiée**. Écrite à la main sur leur `main`. Par leur API publique, sa panique n'est atteignable qu'une fois la 4 en place. |
| 3 | matmul : sortie Flex32 accumule en f32 | cubek | **non vérifiée** (s'applique proprement). Un test rouge/vert demande Flex32 déclaré sur Vulkan, donc la PR 1 en dépendance. |
| 4 | `try_cast` vers Flex32 garde Flex32 | burn | **vérifiée**. Test unitaire rouge sans le correctif (`F32` au lieu de `Flex32`), vert avec ; les 190 tests de `burn-std` passent. Le test lit par l'itérateur : leur `as_slice` refuse une donnée marquée Flex32. |
| 5 | la flash accélérée se lance en Flex32 | burn | **non vérifiée**. Reportée à la main (leur `main` a retiré le paramètre de runtime). La plus discutable sur le fond. |
| 6 | masque d'attention lu avec son vrai pas de ligne | cubek | **vérifiée**. Test ajouté (masque de remplissage diffusé sur `seq_q`, strides explicites, politique stricte) : rouge sans le correctif sur les deux voies (1 689 éléments faux sur 2 048), vert avec. `fmt`, `clippy` propres ; 62 tests d'attention sur Vulkan passent. |
| 7 | fusion : locaux F32 et Flex32 numérotés ensemble | burn | **test écrit, pas joué**. Test unitaire du pool de locaux, dans la branche et dans le patch ; il n'a pas encore été compilé. |

**Là où je me suis arrêtée** : la PR 6 venait d'être vérifiée et son test
intégré à la branche ; le test de la PR 7 était écrit, la compilation de
`burn-cubecl-fusion` pas lancée. Restent, dans l'ordre : 7 (jouer le test), 3,
2, 5, puis `cargo run-checks` sur une branche burn d'intégration.

### Ce que leurs règles demandent, appris en route

- **burn** : cocher « `cargo run-checks` exécuté », et ouvrir une issue avant
  la PR s'il n'en existe pas.
- **cubek et cubecl** : valider chaque PR *avec burn* — une branche burn (et
  cubek, pour cubecl) qui pointe sur le hash de la PR, et une PR burn liée.
- Deux couplages : **2 et 4 vont ensemble** (la 4 rend l'étiquette Flex32
  réelle, la 2 la fait accepter par burn-cubecl) ; **6 avant 5** (la flash
  enfin active en Flex32 est fausse avec un masque diffusé tant que la 6 n'est
  pas en place).

D'où la forme proposée à Lucie, **non tranchée** : trois envois au lieu de
sept — cubecl (1) ; cubek (6 et 3) ; burn : une issue « Flex32 de bout en bout
sur Vulkan », puis une PR d'intégration (2+4, 7, et la 5 si l'issue
l'accueille) qui pointe sur les deux premiers.

### L'adresse retirée des forks, et leur recréation

Les commits de nos forks portaient une adresse qui n'a rien à faire dans un
dépôt public. Fait, avec l'accord de Lucie relayé par l'orchestration :

1. les six commits des trois `rag3weaver/pre.3` recréés à l'identité
   personnelle (mêmes diffs, mêmes messages, mêmes dates d'auteur, diff
   ancien/nouveau vide), poussés avec bail ; les trois `rag3weaver/pre.2`
   supprimées ;
2. l'épinglage de master remonté (`04b5052ec`) : 51 entrées de `Cargo.toml` et
   le `Cargo.lock`, par remplacement de trois hashes et rien d'autre ;
3. Lucie a supprimé les trois forks ; je les ai **recréés** sous son compte
   personnel et j'y ai repoussé les branches depuis des bundles : mêmes hashes
   (burn `21674205`, cubecl `bdf6b77a`, cubek `7ba8affd`), une seule branche à
   nous par fork, journal d'activité sans aucun ancien hash, et un poste neuf
   télécharge les révisions (`cargo fetch --locked` dans un `CARGO_HOME` vide).

**Ce qui reste visible, et pourquoi.** Les neuf anciens commits ne sont plus
sur aucune branche, mais GitHub les sert encore par leur hash — par l'URL des
forks, anciens comme neufs, et par celle des dépôts d'origine : un fork partage
le magasin d'objets de son réseau. Ni la réécriture ni la recréation ne les
retire ; **seule une purge par le support GitHub** le fait. La demande est
rédigée, avec les neuf hashes complets, dans
`.vault/demande-support-github-forks.md` (hors git, exprès), et
`.vault/controle-purge-forks.sh` dira si c'est purgé (dix-huit
« introuvable »). Note publique sans hash :
[`docs/optimiseur/2-octobre-2026-00h15/02`](../../optimiseur/2-octobre-2026-00h15/02-le-retrait-des-anciens-commits.md).

### Les poids granite régénérés

Les `model.bpk` de granite-278m (le modèle par défaut) et granite-107m
n'avaient pas été publiés et sont restés sur l'ancien poste. Régénérés depuis
les `model.onnx` d'IBM (inchangés depuis août 2025, empreintes vérifiées) avec
`burn-onnx 0.22.0-pre.3` : **même taille à l'octet** que ceux de septembre,
code généré identique à celui du dépôt une fois passé par `patch_attention.py`.

**La preuve**, par l'écart absolu maximal contre onnxruntime (CPU, f32) sur 24
phrases fixes (4 à 512 jetons), une phrase par passe, iGPU Radeon 8060S :

| précision | granite-278m | granite-107m |
|---|---|---|
| f32 | 3·10⁻⁷ | 3·10⁻⁷ |
| Flex32 (défaut) | 4,6·10⁻⁵ | 7,6·10⁻⁵ |

Les tests sémantiques rendent les chiffres de septembre à la troisième
décimale. Installés dans `~/.cache/rag3weaver/granite-{278m,107m}/`. Publiés
sur Hugging Face en dépôts **privés** (`Lucie666/granite-embedding-278m-multilingual-burnpack`
et `…-107m-…`), empreintes distantes conformes. Commit `88495d5ba` : le test
contre la référence, les références en fixtures, le script, le README avec les
deux empreintes et la recette rejouée.

## 2. L'accident de cette session, et sa leçon

En générant un document, j'ai passé un script au shell par un heredoc **non
protégé** : le shell a exécuté comme commandes tous les passages entre accents
graves, dont une ligne d'exemple qui ouvrait une PR avec le jeton du compte
personnel. Elle a échoué sur son argument avant toute action. Vérifié
aussitôt, et recoupé par l'orchestration : aucune PR créée sur aucun des sept
dépôts concernés, pour aucun des deux comptes ; compte actif de `gh` et
identité git globale inchangés ; rien n'a compilé.

**La leçon** : un texte qui contient des accents graves ou des substitutions
ne passe jamais par un heredoc non protégé. On l'écrit par un fichier (l'outil
d'écriture), ou par un heredoc à délimiteur entre apostrophes avec les
variables passées en arguments. Et après une faute de ce genre, on vérifie les
effets de bord avant d'affirmer que rien n'a changé, et on le dit tout de
suite.

Une seconde leçon, plus petite : sous fish, une liste dans une variable ne se
découpe pas (`for x in $LISTE`, `env $VARS commande`). Tout script un peu long
va dans un fichier bash.

## 3. Ce qui attend Lucie

1. **La forme des envois** à tracel-ai : sept PR séparées, ou trois envois
   avec une issue burn d'abord (ma proposition, §1).
2. **L'envoi lui-même**, une fois les vérifications finies : rien ne part sans
   son mot. Le compte passe par commande, sans changer l'état du poste.
3. **La demande au support GitHub** (`.vault/demande-support-github-forks.md`) :
   c'est elle qui l'envoie, connectée à son compte personnel.
4. **Le passage en public** des deux dépôts Hugging Face granite.
5. **Les originaux de l'ancien poste** : les `model.bpk` granite du 6
   septembre (empreintes dans `generated/README.md`). Si elle les rapporte, on
   compare ; sinon les régénérés font foi, ils sont prouvés. Un disque de 465 Go
   est branché sur ce poste et non monté : il vient peut-être de là.

## 4. À la reprise, dans l'ordre

1. Lire le [02](02-knowledge-dump.md), puis le journal des chantiers.
2. Si le scratchpad a disparu : recloner les trois dépôts de tracel-ai,
   `git fetch .vault/forks/pr-<dépôt>.bundle` (ou `git am` des patches) pour
   retrouver les sept branches ; poser l'identité **en local** dans chaque
   clone avant le moindre commit.
3. Vérifier que leur `main` n'a pas bougé sous les patches (`git apply --check`).
4. Reprendre les vérifications : 7 (jouer le test unitaire), 3, 2, 5, puis
   `cargo run-checks` (burn), selon la forme que Lucie aura choisie.
5. Avant tout push d'une branche de PR : `git log --format='%ae %ce'` sur la
   branche entière hors amont. Un commit poussé sur un fork est visible dans
   tout le réseau dès le push.
