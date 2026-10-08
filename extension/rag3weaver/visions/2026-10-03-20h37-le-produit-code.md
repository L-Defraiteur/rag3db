# Visions pour le produit code

3 octobre 2026. Sept idées proposées à Lucie, qui les a toutes retenues, avec
deux précisions de sa part (notées « Lucie : »). Rien n'est codé. Ce document
complète `extension/rag3weaver/visions/2026-10-03-20h16-explorer-les-relations.md`.

Le point de départ : un agent muni de grep sait déjà lire du code. Ce que nous
avons en plus est un graphe persistant, synchronisé, mêlé à la recherche par
sens. Chaque idée ci-dessous s'appuie sur cela.

## 0. Une commande en fond, que l'agent consulte pendant qu'elle écrit (Lucie, 8 octobre)

Lucie : « l'exécuteur de commandes devrait avoir une option pour que l'agent
choisisse fond ou pas ; en fond, il peut à tout moment consulter la sortie
pendant qu'elle s'écrit dans un fichier temporaire systématique — un peu
comme dans Claude Code. » Noté, à faire après ; rien n'est lancé.

Ce qui existe déjà (`src/dataflow/run_nodes.rs`, `src/commande.rs`) : chaque
flux d'une commande est **déjà dérivé en entier vers un journal** sous
`rag3weaver-commandes/` (l'agent ne reçoit qu'un aperçu borné, et le chemin
du journal) ; `wait` attend qu'un motif apparaisse dans ce journal, et sait
dire « pas encore » ; la boucle d'agent sait rendre un accusé « en cours » et
livrer le vrai résultat plus tard dans sa boîte (`is_async`). La description
de `run` renvoie déjà à « la variante de fond », qui n'existe pas.

Ce qui manque : un paramètre `background` sur `run`, qui rend tout de suite la
poignée et le chemin du journal au lieu d'attendre ; un verbe « montre-moi la
fin du journal » (les N dernières lignes, ou depuis mon dernier regard), à
côté de `wait` qui cherche un motif ; la fin de la commande livrée comme
message dans la boîte, avec son code de sortie ; et le plafond de 1 800 s
remplacé, en fond, par une mort propre du processus à la fin du run. Le
dossier des journaux ne s'ouvre qu'à ses propres fichiers, comme aujourd'hui.

## 1. L'impact avant l'édition, les tests après

Avant d'éditer une fonction, l'agent voit « 23 appelants, 4 tests la
traversent ». Après, le graphe dit quels tests rejouer.

- Dessous : les usages (vision relations, capacité A) et le voisinage entrant
  (capacité D) ; une marque « est un test » sur les scopes.
- Mesure : sur un changement connu, les tests choisis contiennent ceux qui
  cassent.

## 2. « Le même motif existe ailleurs »

Après une édition, on cherche par similarité le code qui ressemble à l'ancien
texte : « tu as corrigé ici, trois autres endroits ont la même forme ».

**Lucie : un crochet après l'appel d'outil, qui enrichit le retour de
l'outil.** L'agent ne demande rien : le rendu de `edit` porte une section
« à voir aussi » quand la similarité dépasse un seuil.

Cela pose une pièce générique : le **crochet après outil**, déclaré au
manifeste, qui ajoute une section au rendu d'un outil. Il sert aussi à
l'idée 3 (les notes accrochées arrivent avec la lecture) et à la mémoire
longue (document 02). Garde-fous : un budget de lignes, un seuil, et le droit
de se taire — une section vide ne s'affiche pas.

## 3. Les décisions accrochées au code

Une note, une décision, un chantier : une entité reliée aux scopes qu'elle
concerne. L'agent qui lit une fonction lit le « pourquoi » avec.

**Lucie : versionnées, ou au moins datées.** Donc :
- chaque note porte sa date, son auteur, et la révision du code qu'elle a vue ;
- une note ne se réécrit pas : une nouvelle la remplace et garde le lien vers
  l'ancienne ;
- quand le code sous une note change, la synchronisation la marque « à
  revoir » au lieu de la laisser mentir.

C'est le même objet que la mémoire longue, vu depuis le code : voir le
document 02.

## 4. Plusieurs dépôts reliés

Le projet, ses dépendances, ses forks, dans le même graphe, avec les symboles
résolus d'un dépôt à l'autre. « Où notre fork diverge de l'amont », « qui chez
nous appelle cette fonction de la bibliothèque ».

- Dessous : la résolution des symboles entre sources (aujourd'hui dans une
  source), une relation « dépend de » entre sources.
- C'est le produit CLI : il a le disque, il va chercher les autres dépôts.

## 5. L'histoire comme graphe

Les commits comme entités, reliés aux scopes qu'ils touchent. « Qui a changé
ça, et pourquoi » ; les fichiers qui changent toujours ensemble — une relation
qu'aucune analyse du code ne donne.

- Dessous : une source « historique git », un grain commit, une relation
  pondérée « change avec » (le poids est une propriété d'arête).

## 6. Plusieurs agents sur une base

Chaque agent a son arbre de travail comme périmètre ; tous lisent le même
index. C'est là que les écritures parallèles deviennent visibles pour un
utilisateur.

- Dessous : les marches du moteur (verrous, plusieurs écrivains), un périmètre
  par arbre de travail.

## 7. Cloud : une URL, des questions dans la minute

Coller l'adresse d'un dépôt ; le texte est cherchable d'abord, les vecteurs
suivent ; la conversation se partage.

- Dessous : `estimate`, `index`, l'instantané en mémoire — presque tout est
  livré ; il manque la minute (l'insertion des relations ralentit quand la
  base grossit).

## Les propriétés d'une relation : à quoi, et quand

Une propriété sert quand elle qualifie le lien et qu'on veut filtrer, peser ou
nettoyer dessus.

| Famille | Exemples | Lue quand |
|---|---|---|
| Nature | genre d'usage, ligne | au filtre, au rendu |
| Force | nombre d'appels, « change avec », quantité | au poids : chemins pondérés, proximité |
| Confiance et provenance | lue dans l'arbre syntaxique ou devinée ; posée par l'analyse ou par la résolution ; ambiguë | au poids, au nettoyage |
| Temps | quelle synchronisation, quelle révision | à la synchronisation |

La provenance règle deux points ouverts : un nom unique devenu ambigu se
**marque** au lieu de garder une arête fausse ou de la perdre ; et les arêtes
se synchronisent comme les nœuds dès qu'elles disent qui les a posées et
quand.

Quand ne pas en mettre : si la chose doit être cherchée ou reliée elle-même,
c'est un nœud. Et deux coûts d'aujourd'hui : la synchronisation des liens
exige des relations sans propriété, et l'insertion des relations est le point
lent du moteur.

## Ordre proposé

1 et 2 d'abord : elles servent chaque édition, se mesurent, et reposent sur ce
qui vient d'être livré. Puis 3 avec la mémoire longue. 7 dès que la minute est
tenue. 4, 5, 6 ensuite.
