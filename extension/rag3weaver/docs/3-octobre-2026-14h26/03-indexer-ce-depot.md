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
« écarté, parce que ». Sur ce dépôt-ci, c'est ce qui sépare 59,8 Mo de code et de texte retenus
(6 831 fichiers) de 524 Mo écartés, surtout des CSV et du Parquet.

**Le dossier n'est pas le dépôt.** Lu tel quel, ce dossier de travail compte
307 541 fichiers (builds, données d'expérience, dossiers de compilation) ;
l'estimation l'a montré avant qu'on indexe. Depuis, `WorkingTree` respecte
les règles d'exclusion du dossier (7 442 fichiers) et écarte les secrets
probables, avec la raison.

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

**Mesuré le 3 octobre au soir**, deux fois : une fois l'ingestion de code
capable de s'arrêter au plein texte (`sync_source` avec
`exige = RECHERCHE_TEXTE`), puis après le chargement en masse des relations
— les deux livrés par la session de l'arbre principal. Le premier jet de
cette page donnait un calcul, « environ une minute » : la première mesure l'a
trouvé **faux d'un facteur trente** sur le dépôt entier, la seconde le ramène
à un facteur neuf. Les trois chiffres restent ici, datés.

| Corpus (binaire de test non optimisé, granite-278m par le service distant) | Cherchable par mots | Relations | Vecteurs |
|---|---|---|---|
| `src/` de la crate : 122 fichiers, 4 Mo, 9 528 morceaux | **17 s** | comprises | ~1 min prévue |
| Ce dépôt, première mesure : 6 735 fichiers, 59,6 Mo, 418 761 relations posées paquet par paquet | 1 798 s (30 min) | comprises | 710 s mesurés, 569 s prévus |
| **Ce dépôt, après le chargement en masse des relations** : 6 799 fichiers, 60,6 Mo, 123 750 morceaux, 384 790 relations | **514 s** (8 min 30) | **10 s** | **692 s** mesurés, 651 s prévus |

Quatre choses à en retenir.

- **L'exigence est tenue** : à la fin de la passe « plein texte », aucun
  vecteur n'est calculé et une recherche par mots répond. La toute première
  mesure, avant correctif, en trouvait 83 % de faits — un rattrapage tournait
  dans la liaison des symboles.
- **Le temps des mots ne croissait pas comme la taille, et c'est corrigé.**
  À la première mesure, les 1 024 premiers fichiers passaient en 19 s puis
  chaque millier coûtait 53, 151, 283, 422 s : l'insertion des relations
  paquet par paquet. La session de l'arbre principal les met maintenant en
  file pendant les paquets et les charge en masse à la fin ; par millier de
  fichiers, cumulé : 19, 66, 158, 244, 323, 400, 514 s. Ramené aux scopes
  c'est à peu près plat — 5 ms par scope au premier millier, 7 en moyenne.
  Le temps total pour chercher par mots passe de 1 798 s à 523 s.
- **Les relations arrivent d'un coup, en dix secondes**, après les mots.
  Entre-temps la file est vidée chaque fois qu'elle dépasse 200 000 liens
  (deux fois sur ce dépôt), pour borner la mémoire : ces vidages sont comptés
  dans le temps des mots.
- **L'estimation des vecteurs tient à 6 % près** : 651 s prévus, 692 s
  mesurés (20 % à la première mesure, où la sonde ne voyait pas la
  reconstruction de l'index). Chaque indexation note son débit réel ; la
  suivante prévoit mieux.

**Où en est la promesse.** Sur ce dépôt, on cherche par mots au bout de huit
minutes et demie, le graphe est là dix secondes plus tard, et les vecteurs
onze à douze minutes après : le plein texte d'abord fait gagner plus de la
moitié de l'attente. Ce n'est pas encore les deux minutes qu'un moteur
optimisé rendrait — ces mesures sont prises sur un binaire de test non
optimisé — et l'avancement dit donc des fichiers faits, pas un temps restant,
pendant ce premier temps.

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

> **État au 3 octobre au soir** : les points 1 et 2 sont sur master
> (`src/estimate.rs`, `EstimateNode`, `templates/tools/estimate.mmd`,
> `Catalog::index_progress`). Le point 3 attend la correction de l'insertion
> des relations.

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
