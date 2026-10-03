# Les modèles de décision, vérifiés à la source

**3 octobre 2026, session « optimiseur ».** Pour le nœud de décision de la
mémoire longue ([la vision](../../3-octobre-2026-20h37/02-vision-memoire-longue.md)) :
un critère en clair, une chose, des candidats, et en retour un choix dans une
liste fermée avec une probabilité. Une recherche web avait donné des noms que
personne n'avait vérifiés. Ce document est le temps 1 : **des faits lus dans
les dépôts, aucune mesure**. Aucun poids n'a été téléchargé, aucun modèle
lancé, aucune API appelée ; aucun compte, aucun envoi.

Ce qui est « lu » l'a été dans les fichiers (`config.json`, `LICENSE`, code,
carte du modèle) par l'API publique de GitHub et de Hugging Face. **Tous les
chiffres de performance cités sont des affirmations d'auteurs** : aucun n'a
été reproduit.

## En bref

- **Tous les dépôts existent**, tous ont moins de trois semaines (le plus
  ancien date du 17 septembre), tous publient un point de contrôle déjà
  entraîné qui prend le critère et les options **à l'appel** : aucun n'est
  écarté par le critère « rien à entraîner nous-mêmes ».
- **Le français est le vrai filtre.** Un seul des six se déclare multilingue :
  Laya, par son point de contrôle `laya-multilingual`. Les cinq autres disent
  « English only » ou ne disent rien.
- **Trois descriptions de départ étaient fausses** : JevK5 n'est pas une
  lecture de logits sur un Qwen intact (il a ses poids) ; la taille « nano »
  d'OpenDecider n'est pas un Qwen mais un encodeur ; NeoHorse-Jev reprend le
  code et la tête de Kev.
- **À mesurer au temps 2** : `laya-multilingual` et `GLiNER2.5-multi-Decide`
  (petits encodeurs multilingues), JevK5-4B (quelques milliards, lecture de
  probabilités), contre **ce que nous avons déjà** (cosinus granite et
  reranker multilingue), qui ne coûte rien à garder.

## Le tableau

| candidat | famille | base | paramètres, poids | licence (code ; poids) | français | prêt tel quel, critère à l'appel | les candidats | la probabilité | formats, runtime |
|---|---|---|---|---|---|---|---|---|---|
| **Jev** (TypeSafe AI) | fermé | non publiée | non publiés | SDK MIT ; modèle fermé, API seule | « English is the primary training language » | API | une requête, jusqu'à 255 options | rendue par l'API | aucun poids |
| **Laya** anglais | petit encodeur | ModernBERT-large | 421 M, 843 Mo | Apache-2.0 ; Apache-2.0 | non (« English only on root ») | oui, mais l'auteur : « a fast base to specialise, not a zero-shot decision engine » | tous dans une entrée, une passe | tête maison lue aux `[MASK]`, softmax, température par type | safetensors ; script d'export ONNX ; Python |
| **Laya multilingue** | petit encodeur | mmBERT-base (MIT) | 322 M, 644 Mo | Apache-2.0 ; Apache-2.0 | **oui, déclaré** (51 langues dont `fr`) | oui ; températures laissées à 1,0 (non calibré) | tous dans une entrée | idem | safetensors ; ONNX par un tiers (`onnx-community`, 1,29 Go en fp32) |
| **Decide** (jmwri) | petit encodeur | ModernBERT-base | 149 M, 597 Mo | Apache-2.0 ; Apache-2.0 « subject to the training-data terms » | **non** (« English only ») | oui | tous dans une entrée, avec un masque d'attention et des positions sur mesure | tête lue aux `[MASK]`, softmax, température par type | safetensors seul ; **runtime Go uniquement** |
| **OpenDecider nano** | petit encodeur | Ettin-400m (MIT) | 395 M, 790 Mo | Apache-2.0 ; Apache-2.0 | non (« English only so far ») | oui | tous dans une entrée | tête lue aux `[MASK]`, softmax | safetensors ; ONNX quantifié publié le 3 octobre ; Python |
| **OpenDecider small** | quelques milliards | Qwen3-4B-Instruct | LoRA de 132 Mo sur 4,0 Md | Apache-2.0 ; Apache-2.0 | carte `en` | oui | options en lettres dans le prompt, 26 au plus | softmax sur les logits des lettres | GGUF officiel (2,5 et 4,3 Go) ; llama.cpp, vLLM |
| **NeoHorse-Jev-4B** | quelques milliards | Qwen3.5-4B, par NeoHorse-1-4B | 9,1 Go + tête de 5 Mo | Apache-2.0 ; Apache-2.0 | non déclaré, non évalué | oui | par question | tête pointeur, softmax ; « calibration results have not been reported » | GGUF officiel + runtime dédié |
| **JevK5** (4B) | quelques milliards | Qwen3.5-4B | 4,2 Md, 8,4 Go | Apache-2.0 ; Apache-2.0 | **non** (« English only ») | oui (LoRA fusionnée) | lettres A à P, 16 par passe, au-delà plusieurs passes | softmax sur les logits des lettres, température 1,22 | GGUF officiel (2,7 à 4,5 Go) ; llama.cpp |
| **Kev** (4B) | quelques milliards | Qwen3.5-4B-Base | LoRA de 130 Mo + `head.pt` de 5 Mo | Apache-2.0 ; Apache-2.0 | **non** (« Languages : English ») | oui | une ligne par question, 1 à 255 options | tête pointeur, softmax, température 2,41 | runtime du dépôt (CUDA, MLX) ; GGUF par un tiers |

Trouvé en cherchant, hors de la liste de départ, et ajouté parce qu'il est le
seul autre encodeur déclaré multilingue :

| candidat | famille | base | paramètres | licence | français | prêt tel quel | formats, runtime |
|---|---|---|---|---|---|---|---|
| **GLiNER2.5-multi-Decide** (Fastino) | petit encodeur | gliner2.5-multi-v1 | 287 M | Apache-2.0 ; Apache-2.0 | **oui, déclaré** (« Use it when the text is not English ») | oui : « Pass any label set at call time » | safetensors ; bibliothèque Python `gliner2` |

## Ce qu'il faut savoir de chacun

**Jev.** Trois primitives, lues dans sa documentation publique : `noul`
(oui/non, une valeur entre 0 et 1), `choice` (jusqu'à 255 options, chacune
avec sa description), `score` (2 à 10 niveaux ordonnés). C'est ce vocabulaire
que tous les autres copient. Architecture, taille et données ne sont pas
publiées.

**Laya.** Le plus visible (30 000 étoiles sur GitHub en deux semaines, 1 362
commits, une trentaine de contributeurs) et le plus franc sur ses limites. La
carte dit que les points de contrôle de base sont « near chance on
typed-decisions zero-shot — 0.362 […] against a 0.318 random and 0.461
majority-class baseline », que `noul` « can follow its option labels instead
of the state », et que `score` est « the weakest primitive ». Sa comparaison
à Jev (0,766 contre 0,727) porte sur un point de contrôle affiné sur le jeu
du banc, avec des chiffres de Jev « third-party published, never measured
here ». Pour le français, l'auteur donne 0,600 d'exactitude sur MASSIVE à 20
options (0,710 en anglais). Les données d'entraînement des points de contrôle
de base ne sont nommées dans aucun fichier lu.

**Decide.** Petit, propre, honnête (47,6 % sur les tâches jamais vues, dit
par l'auteur), mais anglais seulement, trois commits, un auteur, et un
runtime en Go sans chemin Python ni ONNX. La licence des poids renvoie aux
licences de 35 jeux d'entraînement, dont plusieurs « share-alike » ou non
précisées.

**OpenDecider.** Deux modèles sous un nom : un encodeur (nano) et des LoRA
sur Qwen (small à large). Pas de script d'entraînement dans le dépôt. Se dit
devant Laya (0,796 contre 0,766) et derrière Jev sur les décisions générales
(0,680 contre 0,730).

**NeoHorse-Jev-4B.** Données d'entraînement non décrites, calibration non
mesurée, runtime dédié. Le moins documenté des six.

**JevK5.** Lit les logits du jeton suivant, restreints aux lettres des
options : c'est son mode de sortie, pas l'absence d'entraînement. Il publie
aussi la ligne de référence qu'on croyait qu'il était : le même gabarit sur
Qwen3.5-4B **non entraîné** fait 0,613 là où lui fait 0,784 (chiffres de
l'auteur). Entraîné en partie sur des sorties d'un modèle d'OpenAI, « which
govern their use ». Le pur « lecture de logits sur modèle gelé » existe
ailleurs : SemIf (`TheoLeeCJ/SemIf-OpenJev`, MIT), dont JevK5 reprend le
gabarit.

**Kev.** Le plus rigoureux sur la preuve : Jev mesuré sur les mêmes items,
intervalles de confiance, jeu de test verrouillé. Et il dit perdre hors de sa
distribution : « On held-out datasets it trails Jev by 16 points ». Anglais
seulement ; sa tête est un fichier `head.pt` que seul son runtime sait lire.

## Notre moteur burn peut-il charger les petits encodeurs ?

Notre moteur ne charge pas des « familles » de modèles : il charge un graphe
ONNX traduit en Rust par `burn-onnx`, puis ses poids. Il porte aujourd'hui
des XLM-RoBERTa (granite, BGE-M3, le reranker `bge-reranker-v2-m3`) et des
MiniLM. Les rerankers sont déjà des cross-encodeurs à tête d'un logit : la
forme « des jetons en entrée, un score en sortie » existe.

**ModernBERT n'est pas de la même famille que granite.** Positions rotatives
(RoPE) au lieu d'une table, attention locale sur une fenêtre de 128 jetons
dans deux couches sur trois, normalisation avant chaque bloc. Aucun de ces
graphes n'est passé par `burn-onnx` chez nous : **que la traduction marche
est à essayer, pas à supposer.** Notre réécriture de l'attention
(`patch_attention.py`), qui fait gagner la flash attention, reconnaît le
motif de XLM-RoBERTa ; celui de ModernBERT serait à ajouter, faute de quoi le
modèle tourne par la voie lente.

| | Laya multilingue | Decide |
|---|---|---|
| graphe ONNX disponible | oui : export par un tiers, et un script d'export dans le dépôt | **non**, et le runtime est en Go |
| entrées du graphe | cinq : jetons, masque, positions des `[MASK]`, masque des options, type de question | à écrire nous-mêmes |
| ce qui sort de l'ordinaire | tête maison : deux couches de transformeur de plus, lecture aux positions des `[MASK]` | masque d'attention par option et positions qui repartent après le préfixe : un export standard de ModernBERT ne le reproduit pas |
| tokenizer | `tokenizer.json` (vocabulaire de 256 000), lisible par notre crate | `tokenizer.json` de ModernBERT |
| le travail | traduire le graphe, vérifier la parité contre onnxruntime (écart absolu max), écrire l'assemblage de l'entrée et un trait `Decider` à côté de `Reranker` | réécrire le modèle en PyTorch d'après le Go pour l'exporter, puis le même travail |
| verdict | **faisable si `burn-onnx` traduit le graphe** ; quelques jours, à confirmer par un essai | lourd, pour un modèle anglais |

Les modèles de quelques milliards de paramètres ne passent pas par notre
moteur burn : ils se servent par `llama-server`, déjà installé sur le poste.

## Ce que je recommande de mesurer

| candidat | décision | pourquoi |
|---|---|---|
| Laya multilingue | **retenu** | le seul des six à se déclarer multilingue ; ONNX disponible ; l'auteur prévient qu'il est faible sans affinage : la mesure le dira |
| GLiNER2.5-multi-Decide | **retenu** | l'autre petit encodeur multilingue, étiquettes à l'appel, Apache-2.0 ; hors de la liste de départ, vérifié plus légèrement que les autres |
| JevK5-4B | **retenu** | représente la famille « lecture de probabilités » ; GGUF officiel, tourne par `llama-server` sur CPU ; se dit anglais, mais sa base Qwen ne l'est pas : seul un essai dira ce qu'il vaut en français |
| ce que nous avons déjà | **retenu, comme référence** | le cosinus de granite-278m et le score de `bge-reranker-v2-m3`, tous deux multilingues et déjà dans le moteur : si un seuil sur l'un des deux fait aussi bien, il ne faut pas de modèle de plus |
| Decide | écarté | anglais seulement, runtime Go, licence des poids suspendue à 35 jeux de données |
| Laya anglais | écarté | anglais seulement |
| OpenDecider nano | écarté | anglais seulement |
| OpenDecider small | écarté | même famille que JevK5, sur une base Qwen plus ancienne |
| Kev-4B | écarté de la mesure | anglais seulement, et sa tête ne se lit que par son runtime (CUDA ou MLX) : pas de voie CPU simple ; à reprendre si JevK5 convainc |
| NeoHorse-Jev-4B | écarté | données non décrites, calibration non mesurée, français non évalué |
| Jev | hors mesure | fermé ; on n'appelle pas son API |

Le jeu du temps 2 : une quarantaine de paires de sujets tirées du journal des
chantiers, en français et en anglais (même sujet dit autrement, variante,
sans rapport). Pour chacun : exactitude, calibration, latence sur CPU.

## Ce qui n'a pas pu être vérifié

- **Aucune performance.** Rien n'a tourné.
- **Le français des modèles sur Qwen** : leurs cartes disent « English
  only » ; ce que la base permet n'est écrit nulle part dans ce qui a été lu.
- **Les données d'entraînement** des points de contrôle de base de Laya et de
  NeoHorse-Jev-4B : non nommées. On ne peut donc dire ni si du français y
  figure, ni si une licence de données contraint les poids.
- **Le contenu des fichiers de poids** : aucun n'a été ouvert. Les têtes sont
  établies par le code et les `config.json`.
- **La fidélité des exports ONNX faits par des tiers** : leur existence et
  leur taille seulement.
- **La documentation de Jev** a été lue à travers un outil qui résume : ses
  limites (255 options, 10 niveaux) sont à relire à la source si elles
  deviennent décisives.
- **GLiNER2.5-multi-Decide** : carte, licence et taille lues ; son code et sa
  façon de produire une probabilité ne l'ont pas été.
- **Les 30 000 étoiles de Laya en deux semaines** et ses 0 téléchargement
  affichés sur Hugging Face : ce sont les chiffres que rendent les API ; ils
  ne disent rien de sûr sur l'usage réel.
