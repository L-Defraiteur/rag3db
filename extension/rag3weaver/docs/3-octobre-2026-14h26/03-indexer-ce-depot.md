# « Indexer ce dépôt » — proposition, sans code

3 octobre 2026 — session embarquements. Pour Lucie, portée par
l'orchestration avec la page de la session recherche sur les outils de
l'agent.

Deux produits, un moteur : un agent de code en cloud qui télécharge un dépôt
git, un agent en ligne de commande qui ingère ce qu'il y a sur le disque. Le
verbe qui leur manque à tous deux est le même : **dire ce que ça va coûter,
puis indexer en fond**. Rien de ce qui suit n'est propre au code : la source
est une liste de fichiers, qu'ils soient du C++ ou des comptes rendus.

## 1. Ce qu'on propose : deux verbes et un reçu

Les noms sont ceux de la surface d'outils de la session recherche.

| Verbe | Ce qu'il fait | Ce qu'il rend |
|---|---|---|
| `estimate(source)` | Lit la liste des fichiers et leurs tailles, n'écrit rien. | Fichiers retenus et écartés (avec la raison), caractères, modèle choisi et pourquoi, durées prévues par niveau, et si une confirmation sera demandée. |
| `index(source)` | Ouvre la synchronisation, écrit, rend la main. | Un **reçu**. Le travail avance en fond. |
| `wait(reçu)` (existe déjà) | Attend, ou lit l'état sans attendre. | « plein texte prêt, vecteurs à 40 % », puis le rapport de fin. |

Pas de troisième outil pour l'avancement : l'indexation est une tâche de fond
comme une autre, son reçu se suit par le `wait` que les agents ont déjà.

## 2. L'estimation

**Ce qu'on compte**, sans rien lire d'autre que les noms et les tailles : la
politique d'ingestion existe (`code::verdict` : liste d'extensions interdites,
seuil de 128 Kio pour le texte brut) et dit pour chaque fichier « retenu » ou
« écarté, parce que ». Sur ce dépôt-ci (6 110 fichiers suivis, 427 Mo), c'est ce qui
sépare 35 Mo de code de 392 Mo d'autre chose, surtout des CSV et du Parquet.

**Le modèle** : l'heuristique du premier index, telle qu'elle est sur master
— granite-278m, sauf plus de 50 000 fichiers, carte faible ou absente, ou
seule carte du poste portant l'affichage, où l'on part en granite-107m. Elle
rend sa raison en une phrase ; l'estimation la montre telle quelle. Quand un
service d'embarquement est attaché (`RAG3WEAVER_EMBED_SERVICE`), les deux
déclencheurs qui parlent de la carte d'ici ne s'appliquent pas : ce n'est pas
elle qui calcule. **C'est un changement à faire dans l'heuristique**, elle ne
le sait pas aujourd'hui.

**La durée** : pas de table de débits par modèle ni par carte — elle serait
fausse au premier poste différent. Une **sonde** : une soixantaine de
morceaux pris dans la source, embarqués par l'embarqueur réellement attaché
(la carte d'ici sous son régulateur, ou le service), une à deux secondes. Le
débit mesuré est gardé en base avec le modèle et l'origine ; les estimations
suivantes le relisent, et chaque indexation le corrige.

**Le seuil de confirmation** est déclaré dans la configuration, en durée
prévue (proposé : cinq minutes de vecteurs) — pas en nombre de fichiers, qui
ne dit rien d'un poste à l'autre. Au-delà, `index` sans `confirm: true` est
refusé **avant d'avoir rien écrit**, par le harnais de validation qui existe
(`ToolHarness`, règle `before`) : l'appelant reçoit `accepted: false`,
`executed: false`, et l'estimation dans l'erreur.

> Une précision sur ce qui existe : le harnais sait **refuser par
> validation**, il n'a pas de notion de confirmation. La confirmation est une
> règle de plus à écrire dans ce harnais, pas un mécanisme à inventer.

## 3. L'indexation en fond, plein texte d'abord

`index` s'appuie sur la brique que la session de l'arbre principal prépare,
`sync_source(catalog, source, options)` : elle ouvre elle-même la session de
synchronisation, traite les fichiers par paquets, et finit en deux temps (le
plan de ce qui sera retiré, puis son application). Elle n'existe pas encore ;
sa forme est fixée et cette proposition se branche dessus en trois points.

1. **Ce qu'on exige de chaque paquet** : `RECHERCHE_TEXTE` — la donnée et le
   plein texte tout de suite, les vecteurs laissés en dette. Les quatre
   niveaux existent (`Disponibilites` : donnée, plein texte, sparse, dense),
   l'écriture « jusqu'à un niveau » aussi (`ingest_entities_jusqu_a`) ; il
   manque seulement que l'ingestion de code la prenne en option.
2. **La dette se rembourse seule**, et entre processus : `embarquer_le_retard`
   réclame des lots de morceaux sans vecteur (`_embed_claim`, périmé après
   une minute), donc deux processus ne paient pas deux fois, et un processus
   tué reprend où il en était — la dette est retrouvée en base à l'ouverture.
3. **Le reçu** est l'identifiant de la session de synchronisation, déjà
   persisté en base : il survit à l'arrêt du processus.

### Ce qui manque vraiment

- **Un état d'avancement lisible.** Le compte existe mais il est privé
  (`count_marqueur_manquant`) et ne sort que sous forme d'avertissement. À
  rendre public, par table et par niveau : *plein texte : prêt · vecteurs
  denses : 8 200 sur 20 100 · reste environ 3 min*. Les événements
  d'embarquement sont déclarés (`events.rs`) et jamais émis : on ne propose
  pas de les brancher, un compte en base suffit et vaut pour tous les
  processus.
- **Un ordre pour les vecteurs.** Aujourd'hui les tables sont parcourues dans
  l'ordre d'une table de hachage, 512 morceaux par passe. Proposé, du plus
  simple : d'abord ce qu'une recherche vient de demander (ça existe : une
  lecture qui exige le dense rembourse sa cible), puis les fichiers les plus
  récemment modifiés. Rien de plus fin tant qu'on n'a pas vu le besoin.

### En combien de temps est-on cherchable par mots ?

**Calculé, pas mesuré** : l'ingestion de code ne sait pas encore s'arrêter au
plein texte, il n'y a donc rien à chronométrer sans écrire ce qu'on propose.

| Pour ce dépôt (35 Mo de code, ~70 000 morceaux ; les textes retenus s'y ajoutent) | Durée | D'où vient le chiffre |
|---|---|---|
| Analyse | ~25 s | 3,9 Mo en 2,7 s, mesuré aujourd'hui sur `src/` |
| Écriture et plein texte, sans modèle | ~40 s | 21 778 morceaux en 11 à 12 s (cœur C++, 7 septembre) |
| **Cherchable par mots** | **environ une minute** | somme des deux |
| Vecteurs, par le service de l'autre poste, 278m | ~4 min | 148 000 caractères/s, mesuré aujourd'hui |
| Vecteurs, carte d'ici, 278m, régulateur provisoire | ~30 min | 26 000 jetons/s, un quart du temps sur la carte |
| Vecteurs, carte d'ici, 107m, régulateur provisoire | ~5 min | 148 000 jetons/s, idem |

L'ordre de grandeur est le résultat utile : **une minute pour chercher par
mots, des minutes à une demi-heure pour les vecteurs**. C'est l'écart qui
justifie le plein texte d'abord, et le troisième déclencheur de 107m.

## 4. Les deux politiques

Le mécanisme ne connaît que `FileSource` (lister, lire). La politique décide
quelles sources on a le droit de lui passer.

| | Politique « poste » (ligne de commande) | Politique « cloud » (agent en ligne) |
|---|---|---|
| Dossier local | Lu en place (`WorkingTree`), rien n'est copié. | **Interdit** : pas de chemin du disque. |
| Adresse git | Clonée dans un espace de travail géré. | Clonée dans l'espace de travail de la session — la seule source permise. |
| Révision | Celle qu'on demande, sinon la branche par défaut ; écrite dans le reçu. | Idem, et **obligatoirement résolue en un commit** avant d'indexer. |
| Rafraîchir | Relancer `index` : seuls les fichiers changés sont repris. | `fetch` puis `index` sur la nouvelle révision. |
| En plus | — | Schémas d'adresse permis (https), taille maximale du clone, pas de sous-modules ni de crochets git. |

La source git n'existe pas (`FileSource` porte le commentaire « demain
`git:<sha>` ») : c'est un clone suivi d'un `WorkingTree` sur le dossier
cloné. Les champs `repo` et `revision` sont déjà dans le schéma des fichiers.

## 5. Ce qu'on recommande de coder en premier

1. **`estimate`**, entièrement : compte par `verdict`, modèle et raison,
   sonde de débit, seuil. C'est une lecture, elle ne dépend de personne, et
   elle se teste avec un embarqueur factice. Elle sert dès maintenant, même
   devant l'ingestion d'aujourd'hui.
2. **L'état d'avancement public** par table et par niveau. Petit, et il rend
   visible ce qui existe déjà (la dette, son remboursement).
3. **`index`**, quand `sync_source` est livrée avec son option d'exigence :
   le reçu, la confirmation dans le harnais, le remboursement en fond.
4. **La source git et la politique « cloud »**, en dernier : c'est ce qui
   demande le plus de décisions (où cloner, quoi interdire) et le moins de
   moteur.

## 6. Ce qui attend une décision de Lucie

- **Le seuil de confirmation** : cinq minutes de vecteurs prévues, ou autre.
- **L'ordre des vecteurs** : « ce qu'on vient de chercher, puis le plus
  récent » suffit-il pour commencer ?
- **La politique « cloud »** : faut-il autoriser autre chose que https public
  (jeton d'accès à un dépôt privé) dès la première version ?
