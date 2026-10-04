# La stèle du moteur — où l'on s'arrête d'y regarder

4 octobre 2026. Décision de Lucie :

> « Cette liste exactement définit un point d'arrêt de version pour le moteur :
> une stèle, pour savoir continuer sans y regarder encore ad vitam æternam. »

La liste : **les verrous, le chargement en masse journalisé, les écritures
parallèles**, et les défauts que les bancs ont trouvés. Quand elle est
tenue, le moteur est déclaré « assez bon » : on pose la version, et on n'y
revient que sur un défaut **constaté** dans le produit — plus par
précaution, plus pour l'améliorer.

Pourquoi : « le moteur nous aveugle sur la complexité ; c'est lui qui nous
prend beaucoup de temps à régler ; quand on l'aura bien fait, ce sera plus
simple de faire les autres sujets. » Sans point d'arrêt écrit, il absorbe
l'effort sans fin.

## 1. Ce qui doit être vrai

| # | Condition | Ce que « fait » veut dire | La preuve |
|---|---|---|---|
| 1 | **Les verrous** | le gestionnaire de verrous (V1 : ressource générique, genres ligne et index), puis A3′, A4′, V2 de la note de conception | les témoins du banc de concurrence (§6 de la note des verrous) verts |
| 2 | **Le chargement en masse journalisé** | un `COPY` n'impose plus son point de reprise ; ses lignes survivent à un arrêt brutal par le journal | arrêt brutal au milieu d'un chargement, processus neuf, comptes justes ; le premier index n'y perd plus de temps |
| 3 | **Les écritures parallèles** | plusieurs écrivains sur une même base, sans perte ni blocage, le mode allumé hors du banc | le banc de concurrence entier, vert, mode allumé ; deux agents qui écrivent en même temps dans un test du produit |
| 4 | **Plus de défaut connu qui corrompt ou qui perd** | chaque ticket de `docs/tickets/` qui touche la mémoire, la durabilité ou la justesse d'un résultat est fermé | le ticket fermé avec son témoin au banc |

Ce que la stèle **n'exige pas** : les défauts de confort (un message, un cas
de syntaxe contourné, une lenteur sans perte), les correctifs d'amont non
reproduits, toute optimisation. Ils restent en tickets et n'empêchent pas de
poser la version.

## 2. Où l'on en est (4 octobre, 17 h)

- **1, les verrous** : conçus (`docs/3-octobre-2026-15h47/01-note-de-conception-les-verrous.md`),
  témoins écrits au banc ; V1 pas commencée.
- **2, le chargement journalisé** : la demi-page est écrite
  (`extension/rag3weaver/docs/3-octobre-2026-23h31/coeur-cpp/03-le-point-de-reprise-de-copy.md`) ;
  trois voies — alléger le point de reprise (en cours), une fenêtre de
  chargement initial, journaliser pour de bon (la voie que la stèle demande).
- **3, les écritures parallèles** : T0, A2, A5, A5 bis livrées ; le mode
  reste éteint hors du banc ; la suite attend les verrous.
- **4, les défauts** : la traque de la corruption de mémoire d'`e2e_code` a donné trois
  correctifs le 4 octobre — l'index vectoriel dimensionné par le nombre de lignes et trois
  refus nommés à la place d'écritures hors bloc (`1ea49837f`) ; puis, en faisant un test de
  l'essai du banc, deux défauts d'origine après un `COPY` refusé : le point de reprise qui
  écrivait hors de son bloc, et la ligne d'origine d'une clé en double qui sortait de l'index
  de clé primaire (`05788a868`). **La cause de la corruption d'`e2e_code` elle-même n'est pas
  établie** : le point de reprise après un ajout annulé en est un candidat sérieux, non
  vérifié ; le ticket reste ouvert jusqu'à ce que les passes du banc sous AddressSanitizer
  le disent. Les tickets du moteur sont triés au §3.1.

## 3. Ce qui reste à faire pour que la stèle soit utilisable

1. **Trier les tickets** : une ligne par ticket du moteur — bloque la stèle
   (mémoire, durabilité, résultat faux) ou non. Fait au §3.1, à relire par le banc.
2. **L'ordre** : les verrous, puis les écritures parallèles, puis le
   chargement journalisé — ou le chargement d'abord, puisqu'il est sur le
   chemin des 90 s du premier index. À trancher.
3. **Poser la version** quand les quatre conditions sont tenues : une
   étiquette git, la date au journal des chantiers, et la règle « on n'y
   revient que sur un défaut constaté ».

### 3.1 Le tri des tickets du moteur (session cœur C++, 4 octobre, à relire par le banc)

Seuls les tickets **ouverts** qui touchent le moteur. Les tickets fermés ne bloquent plus
rien ; ceux de l'analyse de code, de codeparsers et de rag3weaver ne sont pas l'affaire de
la stèle.

| Ticket | Verdict | Pourquoi |
|---|---|---|
| Une corruption de mémoire tue `e2e_code` | **bloque** | mémoire ; cause non établie, trois correctifs posés autour |
| Le point de reprise plante, à jamais, après `ALTER TABLE … DROP` | **bloque** | mémoire (SIGSEGV) et durabilité : la base ne peut plus écrire de point de reprise ; rag3weaver ne supprime pas de colonne, mais le défaut corrompt |
| Dans une transaction, un balayage de plusieurs tables de relations relit celles d'une autre table | **bloque** | résultat faux |
| Un `COPY` rend une erreur alors qu'il est validé | **bloque** | résultat faux sur la durabilité : l'appelant qui recommence écrit deux fois |
| L'ordre de synchronisation au point de reprise | **bloque, à éprouver** | durabilité, si le doute est fondé ; un arrêt au bon instant le dit |
| Durabilité sur faute d'entrée-sortie, coupure ou mort au mauvais instant | **bloque, à borner** | durabilité ; non éprouvable au banc sans crochet dans `src/` — dire lesquels de ses cas la stèle exige |
| La réouverture d'une base échoue par intermittence, sous charge | **bloque, tant que la cause manque** | une réouverture qui échoue sur un journal de taille nulle touche à la durabilité ; quatre hypothèses écartées |
| Mise à jour massive de vecteurs : des lignes restent injoignables dans l'index | **bloque — sans ticket, à ouvrir** | résultat faux (journal des chantiers §6) |
| Une ligne très loin des autres est injoignable dans un index bâti d'un coup | **bloque — sans ticket, à ouvrir** | résultat faux (journal des chantiers §6) ; à vérifier depuis `1ea49837f`, qui borne autrement le parcours de la construction |
| Un doublon de clé au journal empêche de rouvrir (H4) | **bloque — porté par la condition 1** | durabilité ; c'est A3′ |
| Un `CHECKPOINT` retient la validation d'un lecteur jusqu'à son délai | confort | une attente, sans perte ; à revoir avec la condition 3 (« sans blocage ») |
| Le point de reprise échoue quand le tampon du moteur est petit | confort, au sens de la stèle | l'indexation s'arrête sous un nom, rien n'est perdu ni faux ; gênant pour le produit à 4 Gio |
| Un `COPY` refusé laisse la cardinalité gonflée | confort | ne sert plus qu'au planificateur depuis `1ea49837f` |
| Un `NULL` en tête d'une liste de paramètres type sa colonne en `STRING` | confort | un refus nommé, contourné dans rag3weaver |
| Un accent grave doublé dans un nom n'est pas réduit | confort | un cas de syntaxe |
| Deux défauts de chaînes annoncés par Ladybug, non reproduits | hors stèle | correctifs d'amont non reproduits |

Ce tri est un avis, pas une décision : sept tickets ouverts bloquent, plus trois défauts
sans ticket ou portés par une condition. Les deux qui demandent une décision de Lucie ou
de l'orchestration sont « durabilité sur faute d'entrée-sortie » (jusqu'où l'exiger) et
« le tampon petit » (confort pour la stèle, bloquant pour le produit).

## 4. Ce qui n'attend pas la stèle

Un agent seul sur un dépôt marche déjà : l'interface des fiches de contexte
et un premier utilisateur extérieur ne dépendent ni des verrous ni des
écritures parallèles.
