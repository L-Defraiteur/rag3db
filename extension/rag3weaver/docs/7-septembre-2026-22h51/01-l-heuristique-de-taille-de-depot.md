# L'heuristique de taille de dépôt

**7 septembre 2026, 22 h 51.** Conception, avant le code. Suite de
[l'index à plusieurs embarquements](../7-septembre-2026-21h34/01-un-index-a-plusieurs-embarquements.md),
fusionné (`3ea70fe93`).

## 1. La demande, et ce qui la rend possible

> quand on aura plusieurs embarqueurs possibles, on commencera par 107m selon
> une heuristique de taille de dépôt — 278m restant le défaut.

Ce qui existe déjà :

- **278m est le défaut du démon** (`MODELE_PAR_DEFAUT`, `bb538cb0d`) ;
  `RAG3WEAVER_EMBED_MODEL` l'écrase.
- **Un démon sert un modèle**, et se lance par `Serveur::assurer`
  (`serveur.rs:247`) avec l'environnement que l'appelant lui donne
  (`serveur.rs:329`). Le modèle d'un démon est donc une **variable posée au
  lancement** — c'est là qu'une heuristique peut parler.
- **Un index accueille plusieurs modèles** : commencer en 107m n'engage à
  rien. Passer en 278m plus tard est un retard de 100 % que le rattrapage
  solde, sans redécouper.

Le troisième point est ce qui change la nature du choix : ce n'est plus « quel
modèle pour cet index », c'est « **par quel modèle on commence** ». Un mauvais
choix coûte une passe de rattrapage, pas une réindexation.

## 2. Les chiffres qu'on a

**Vitesse** — le cœur C++ de rag3db, 1 642 fichiers, 20 136 chunks
(`docs/optimiseur/7-septembre-2026-21h56/01`, `docs/optimiseur/6-septembre-2026-16h30/04`) :

| modèle | temps de modèle | par chunk |
|---|---:|---:|
| granite-107m | 10,7 s | 0,53 ms |
| granite-278m | ~20 s | ~1 ms |

**Qualité** — le banc de l'optimiseur, 45 questions sur notre code :

| | MRR | R@1 | R@5 | sans commentaires : MRR · R@5 |
|---|---:|---:|---:|---|
| granite-107m | 0,779 | 0,62 | **1,00** | 0,738 · 0,91 |
| granite-278m | **0,844** | **0,71** | **1,00** | 0,812 · 0,93 |

Ce que ça dit : 107m perd **6 à 7 points de MRR et 9 de rappel à 1**, et rien
en rappel à 5 quand le code est commenté. Deux fois plus rapide, un index deux
fois plus petit. Il n'y a pas de dépôt où 107m est *meilleur* ; il y a des
dépôts où 278m est *trop long à attendre au premier index*.

Le critère est donc **un temps d'attente**, et la question devient : à partir
de combien de chunks 20 s deviennent une gêne ?

## 3. Le critère

### 3.1 Compter quoi

| mesure | disponible avant `Catalog::new` ? | ce qu'elle rate |
|---|---|---|
| nombre de fichiers | oui | un en-tête généré de 78 000 lignes compte pour un — mesuré sur ce dépôt, `utf8proc_data.h` |
| **octets de source lisibles** | **oui** — `read_sources_report` les lit de toute façon | rien de grave : c'est ce que le modèle va lire |
| chunks estimés | non — le découpage vient après | — |

**Les octets de source**, retenus par les données. `read_sources_report` a déjà
tout lu : le compte est gratuit. Et le rapport octets → chunks est stable sur
ce qu'on a mesuré : le cœur C++ fait 20 136 chunks pour ~10 Mo de source,
soit **~500 octets par chunk**.

Le nombre de fichiers ne sert que de garde-fou contre un cas absurde (un seul
fichier de 100 Mo n'est pas un gros dépôt, c'est un dump — et la politique
d'ingestion l'écarte déjà au-delà de 128 Kio pour le texte).

### 3.2 Le seuil

Avec 1 ms par chunk en 278m et 500 octets par chunk :

| source | chunks | 278m | 107m |
|---|---:|---:|---:|
| 10 Mo (le cœur) | 20 000 | 20 s | 11 s |
| **25 Mo** | **50 000** | **50 s** | 27 s |
| 50 Mo | 100 000 | 100 s | 53 s |
| le noyau Linux (~90 000 fichiers) | ~1 M | ~17 min | ~9 min |

**Proposition : 25 Mo de source.** En dessous, 278m coûte moins d'une minute
de modèle au premier index — on prend le meilleur. Au-dessus, on commence en
107m, et 278m viendra en retard si on le demande.

C'est un ordre de grandeur, pas une mesure : il pose « une minute » comme
limite de l'attente acceptable, et c'est ça qui est à confirmer avec Lucie —
pas le chiffre, la minute.

### 3.3 La précédence, la même que partout

**Le code l'emporte sur la variable, qui l'emporte sur l'heuristique, qui
l'emporte sur le défaut.** `RAG3WEAVER_EMBED_MODEL` posée gagne toujours ;
l'heuristique ne parle que dans le silence. Pas d'exception cette fois : la
variable dit ce qu'on veut *pour ce dépôt*, et un régime `confort` ne change
pas de modèle — la carte est la même, c'est le temps qui diffère.

## 4. Où ça vit

### 4.1 Le calcul

Un module pur, `embedding_choice.rs` :

```rust
pub fn recommended_model(source_bytes: u64, files: usize) -> Choice
// Choice { model: "granite-107m" | "granite-278m", reason: String }
```

Testable sans base. La raison est une phrase — *« 31 Mo de source, au-delà de
25 Mo : granite-107m pour le premier index »* — parce qu'un choix qui ne se
dit pas se conteste.

### 4.2 Le moment

**Là où l'appelant construit le `Serveur` du démon**, avant `Catalog::new` :
il a lu les sources, il connaît les octets, il pose
`RAG3WEAVER_EMBED_MODEL` dans l'environnement du `Serveur` si elle n'est pas
déjà posée. `Serveur::assurer` lance le démon avec, `DaemonEmbedder::assurer`
s'y attache, le catalogue reçoit un embarqueur qui se nomme.

**Un démon déjà là sert ce qu'il sert.** Si son `Identite.modele` n'est pas
celui que l'heuristique voulait, on **prend ce qui tourne et on le dit** — on
ne relance pas un démon qu'une autre session utilise peut-être. Relancer est
un geste explicite (`RAG3WEAVER_EMBED_MODEL` posée, ou `quitter`).

### 4.3 Où le choix se lit

- **Au montage**, une ligne : `[rag3weaver] granite-107m pour le premier
  index — 31 Mo de source, au-delà de 25 Mo ; 278m viendra en retard si on
  l'exige`. Et, si le démon servait autre chose : `… mais le démon en place
  sert granite-278m — on le garde`.
- **Dans la méta**, une clé `embedding_choice` : `{model, reason, at}`,
  posée au premier enregistrement. Ce que `embedding_model:{slug}` ne dit
  pas, c'est *pourquoi* ce modèle est là le premier ; c'est ce qu'on lira
  dans six mois en se demandant pourquoi cet index est en 107m.

## 5. Quand le dépôt grossit après coup

**Rien.** Le seuil ne se réévalue pas : il décide du *premier* index, et le
premier index a eu lieu. Un dépôt qui passe de 20 à 60 Mo garde son 278m et
embarque plus longtemps ; un dépôt né gros et resté en 107m y reste tant que
personne ne demande autre chose.

Demander autre chose, c'est : `RAG3WEAVER_EMBED_MODEL=granite-278m`, ouvrir,
exiger « dense ». Le modèle s'enregistre, le retard vaut 100 %, le rattrapage
le solde — avec l'index tombé au-delà du seuil de reconstruction, puisque
c'est un gros rattrapage par construction. Le 107m reste disponible dans sa
colonne. C'est exactement le chemin du chantier précédent, sans une ligne de
plus.

Ce qu'il ne faut **pas** faire : réévaluer à chaque ouverture et changer de
modèle courant en silence. Deux colonnes à moitié pleines, et personne ne
sait laquelle a été cherchée.

## 6. Ce qui dirait que ça marche

1. `recommended_model(10 Mo, 1 642)` rend 278m avec sa raison ;
   `recommended_model(31 Mo, 5 000)` rend 107m avec la sienne.
2. La variable posée gagne : `RAG3WEAVER_EMBED_MODEL=granite-278m` sur 31 Mo
   donne 278m, et la ligne de montage le dit.
3. Un démon en place qui sert un autre modèle est **gardé** et nommé.
4. Après un premier index en 107m, `RAG3WEAVER_EMBED_MODEL=granite-278m` +
   « dense » exigé : les deux colonnes existent, la recherche prend 278m,
   `embedding_choice` dit toujours 107m et pourquoi.

## 7. À trancher

- **La minute.** Le seuil de 25 Mo découle de « moins d'une minute de modèle
  au premier index en 278m ». Si l'attente acceptable est de trois minutes, le
  seuil est 75 Mo et le cœur n'y arrive jamais. C'est le seul vrai choix.
- **Le démon en place.** Garder ce qui tourne (proposé) ou relancer avec le
  modèle voulu. Garder est sûr et se dit ; relancer est rapide et casse
  l'autre session.
