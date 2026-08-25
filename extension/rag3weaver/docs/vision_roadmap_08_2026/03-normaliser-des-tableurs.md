# 03 — Normaliser des tableurs : ce qu'un pipeline réel a appris

Un pipeline d'ingestion de tableurs a été construit ailleurs, en production, en
Python, sur un autre domaine. On l'a relu **pour ses leçons uniquement** : ce
document ne contient ni code, ni nom, ni donnée de ce projet. Les exemples sont
inventés.

Il compte parce qu'il **contredit notre spécification de février** ([02](02-les-deux-moities.md) §3)
sur quatre points, et qu'il a payé chaque contradiction en correctifs.

## 1. La forme qui a survécu

Quatre phases **strictement séquentielles**, chacune **persistant son artefact**
dans un stockage objet. Une reprise après incident relit le dernier artefact et
saute ce qui est fait.

> C'est la décision d'architecture la plus rentable du projet, et celle qu'il
> faut prendre **le premier jour** : elle rend le débogage possible sur un
> fichier de treize mille lignes qui échoue au bout de vingt minutes.

| phase | ce qu'elle fait | modèle ? |
|---|---|---|
| **1a** structurel | défusion des cellules (valeur du coin haut-gauche recopiée), boîte englobante, découpe des marges vides **en mémorisant le décalage** pour garder les numéros de ligne réels ; lignes vides internes **conservées et marquées** — ce sont des séparateurs de blocs | non |
| **1b** structure | la grille numérotée part telle quelle ; il revient des *sections* (titre, ligne d'en-tête, colonnes avec type et confiance, lignes de données) et des avertissements localisés | **oui** |
| **1c** + croisé | réassemblage, puis une passe qui voit **toutes les colonnes de tout le fichier** et cherche les incohérences entre feuilles | oui |
| **2** colonnes | regroupement par nom à travers le fichier, puis trois voies selon cardinalité et type (§3) | oui, mais borné |
| **3** application | tables de correspondance, conversions d'unités, plages éclatées en borne basse / haute / unité, **valeur brute conservée à côté** | **non** |
| **4** matérialisation | deux représentations depuis la même ligne (§4), entités de catégorie, schéma de collection dérivé | non |

## 2. Le renversement d'échelle — la meilleure idée technique

> **Ne jamais montrer au modèle ce qu'on peut lui résumer.**

Pour une colonne numérique à forte cardinalité, on ne montre **pas les valeurs**.
On parse d'abord tout en code (nombre + unité) — et ce parse sert de
**validateur** : sous 70 % de réussite, la colonne n'est pas numérique et
repasse en texte. Puis on n'envoie que **l'ensemble des unités distinctes**, et
seulement s'il y en a au moins deux après canonicalisation. Le modèle rend la
table de canonicalisation, l'unité cible et les formules.

Le coût devient proportionnel à la **diversité structurelle**, pas au volume.
C'est arrivé après trois contournements successifs sur l'explosion du coût.

Généralisation : statistiques par colonne plutôt que colonnes entières, unités
distinctes plutôt que valeurs distinctes, noms d'en-tête plutôt que contenu.

## 3. L'inférence de type : le rapport de forces est l'inverse de ce qu'on croyait

Sept types (énumération, numérique, plage, texte, lien, identifiant, booléen).
**Aucun type date** — trou jamais comblé.

Le signal décisif n'est **pas sémantique, il est statistique**, et il a été
ajouté après coup : pour chaque colonne, sur **toute la grille**, le nombre de
valeurs distinctes, le total, le taux d'unicité et trois exemples — injectés
dans le prompt. Les règles sont écrites en fonction de lui : faible unicité →
énumération ; forte unicité → **jamais** énumération.

Puis des **garde-fous déterministes qui écrasent la décision du modèle** :

- au-delà de **500 valeurs distinctes**, ou d'un taux distinct/total de **0,5**
  (avec un plancher de 50 distincts pour ne pas punir les petites tables de
  référence), la colonne cesse d'être une catégorie ;
- une colonne dont les valeurs commencent par un chiffre suivi de mots
  (« 300 pages illustrées ») parsait comme numérique et **perdait ses
  vecteurs** — le test qui l'attrape est que les *unités* extraites soient peu
  nombreuses : une vraie colonne numérique en a deux ou trois, pas quarante.

**D'où la forme qui a survécu, et qui renverse notre spécification :**

> statistiques déterministes → **arbitrage du modèle** → **écrasement
> déterministe** sur bornes dures.

Notre plan de février disait « inférence par heuristiques ». En pratique
l'inférence utile est faite **par le modèle** ; ce sont les *garde-fous* qui
sont heuristiques, et **ils servent à le contredire**. Et chaque écrasement doit
être **journalisé** — jamais de plafonnement silencieux.

## 4. Deux représentations depuis une seule ligne

- le **texte à vectoriser** vient des valeurs **originales, en langage
  naturel**, préfixé du chemin catégorie › sous-catégorie et du titre ;
- le **payload filtrable** vient des valeurs **normalisées**.

Vectoriser du texte normalisé en identifiants techniques dégrade mesurablement
le rappel sémantique. Et la valeur brute est conservée **à côté** de la valeur
convertie, systématiquement.

## 5. Le sale du réel

**Traité** : cellules fusionnées, marges vides, lignes vides internes comme
séparateurs, **plusieurs tableaux par feuille** (cas nominal, N sections avec
des colonnes différentes), en-tête répété plus bas = nouvelle section, lignes de
titre (une ou deux cellules remplies), noms de colonnes en double (suffixés puis
retrouvés par comparaison de position), colonne sans nom (avertissement + nom
proposé), **section sans titre → nom inventé** (interdiction explicite de
laisser un titre nul : mieux vaut une catégorie inventée qu'un trou de
navigation), nombres en texte (virgule décimale, espaces insécables), plages en
langue naturelle (« moins de X », « X à Y » → bornes, borne ouverte = nul),
unités mélangées (alias → forme canonique, cible = la plus fréquente,
conversions usuelles en dur, formule libre **testée avant d'être retenue**),
pourcentages contre ratios, devises **détectées jamais converties**, et une
valeur inconnue de la table est **conservée brute plutôt que perdue**.

**Renoncé — la liste la plus utile :**

| non traité | conséquence |
|---|---|
| **CSV, entièrement** | le point d'entrée refuse tout ce qui n'est pas un classeur. Donc ni séparateur, ni encodage, ni BOM, ni guillemets mal échappés. **Hors périmètre par construction — et personne ne s'en est plaint.** |
| **Dates** | pas de type, pas de reconnaissance, pas de désambiguïsation jour/mois. Une date devient du texte : ni filtrable, ni comparable. **Trou béant.** |
| **Lignes de total** | rien. Une ligne « Total » à une cellule tombe dans l'heuristique « titre de section » → elle devient une **catégorie**. Une ligne de total complète devient une **entité fantôme parfaitement indexée**. |
| **Valeurs sentinelles** | « N/A », « — », « néant », un zéro qui veut dire inconnu : deviennent des valeurs d'énumération légitimes, ou font chuter le taux de parse et basculer toute la colonne en texte. |
| Commentaires, notes, barré, couleur, mise en forme conditionnelle | ignorés |
| **Formules** | lues via leur dernière valeur en cache. Fichier jamais recalculé → valeur absente, **indiscernable d'une cellule vide** |
| Lignes masquées, filtres, plages nommées | ignorés |
| **Doublons de lignes** | aucune détection, ni exacte ni approchée |
| Cellules multi-valuées (« rouge ; bleu ») | retombent en texte, jamais éclatées |

**Et un piège structurel** : le regroupement des colonnes se fait **par nom, à
travers tout le fichier**. Deux homonymes dans deux feuilles qui ne parlent pas
de la même chose sont **fusionnés** ; deux synonymes (« Prix », « Tarif »)
restent séparés. L'unification des synonymes est *promise dans un prompt* et
**jamais implémentée**.

## 6. L'identité : l'échec le plus instructif

L'identifiant d'une ligne est **positionnel** — feuille + titre de section +
compteur. Il ne dérive d'**aucune** colonne, pas même de celle typée
« identifiant ». Donc : insérer une ligne au milieu décale toutes les identités
suivantes ; renommer une feuille invalide tout ; et comme **les titres de section
peuvent être inventés par le modèle**, une variation de formulation d'un import
à l'autre invalide une section entière.

Conséquence assumée dans le code : **la collection est intégralement vidée puis
reconstruite** avant chaque ingestion. Le mécanisme de saut par hachage existe,
est actif, et est **structurellement neutralisé** — il ne trouve jamais rien.
On a préféré payer le ré-embedding complet plutôt que résoudre l'identité.

> **La leçon n'est pas que l'identifiant unique soit une mauvaise idée. C'est
> qu'il suppose une désignation humaine de la colonne clé que personne
> n'effectue** — donc le système doit rester correct sans.

## 7. La validation humaine : il n'y en a aucune

**Aucune question, aucun point d'arrêt, aucune attente de réponse.** Ce qui
existe à la place, et où l'effort a vraiment été mis :

- **un flux de progression à granularité fine** — bloc *n* sur *m*, colonne par
  colonne, lot par lot, avec instantané rejouable pour qui se connecte en
  retard. Le volume de travail investi là dit que **le vrai problème
  d'expérience était l'attente silencieuse**, pas l'exactitude ;
- **des avertissements localisés et illustrés** : coordonnées exactes, un
  **mini-tableau de deux ou trois lignes réelles** avec la ligne fautive
  surlignée, et pour les incohérences entre feuilles un **témoin** — « voilà
  l'onglet où c'est correct, comparez ». C'est la meilleure idée d'ergonomie du
  projet : on ne dit pas « colonne suspecte », on montre les deux tableaux ;
- **déduplication et plafond** : un problème = un avertissement, avec un
  coupable et un témoin. Noyer sous deux cents signalements équivaut à ne rien
  signaler ;
- **une frontière explicite entre ce qu'on corrige seul et ce qu'on remonte**,
  écrite dans les instructions : les variantes de nommage, la casse, les unités
  différentes seront corrigées en aval → **ordre de ne pas les signaler** ; les
  colonnes aux valeurs échangées et les en-têtes qui mentent ne le seront
  jamais → **signaler en priorité**.

> **On ne demande à l'humain que ce qui est irrattrapable en aval et
> silencieusement destructeur.** Une inversion de deux colonnes ne fait pas
> échouer l'import — elle corrompt les filtres sans bruit. C'est exactement le
> seul cas qui mérite un humain.

Le cycle réel est « je corrige mon tableur et je recommence », pas « je réponds
à un questionnaire » — le projet expose d'ailleurs le **retéléchargement du
fichier source**.

## 8. Ce qu'il ne referait pas

1. **Confier la détection de structure entièrement au modèle**, puis lui
   réinjecter des statistiques, puis ajouter des garde-fous qui l'écrasent. À
   l'arrivée la vérité est dans le code et le modèle ne fait plus que nommer et
   découper. **L'ordre aurait dû être inverse dès le départ.**
2. **Demander au modèle d'appliquer une correction et de déclarer l'avoir
   appliquée** — il déclarait des corrections fantômes. *Le modèle propose, le
   code applique*, avec un test d'idempotence.
3. **Envoyer des valeurs quand on peut n'envoyer que des unités ou des
   statistiques.**
4. **Produire un score de confiance que personne ne consomme.** Il coûte des
   jetons et donne l'illusion d'un pilotage par l'incertitude.
5. **Empiler trois mécanismes pour une seule panne** (la troncature). Et
   détecter une troncature en cherchant une sous-chaîne dans un message
   d'erreur est un pari qu'on perd au prochain changement de fournisseur.
6. **Identifiant positionnel + effacement total.**
7. **Apparier deux représentations d'une même ligne par leur rang** dans deux
   listes parallèles : tout filtrage d'un seul côté décale silencieusement tout
   le fichier. Il faut un identifiant de ligne **porté par la donnée**.
8. **L'agent explorateur qui « architecture » le schéma.** Une première version
   reposait sur des agents qui découvraient un schéma d'extraction. Elle a perdu
   contre **quatre étapes fixes, chacune remplaçable et persistée**.

## 9. Ce que ça contredit dans notre spécification de février

| notre plan | ce que la pratique dit |
|---|---|
| « inférence de type par heuristiques » | l'inférence utile est **du modèle** ; les heuristiques servent à **le contredire** (§3) |
| « scores de confiance » | produits, **jamais consommés**. Soit on les branche sur une décision réelle, soit on ne les demande pas |
| « questions de validation » | **aucune question**. Rien ne bloque. Ce qui marche : avertissements illustrés non bloquants + « corrige et recommence » |
| « identifiant unique pour l'incrémental » | **non atteint, délibérément abandonné**. Il suppose une désignation humaine que personne ne fait (§6) |

Silences confirmés de part et d'autre : pas de date, pas de lignes de total, pas
de sentinelles, pas de doublons. Les quatre sont de vraies dettes. Le CSV, lui,
est un renoncement lucide.

## 10. Le portage en Rust

**Le vrai coût n'est pas le parsing du tableur.** C'est le dialogue avec le
modèle : sortie structurée validée par schéma, comportement en cas de
troncature, découpage adaptatif, réinjection du contexte entre morceaux. En
Python une bibliothèque le donne gratuitement ; chez nous il faut le bâtir — et
c'est exactement ce que le [doc 50](../23-aout-2026-20h33/50-tool-calling-local-formats-schemas-et-ce-quon-ecrit.md)
prépare (`schemars` + `llguidance` + notre `response_format`).

Bonne nouvelle : **aucune bibliothèque de tableaux de données n'est utilisée**
dans le pipeline lu. L'extraction est cellule par cellule, et l'inférence de
type « gratuite » par colonne a été **refusée** — elle n'a pas de sens quand une
feuille contient cinq tableaux.

À prévoir : implémenter soi-même la **propagation des cellules fusionnées** ;
distinguer explicitement *cellule absente* / *chaîne vide* / *formule sans
valeur en cache* (le typage dynamique le masquait) ; concevoir les types somme
**avant** d'écrire le pipeline, mais garder une valeur dynamique **aux
frontières entre phases**, sinon chaque évolution d'un prompt casse la
compilation de tout l'aval. Pour les formules de conversion : un
mini-évaluateur arithmétique écrit à la main est **plus sûr** qu'une liste
blanche sur un interpréteur généraliste — ou renoncer aux formules libres, ce
qui est probablement le bon choix puisqu'elles ne servent qu'aux paires
exotiques et sont de toute façon testées avant d'être retenues.

## 11. Sept questions ouvertes que personne n'a tranchées

1. **Où vit l'identité d'une ligne sans clé ?** Trois pistes non explorées :
   empreinte sur un sous-ensemble de colonnes **choisi automatiquement** (forte
   unicité, peu de nuls) ; identité positionnelle **plus** un rapprochement
   approché entre imports pour transférer les identités ; ou l'aveu explicite
   « ce catalogue n'est pas synchronisable », **dit à l'utilisateur** au lieu
   d'être caché.
2. **Les lignes de total** sont indétectables par le contenu mais évidentes par
   la forme — mot-clé, position en fin de bloc, **valeur qui est la somme de la
   colonne au-dessus**. Un test arithmétique déterministe est faisable et
   personne ne l'a écrit.
3. **Un type date, et l'ambiguïté jour/mois** — à décider sur la colonne
   entière, pas cellule par cellule, et n'avertir que si les deux lectures
   restent possibles sur *toute* la colonne.
4. **Quelle est la portée d'un nom de colonne ?** Feuille, section, fichier ? Le
   projet a choisi « fichier » **sans le dire** et en paie le prix.
5. **Le modèle rend-il des corrections ou des constats ?** L'expérience tranche
   pour « constats + proposition » — ce qui implique un **journal de corrections
   appliquées par le moteur, réversible et inspectable**, inexistant là-bas.
6. **Comment reprendre un import après correction du fichier source sans tout
   refaire ?** Avec des artefacts persistés et une identité stable, on ne
   rejouerait que les feuilles modifiées. Jamais essayé.
7. **Combien coûte réellement le ré-embedding intégral**, et à partir de quelle
   taille ce choix devient-il intenable ? Le projet ne le mesure pas ; il a
   relevé ses limites de temps jusqu'à ce que ça passe.
