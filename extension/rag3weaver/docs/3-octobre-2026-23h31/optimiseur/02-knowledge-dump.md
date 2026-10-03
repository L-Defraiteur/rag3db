# Optimiseur — ce que cette session sait

**Mis à jour le 3 octobre 2026 à 23 h 45.** Trois parties : le moteur burn et
nos forks, les envois préparés pour tracel-ai, les modèles de décision. Le
relevé du 2 octobre (`../../2-octobre-2026-01h07/optimiseur/02-knowledge-dump.md`)
reste la référence pour le moteur lui-même ; ce qui suit dit ce qui a changé
ou ce qui s'est ajouté.

## 1. Le moteur burn et nos forks

- **Ce qu'il charge.** Des graphes ONNX traduits en Rust par `burn-onnx`
  (`extension/rag3weaver/generated/`), puis leurs poids. Aujourd'hui : des
  XLM-RoBERTa (granite-278m par défaut, granite-107m, BGE-M3, le reranker
  `bge-reranker-v2-m3`), des MiniLM, et PP-OCRv6. Les rerankers sont des
  cross-encodeurs à tête d'un logit.
- **Les forks.** `L-Defraiteur/{burn,cubecl,cubek}`, branche
  `rag3weaver/pre.3`, sur burn 0.22.0-pre.3. Sept défauts de Flex32 y sont
  corrigés ; ce sont eux que les envois proposent à l'amont.
- **Flex32.** Stockage en f32, produits matriciels et attention en f16 sur
  les matrices coopératives de la carte, accumulation en f32.
- **Ce qui n'y est pas.** Aucun ModernBERT. Le charger demanderait de
  traduire son graphe par `burn-onnx` (non essayé) et d'ajouter son motif
  d'attention à `patch_attention.py`, sans quoi il tourne par la voie lente.
- **Les modèles de quelques milliards de paramètres ne passent pas par ce
  moteur** : ils se servent par `llama-server`.

## 2. Les envois préparés pour tracel-ai

Tout est dans `../../optimiseur/3-octobre-2026-14h13/` : l'étude du ton
(01), les textes, la ligne de déclaration, les questions probables d'un
mainteneur et une fiche par correctif (02), les patches (`patches/`).

### Les cinq envois

| | dépôt | envoi | ce qu'il corrige | preuve |
|---|---|---|---|---|
| A | cubecl | PR | Flex32 n'est pas inscrit dans le backend Vulkan, alors que WGSL l'inscrit | test unitaire sur le runtime SPIR-V |
| B | cubek | PR | le masque d'attention matérialisé est lu avec `seq_kv` au lieu du vrai pas de ligne : faux pour un masque de remplissage diffusé | deux tests |
| C | cubek | PR | matmul : une sortie Flex32 n'accumule pas en f32, donc aucune configuration à matrices coopératives ne convient | un test ; 42 tests du crate |
| D | burn | issue | deux défauts reproductibles sur leur `main` en WGSL, et la flash attention en question | — |
| E | burn | PR, quatre commits | `try_cast` vers Flex32 rend des données étiquetées F32 ; `float_from_data` refuse Flex32 ; le pool de locaux de la fusion numérote F32 et Flex32 à part alors qu'ils partagent un registre | trois états pour `from_data` ; `y * hard_sigmoid(gate)` rend 0,25 au lieu de 1 |

### Ce que nous savons de leurs usages

- **burn** : `CONTRIBUTING.md` autorise le code écrit avec une IA, n'exige
  aucune déclaration, exige que l'autrice comprenne et défende chaque
  changement. Une ligne de déclaration sobre est bien reçue (plusieurs PR
  d'extérieurs fusionnées en septembre en portent une). Ce qui ferme une PR :
  des réponses de revue collées depuis un agent.
- **Forme qui passe** : titre en commit conventionnel ; la première phrase
  dit ce qui casse pour l'utilisateur ; un test de régression nommé et la
  phrase « it fails on `main` and passes with this change » ; la liste exacte
  de ce qui a été lancé. Corps médian d'une PR d'extérieur : 27 lignes.
- **Les cases du gabarit ne sont pas exigées, la franchise si.** Les PR
  compagnes que demandent les gabarits de cubek et de cubecl ne sont faites
  que pour les grosses PR ; pour une petite, un tableau avant/après de la
  suite burn lancée en local est accepté.
- **Délais** : burn répond dans la journée aux petits correctifs ; cubek en
  une semaine environ ; cubecl en trois jours. Un compte neuf attend qu'un
  mainteneur approuve la CI.

### Faits utiles pour une revue

- burn a un test qui affirme que Flex32 n'est **pas** supporté sur Vulkan
  (`should_support_vulkan_dtypes`, `crates/burn-wgpu/src/lib.rs`) ; il échoue
  avec cubecl corrigé tant que son assertion n'est pas retournée.
- `into_data()` d'un tenseur Flex32 rend des données étiquetées F32 : nous
  n'y avons pas touché.
- `from_data` sans type explicite ramène les données au type par défaut du
  device : c'est le contrat, pas un défaut.
- Précédents chez eux : burn#3551 (des cas Flex32 manquants, groupés) ;
  cubek#639 (l'accumulation en f32 pour f16, bf16 et flex32, déjà faite pour
  le pooling).
- Cinq tests de leur suite échouent sur cet iGPU avant comme après
  (`test_mean_dims_2d`, quatre `remainder`) : non signalés.
- Les chiffres de débit cités dans les textes datent de septembre, sur
  l'ancien poste (RDNA4) et burn 0.22.0-pre.3 ; ils sont étiquetés ainsi.

### Où sont les choses

| quoi | où | durable ? |
|---|---|---|
| les branches d'envoi et toutes les branches intermédiaires | `.vault/forks/pr-{burn,cubek,cubecl}.bundle` | oui, hors git |
| les patches des envois | `../../optimiseur/3-octobre-2026-14h13/patches/` | oui, dans git |
| les clones de travail et les worktrees | le scratchpad de la session, sous `/tmp` | **non** : perdus au redémarrage |
| les dossiers de compilation | `~/.cache/rag3weaver-amont/target-199` (44 Go), `~/.cache/rag3weaver-build/optimiseur/` (2,6 Go) | supprimables : ils se rebâtissent |
| la demande au support GitHub et son script de contrôle | `.vault/demande-support-github-forks.md`, `.vault/controle-purge-forks.sh` | oui, hors git |
| Rust 1.99.0 | `rustup`, à côté de la toolchain par défaut | — |

## 3. Les modèles de décision

Tout est dans `../../optimiseur/3-octobre-2026-20h55/`. Lire d'abord
`00-ou-en-est-la-question.md`.

### Les candidats, vérifiés à la source

| candidat | famille | français | prêt sans entraînement | ce qu'il faut en savoir |
|---|---|---|---|---|
| Jev (TypeSafe AI) | fermé, API | « primary language: English » | API seule | trois primitives que tous copient : oui/non, choix, score |
| Laya multilingue | encodeur, 322 M | déclaré | oui | l'auteur le dit faible sans affinage ; mesuré : ne juge pas |
| Decide | encodeur, 149 M | non | oui | runtime en Go seulement |
| GLiNER2.5-multi-Decide | encodeur, 287 M | déclaré | oui | trouvé hors de la liste ; mesuré : ne juge pas |
| OpenDecider | encodeur 395 M, ou LoRA sur Qwen 4 Md | non | oui | — |
| NeoHorse-Jev-4B | Qwen 4 Md, tête pointeur | non évalué | oui | reprend le code de Kev |
| **JevK5** | Qwen3.5 4 Md, lecture des logits des lettres | se dit anglais, lit le français | oui | GGUF officiel, tourne par `llama-server` |
| Kev | Qwen3.5 4 Md, tête pointeur | non | oui | le plus rigoureux sur la preuve ; tête lisible par son seul runtime |

Tous sous Apache-2.0, code et poids ; tous ont moins de trois semaines.

### Ce qui tient, sur tous les jeux

- La liste fermée est tenue (99,9 % de la probabilité sur les lettres
  offertes) et tout est déterministe.
- Ranger un texte parmi des sujets existants est un problème d'ordre : le
  cosinus de granite sur le **texte complet** des sujets (nom, description,
  deux textes rangés) met le bon en premier 14/16, 19/32, 22/30 ; JevK5 12,
  15, 20 ; avec les noms seuls, le cosinus fait 8, 7, 15.
- Un ordre n'est pas un verdict : le cosinus note une contradiction 0,85 et
  une redite 0,83. JevK5 ne prend aucune contradiction pour une redite.
- La façon de poser la question pèse plus que le modèle.
- Laya et GLiNER ne jugent pas une paire, dans aucune forme.

### Ce qui ne tient sur aucun

- **Décider de créer un sujet.** Le sujet neuf nommé avec les mots du texte
  est choisi à tort ; « aucun de ces sujets » n'est presque jamais choisi ; la
  faiblesse du meilleur score ne sépare pas (AUC 0,53 à 0,80).
- **Un seuil qui se transporte** : 0,50 pour un critère, 0,07 pour un autre ;
  réglé sur 20 paires, il laisse 6 fusions à tort sur 40 ailleurs.
- **L'étiquette « variante »** : huit sur quinze contredites avec constance.
- **Une forme qui gagne partout** : 15 sur 16 sur un jeu, 13 sur 32 sur un
  autre.

### Ce qui a été essayé sans succès

- Le critère en français : moins bon que l'anglais, même pour des textes
  français.
- Des exemples dans le critère : sans effet.
- Deux oui/non enchaînés ; « sous-sujet » à la place de « variante » ; deux
  temps (« un existant convient-il ? » puis « lequel ? ») : moins bien qu'un
  seul choix.
- Ajouter aux sujets existants des textes déjà rangés, annoncés « Texts
  already filed under it » : fait baisser ; annoncés « For example, among
  many other texts » : presque neutre.
- JevK5 en taille 2B : à peine plus rapide sur CPU, nettement moins bon sur
  les mémoires.
- GLiNER dans la forme « ranger un texte » : 2 à 7 sur 16.

### Ce qu'on a appris sur la manière de s'en servir

- Le modèle reçoit un JSON `evidence` / `criterion` / `options`, chaque option
  portant une lettre ; on lit les log-probabilités du jeton suivant.
- L'ordre des options déplace des verdicts (17 à 14 justes sur 20 selon
  l'ordre) : un nœud doit fixer l'ordre ou en moyenner deux.
- Tout ce qui allonge les options existantes profite à l'option courte.
- Pour les mémoires, montrer le pourquoi en plus de l'énoncé aide JevK5
  (5 redites reconnues sur 6 au lieu de 3) et gêne le cosinus.
- Latence d'une décision JevK5 : 230 ms sur carte (Radeon AI PRO R9700, par
  tunnel), 330 ms à 430 jetons, 1 955 ms sur CPU à huit fils.

### Les jeux

| jeu | fichier dans `banc/` | d'où il vient | taille |
|---|---|---|---|
| paires de sujets et de mémoires | `jeu.json` | écrit et étiqueté par cette session, non relu | 42 + 20 paires, 6 décisions à dix candidats |
| rangement | `rangement.json` | règles de travail réelles, rangées par cette session | 10 sujets, 24 textes |
| journal | `journal.json`, bâti par `batir_journal.py` | les entrées du journal des chantiers sous leurs titres de section ; aucune étiquette posée à la main | 5 sections, 47 cas |
| public | `public.json`, tiré par `tirer_public.py` (graine 2026) | MASSIVE, annotations humaines, Apache-2.0 sur la reprise utilisée | 10 scénarios, 42 textes |
| contrôle du banc de la mémoire | recopié dans `banc4.py` et `final.py` | `scripts/test_banc_memoire.py` | 48 paires |

### Rejouer une mesure

L'environnement est sous `~/.cache/rag3weaver-build/decision/` : un venv
Python 3.12 (`venv/`), les poids (`hf/`), les sources de JevK5
(`jevk5-src/`), les scripts (`banc/`, copie de ceux du dépôt).

```
cd ~/.cache/rag3weaver-build/decision/banc
export HF_HOME=../hf HF_HUB_OFFLINE=1 CUDA_VISIBLE_DEVICES="" TOKENIZERS_PARALLELISM=false
# JevK5 sur CPU : un serveur, puis le banc
llama-server -m <fichier.gguf> -c 8192 -ngl 0 --device none -t 8 --host 127.0.0.1 --port 8091
JEVK5_URL=http://127.0.0.1:8091 ../venv/bin/python final.py <étiquette>
```

Les scripts lisent l'adresse du serveur dans `JEVK5_URL`. Températures de
lecture, données par la carte du modèle : 1,367 pour le 4B v0.3, 1,42 pour le
2B. Les scripts commités gardent des chemins absolus vers ce dossier.

### Ce qui dort sur le disque, et ce qu'on peut supprimer

| dossier | taille | à garder ? |
|---|---|---|
| `~/.cache/rag3weaver-build/decision/hf/` | 8,8 Go | supprimable : tout se retélécharge sans jeton. Contient JevK5 4B Q4 (2,7 Go) et 2B Q8 (2,0 Go), Laya multilingue, GLiNER, et les copies Hugging Face de granite-278m et du reranker |
| `…/decision/venv/` et `uv-cache/` | 2,4 Go | supprimables : `installer.sh` les refait |
| `…/decision/banc/` | 1,3 Mo | copié dans le dépôt ; supprimable |
| `~/.cache/rag3weaver-amont/target-199` | 44 Go | supprimable : compilation de burn avec Rust 1.99.0. À garder seulement si l'envoi est proche, pour ne pas la refaire |
| `~/.cache/rag3weaver-build/optimiseur/` | 2,6 Go | supprimable |

Le service JevK5 de l'autre poste (`luciepc`, par tunnel sur le port 7982)
appartient à la session embarquements ; cette session ne s'en sert plus.

### Ce qu'on ne sait pas

- Ce que tout cela donne sur de vrais sujets de la base, rangés par
  quelqu'un d'autre.
- Si le verdict sur les mémoires tient : deux contradictions seulement ont
  été jouées hors de la main de cette session.
- Ce que vaudrait un modèle plus gros (JevK5-9B, Kev).
- L'accord avec un jugement humain : aucune étiquette n'a été relue.
