# 07 — Le langage de déclaration : la frontière, le mur, et le coût

Ambition n°12 de Lucie — *« qu'un backend soit facile à poser quelque part et
à écrire par les agents m'importe beaucoup »*. Le
[doc 06 §1.4](06-le-tamagotchi-et-le-compilateur.md) dit l'idée ; celui-ci dit
comment on y va sans la rendre nuisible.

L'idée tient en une phrase : au lieu de demander à un modèle huit mille lignes
de backend, on lui demande une **représentation intermédiaire contrainte**, et
le compilateur descend vers le stockage, la recherche, les graphes, les
événements et la surface.

> **Le LLM choisit dans un langage que le moteur sait vérifier.**

Trois questions décident si ça marche : où passe la frontière, où est le mur,
et — la plus importante en pratique — **ce que le langage encourage**.

## 1. La frontière n'est pas où on croit

Elle ne sépare pas les applications simples des compliquées :

> Elle sépare les invariants **exprimables dans la déclaration** de ceux qui
> ne le sont pas.

Tout invariant exprimable est un invariant que le compilateur fait respecter et
que le modèle **ne peut pas** enfreindre. Tout invariant resté dans la tête de
quelqu'un est un invariant qu'il enfreindra, quel que soit son talent.

D'où la seule bonne façon d'avancer :

> **On ne déplace pas la frontière en rendant l'agent plus malin. On la déplace
> en agrandissant le langage de déclaration.**

C'est la forme exacte du corollaire du
[doc 49 §7.3](../23-aout-2026-20h33/49-vision-le-catalogue-comme-graphe-outils-tags-memoire.md) :
*ce qu'un agent n'arrive pas à faire sans Cypher devient la feuille de route.*
Chaque tranche est petite, mesurable, et utile toute seule.

## 2. Le mur n'est pas la v1, c'est la v2

Générer une v1 n'a jamais été le problème. Ce qui tue les backends générés,
c'est **le changement de schéma avec des données dedans**.

Là, l'avantage est réel : un schéma étant une **donnée versionnée**, la
différence entre deux versions est elle aussi une donnée — une migration peut
être **dérivée** plutôt qu'écrite. La graine existe : `register_relation_with`
fait un `ALTER` additif au lieu d'exiger une reconstruction, et
`EntityConfig::validate` refuse déjà une configuration dont les morceaux se
contredisent.

## 3. Ce que le langage encourage — la partie qui décide de tout

> « Faut éviter d'encourager que tout soit systématiquement *embedded*, et
> s'assurer que les index et les relations soient bien foutus, encourager les
> bons comportements en connaissance du natif. » — Lucie, 27 août

C'est **la** contrainte de conception, et pas un raffinement. Un modèle à qui
on demande de décrire un domaine cochera tout ce qui est cochable : déclarer
« cherchable » est gratuit à écrire et cher à exécuter. Si le langage rend le
mauvais comportement facile, il produira des schémas coûteux **à grande
échelle et sans bruit**.

### 3.1 Le défaut actuel embarque tout — et il nous a déjà mordus

Aujourd'hui, `EntityConfig::default()` vaut `SearchSignals::HYBRID`, c'est-à-dire
**BM25 + vecteur**. Toute entité déclarée sans y penser reçoit donc des
embeddings.

Et ce n'est pas une inquiétude théorique. C'est écrit dans notre propre code,
en commentaire, à `src/code.rs:180` :

> *« le défaut `HYBRID` faisait calculer et stocker un embedding pour chacun
> des 3 275 symboles de `src/dataflow`, ce que personne n'avait voulu. »*

**Le défaut a déjà piégé les gens qui connaissent le système.** Un modèle ne
fera pas mieux.

La correction n'est pas de changer `Default` — c'est un défaut raisonnable pour
du code écrit à la main, et le changer casserait les appelants. C'est de poser
que **le chemin de déclaration emprunté par un modèle n'hérite pas de ce
défaut** : les signaux y sont **dits**, ou c'est BM25. Un vecteur se demande, et
la demande porte une raison.

La règle sous-jacente est celle de `Symbol`, qui est le bon exemple à copier :

| | ce qui est déclaré | pourquoi |
|---|---|---|
| `Symbol` | `BM25`, `chunked: Some(false)` | un nom de symbole n'a rien à gagner d'un vecteur, et son contenu **est** son titre |

### 3.2 Une relation bien faite bat le volume — mesuré, pas supposé

Le meilleur argument est un chiffre qu'on a déjà :
[doc 04 du 25 août](../25-aout-2026-18h58/04-attribution-des-references-le-graphe-divise-par-sept.md).

| | avant | après |
|---|---|---|
| relations | 66 771 | **9 645** |
| cibles distinctes | 413 | **416** — *rien de perdu* |
| résolution | 194 ms | 54 ms |

Sept fois moins de relations, **et une cible de plus**. Ce qui avait changé
n'était ni un index ni un modèle : c'était **à quel scope on attribuait une
référence**. Une relation bien posée ne remplace pas seulement du bruit, elle
remplace du travail.

Donc le langage doit rendre une relation **plus facile à déclarer qu'un
signal sémantique**, pas l'inverse. Si une question se répond par un parcours,
elle ne doit pas se répondre par une recherche vectorielle.

### 3.3 Le placement des index est un savoir du compilateur

C'est le piège qui a coûté une nuit, et il est parfaitement inapprenable pour
un modèle :

- l'index **plein texte** vit sur la table **parente** ;
- les index **vecteur** et **sparse** vivent sur la table de **chunks**.

Un modèle n'a pas à le savoir, et surtout **ne doit pas avoir à le savoir** :
il déclare une intention — « ce champ se cherche par le sens » — et le
compilateur place l'index au bon endroit. C'est exactement ce que veut dire
« en connaissance du natif » : la connaissance est dans le compilateur, pas
dans l'invite.

C'est aussi le meilleur argument *pour* toute cette approche. Un modèle qui
écrirait le backend à la main referait cette erreur à chaque fois.

### 3.4 Le coût doit être visible **avant** d'être payé

Une déclaration doit pouvoir répondre à *« qu'est-ce que ça va coûter »* sans
qu'on l'exécute : combien d'entités, combien de chunks, combien d'embeddings,
quels index construits, lesquels reconstruits à la prochaine version.

On sait déjà produire ces nombres — l'ingestion est passée de 28 s à 5,9 s
parce qu'on les a regardés. Les rendre disponibles à la déclaration, c'est
transformer un coût invisible en une ligne qu'on lit avant de valider.

Et un coût qu'on voit est un coût qu'on discute. C'est la même règle que
partout cette semaine : **rendre visible ce dont l'absence ne se voit pas.**

### 3.5 Refuser, pas seulement conseiller

`EntityConfig::validate` refuse déjà `chunked: Some(false)` avec un signal
vecteur ou sparse — parce que l'entité serait *silencieusement introuvable*.
C'est le bon réflexe, et il faut l'étendre : un signal vecteur sur un champ qui
est un identifiant, une entité dont le contenu est son titre, un filtre déclaré
sur un champ qui n'existe pas.

Un conseil qu'on peut ignorer sera ignoré — à grande échelle, par une machine,
sans que personne le remarque. **Une erreur de configuration vaut mieux qu'une
entité silencieusement introuvable**, et ça vaut aussi pour un schéma
silencieusement ruineux.

### 3.6 Le pré-filtre change ce qu'est un bon schéma

Depuis cette semaine, le pré-filtre est **exact sur les trois signaux**. Un
champ déclaré filtrable est donc devenu réellement bon marché — ce qui n'était
pas vrai il y a trois jours.

Conséquence à retenir : *un bon schéma déclare des filtres là où il déclarait
des recherches*. La déclaration doit rendre ça évident, parce que l'intuition
de la plupart des gens — et des modèles — date d'avant.

## 4. Par où commencer

`EntityConfig` décrit aujourd'hui **une forme et des signaux** : champs,
identité (`hashsafe`), recherche, chunking, champs rendus. **Aucun
comportement.**

Première tranche : **l'état et ses transitions**. Déclaratif, vérifiable
statiquement — toute transition mène-t-elle quelque part, tout état est-il
atteignable — et incontournable une fois déclaré.

Et le meilleur argument pour ce bout-là plutôt qu'un autre : **le premier
backend que ce compilateur décrira est le nôtre.** Un outil qui passe de
brouillon à promu ([doc 49](../23-aout-2026-20h33/49-vision-le-catalogue-comme-graphe-outils-tags-memoire.md),
[doc 05](05-la-reputation-des-abstractions.md)) est exactement une transition
d'état sous conditions de preuve. Pas besoin d'inventer un cas d'usage pour
l'éprouver : on s'en sert le jour où on l'écrit, et les mesures viennent
gratuitement.

## 5. Les quatre règles, en une page

1. **Le défaut du chemin déclaré par un modèle est BM25.** Un vecteur se
   demande, avec une raison.
2. **Une relation est plus facile à déclarer qu'un signal sémantique.** Le
   graphe divisé par sept dit pourquoi.
3. **Le placement des index appartient au compilateur.** Le modèle déclare une
   intention, jamais un emplacement.
4. **Le coût est visible avant d'être payé, et l'incohérent est refusé** — pas
   déconseillé.
