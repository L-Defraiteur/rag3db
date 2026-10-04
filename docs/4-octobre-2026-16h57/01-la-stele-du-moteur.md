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
- **4, les défauts** : la corruption de mémoire d'`e2e_code` a sa cause et son correctif
  (`57c8389b4`, 4 octobre au soir) : deux exemplaires de la même base ouverts en écriture dans le
  même processus — rag3weaver rouvrait avant la fin de la fermeture, et le verrou du fichier
  n'exclut qu'un autre processus. Le moteur refuse maintenant la seconde ouverture par son
  nom ; rag3weaver doit encore rendre sa fermeture synchrone. En chemin, la traque a donné
  trois refus nommés à la place d'écritures hors bloc et l'index vectoriel dimensionné par
  le nombre de lignes (`1ea49837f`), et deux défauts d'origine de l'annulation d'un `COPY`
  (`05788a868`, `5c8507577`). Les tickets du moteur sont triés au §3.1.

## 3. Ce qui reste à faire pour que la stèle soit utilisable

1. **Trier les tickets** : une ligne par ticket du moteur — bloque la stèle
   (mémoire, durabilité, résultat faux) ou non. Fait au §3.1, à relire par le banc.
2. **L'ordre** : tranché au §3.2.
3. **Poser la version** quand les quatre conditions sont tenues : une
   étiquette git, la date au journal des chantiers, et la règle « on n'y
   revient que sur un défaut constaté ».

### 3.1 Le tri des tickets du moteur (session cœur C++, 4 octobre, à relire par le banc)

Seuls les tickets **ouverts** qui touchent le moteur. Les tickets fermés ne bloquent plus
rien ; ceux de l'analyse de code, de codeparsers et de rag3weaver ne sont pas l'affaire de
la stèle.

| Ticket | Verdict | Pourquoi |
|---|---|---|
| Une corruption de mémoire tue `e2e_code` | corrigé dans le moteur (`57c8389b4`) | la cause : une double ouverture en écriture dans un même processus, maintenant refusée par son nom ; reste à rag3weaver d'attendre la fin de la fermeture |
| Le point de reprise plante, à jamais, après `ALTER TABLE … DROP` | **bloque** | mémoire (SIGSEGV) et durabilité : la base ne peut plus écrire de point de reprise ; rag3weaver ne supprime pas de colonne, mais le défaut corrompt |
| Dans une transaction, un balayage de plusieurs tables de relations relit celles d'une autre table | **bloque** | résultat faux |
| Un `COPY` rend une erreur alors qu'il est validé | **bloque** | résultat faux sur la durabilité : l'appelant qui recommence écrit deux fois |
| L'ordre de synchronisation au point de reprise | **bloque, à éprouver** | durabilité, si le doute est fondé ; un arrêt au bon instant le dit |
| Durabilité sur faute d'entrée-sortie, coupure ou mort au mauvais instant | **bloque, borné** | la stèle exige les cas qu'un arrêt au mauvais instant suffit à déclencher (l'ordre des suppressions à la reprise, le rejeu des pages d'ombre non synchronisé) et, pour le reste, « détecter et refuser de continuer » ; le disque plein et la coupure de courant viennent après la stèle, avec un crochet de faute |
| La réouverture d'une base échoue par intermittence, sous charge | **bloque, tant que la cause manque** | une réouverture qui échoue sur un journal de taille nulle touche à la durabilité ; quatre hypothèses écartées |
| Mise à jour massive de vecteurs : des lignes restent injoignables dans l'index | **bloque — sans ticket, à ouvrir** | résultat faux (journal des chantiers §6) |
| Une ligne très loin des autres est injoignable dans un index bâti d'un coup | **bloque — sans ticket, à ouvrir** | résultat faux (journal des chantiers §6) ; à vérifier depuis `1ea49837f`, qui borne autrement le parcours de la construction |
| Un doublon de clé au journal empêche de rouvrir (H4) | **bloque — porté par la condition 1** | durabilité ; c'est A3′ |
| Un `CHECKPOINT` retient la validation d'un lecteur jusqu'à son délai | **bloque** (relecture du banc) | l'écrivain reçoit une erreur de délai alors que sa ligne est validée : même classe que le `COPY` validé qui rend une erreur — un appelant qui recommence écrit deux fois |
| Le point de reprise échoue quand le tampon du moteur est petit | confort, sous condition | à condition que l'erreur soit nommée et qu'aucune écriture validée ne soit perdue, avec un témoin au banc de cette condition |
| Un `COPY` refusé laisse la cardinalité gonflée | confort, sous condition | ne sert plus qu'au planificateur depuis `1ea49837f` ; à condition que `STATS_INFO` dise qu'il rend une estimation |
| Le point de reprise réécrit en entier une table de blobs | hors stèle | une optimisation (ticket du 4 octobre) ; contournée dans rag3weaver |
| Un `NULL` en tête d'une liste de paramètres type sa colonne en `STRING` | confort | un refus nommé, contourné dans rag3weaver |
| Un accent grave doublé dans un nom n'est pas réduit | confort | un cas de syntaxe |
| Deux défauts de chaînes annoncés par Ladybug, non reproduits | hors stèle | correctifs d'amont non reproduits |

Relu par la session du banc le 4 octobre au soir ; les quatre lignes « relecture » et
« sous condition » sont ses corrections, tranchées par l'orchestration. Le partage : les
tickets « bloque » qui ne sont portés par aucune condition vont au banc (point de reprise
après `DROP` de colonne, balayage de plusieurs tables de relations, `COPY` validé qui rend
une erreur et `CHECKPOINT` qui retient un lecteur, ordre de synchronisation, réouverture
intermittente, durabilité à l'arrêt) ; le terrain du `COPY` et de son annulation, la
corruption d'`e2e_code` et les verrous restent au cœur C++. Les deux défauts de l'index
vectoriel sans ticket sont à ouvrir par le banc, qui en a les témoins.

### 3.2 L'ordre (tranché le 4 octobre, 19 h, choix réversibles)

Lucie : « on se focus sur la stèle ». L'ordre, d'après le découpage du point de reprise
(les 5 s par paquet étaient celles d'une seule table, `_index_blobs` ; sans elle un point de
reprise de quatre paquets coûte environ 4 s en tout : le chargement journalisé n'est plus
sur le chemin des 90 s du premier index) :

1. l'annulation d'un `COPY` (clés de l'index, point de reprise) — fait ;
2. la corruption d'`e2e_code` — fait : c'était une double ouverture de la base ;
3. les verrous : V1, A3′, A4′, V2, la maintenance de l'index vectoriel au commit
   (14 à 20 jours de session) ;
4. le chargement en masse journalisé (6 à 10 jours) ;
5. les écritures parallèles (10 à 20 jours, la plus incertaine).

Les durées sont celles de la session cœur C++, avec une incertitude d'un facteur 1,5.

## 4. Ce qui n'attend pas la stèle

Un agent seul sur un dépôt marche déjà : l'interface des fiches de contexte
et un premier utilisateur extérieur ne dépendent ni des verrous ni des
écritures parallèles.
