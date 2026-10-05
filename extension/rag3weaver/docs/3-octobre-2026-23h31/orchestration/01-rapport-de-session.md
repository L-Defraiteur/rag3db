# Orchestration — rapport de session

Mis à jour le 5 octobre 2026 à 10 h, à la **mise en pause** demandée par Lucie
(reprise un soir ou au week-end). Ce fichier se met à jour sur place.

## 1. Le rôle

La session d'orchestration cadre les autres sessions par messages, relit par
git, et ne remonte à Lucie que les vrais choix. Lucie délègue les choix
réversibles (« prendre le meilleur chemin possible et laisser la voie aux
autres options ») ; un changement de conception ou de rendu visible attend son
mot. Les noms des sessions changent à chaque relance : `ListAgents` les donne.
Au 5 octobre : cœur C++ `rag3db-e2`, banc `rag3db-36`, arbre principal
`rag3db-c4`, embarquements `rag3db-ac`.

## 2. À la reprise : dans cet ordre

1. **La réponse de Lucie sur la branche `defauts-bascules`** (`7b82c761f`, non
   fusionnée ; page `docs/5-octobre-2026-08h45/01-les-defauts-bascules.md`
   dans la branche). Trois questions lui sont posées, voir §5. Si oui :
   l'arbre principal rebase, joue la batterie complète une fois, fusionne.
2. **Cœur C++ : la fuite de pages** (page acceptée
   `coeur-cpp/06-les-pages-sans-proprietaire.md`, ticket
   `docs/tickets/2026-10-05-pages-d-un-copy-journalise-perdues-au-rejeu.md`).
   Rien n'est codé ; les témoins sont un brouillon jamais bâti. Les deux
   mesures manquantes (COPY forcé tué avant son point de reprise, COPY replié
   tué) d'abord. Elle **bloque le basculement** du moteur.
3. **Banc : le lot d'élagage** de l'index vectoriel (i = 0 + règle classique +
   borne des copies exactes), mesures et critère de push au §4 ; puis la mise à
   jour massive de vecteurs, dont la référence se prend sur ce nouvel élagage.
   Le cœur C++ lui doit la relecture.
4. **Embarquements : la série tenue d'avant basculement**, sur un moteur
   ≥ `fb98852e1`, au signal du cœur C++ : fichiers à ±3 %, blobs à leur temps
   d'avant, et ce que le défaut d'aujourd'hui a gagné.
5. Puis, cœur C++ : `RelCopyBMExceptionRecoverySameConnection` (pourquoi le
   repli ne s'est pas déclenché sous un tampon minuscule), les huit tests qui
   supposaient le point de reprise d'un COPY, le reste du banc sous le défaut
   basculé (la passe à blanc s'est arrêtée à 51 cas), le basculement.

## 3. La stèle du moteur, au 5 octobre 10 h

`docs/4-octobre-2026-16h57/01-la-stele-du-moteur.md`.

| Condition | État |
|---|---|
| plus de défaut connu qui corrompt ou qui perd | voir la liste ci-dessous |
| le chargement en masse journalisé | étapes 1 à 3, statistiques dans la transaction, forme compacte, repli au seuil, saut du journal des transactions forcées : sur master, **derrière le réglage**. Reste la fuite de pages, puis le basculement. |
| les verrous | V1 sur master ; A3′, A4′, V2, index au commit après le basculement |
| les écritures parallèles | en dernier |

Fermés le 5 octobre (tous sur master) :

- `836edfc29` — point de reprise sans fin après un COPY annulé, refusé ou à
  court de mémoire (compte et réservation de l'index de clé).
- `eb2d78e46` — **une clé présente sur disque et introuvable** : lecture dans
  une mémoire libérée à la division des cases de l'index de clé, à tout point
  de reprise qui agrandit un index non vide. Défaut d'origine. Trouvé par le
  filet « COPY refusé = rouvrir » de rag3weaver ; avant lui, le repli MERGE
  sautait l'arête en silence. Une base touchée se réindexe (requête de
  contrôle au ticket).
- `99059fb69`, `ff76b1ee3`, `31ad712c5` — DROP de colonne, fenêtres A et B,
  réparation à l'ouverture.
- `62f5c9817`, `1177f5794` — statistiques : recalage au point de reprise, puis
  celles d'un COPY dans sa transaction (deux défauts de notre `f1d8c7190`
  trouvés par lecture, rougis par exécution, corrigés).

Restent devant la condition 1, tous à l'index vectoriel, chez le banc :

- la ligne injoignable (index bâti d'un coup, COPY successifs) ;
- la mise à jour massive de vecteurs (page
  `banc-de-concurrence/04-la-mise-a-jour-de-vecteurs.md`, acceptée).

Reclassés hors de la condition 1 par l'orchestration (Lucie peut renverser) :
l'accent grave doublé dans un nom de table (confort), l'extension chargée au
rejeu sans contrôle de bâti (robustesse).

Du banc : 60 rouges, 179 verts ; 57 des rouges sont les manques de verrous et
de multi-écrivains, c'est-à-dire les marches suivantes.

## 4. L'élagage de l'index vectoriel

Trouvé le 5 octobre par le banc, hérité du premier commit de l'extension chez
l'amont :

1. `shrinkForNode` saute toujours le plus proche voisin (`i = 1`), sur disque
   et en mémoire.
2. Sa règle est inversée par rapport à l'heuristique publiée de HNSW et à celle
   de DiskANN : elle garde le plus lointain d'un amas serré et écarte le plus
   proche. C'est une lecture, cohérente avec la sonde (lignes joignables par
   l'exhaustif, introuvables à efs = 2000), pas encore prouvée par un
   correctif.

Mesuré pour `i = 0` seul : COPY successifs 28-29 rouges sur 30 → 0-1 ; ligne
lointaine 10 sur 10 → 0 ; mise à jour de 10 000 lignes 38,4 s → 13,6 s et
68-79 % → 97-100 % de lignes joignables. Mais sur les 12 278 vrais vecteurs,
les introuvables passent de 2-13 à 9-31, concentrés sur les quasi-doublons
(40,5 % des introuvables ont un voisin distinct à moins de 1e-4, contre 1,5 %
de l'ensemble). L'explication « îlots de doublons exacts » a été réfutée par
la sonde exhaustive.

Décidé : règle classique dans les deux chemins, plus au plus une copie à
distance nulle par liste ; `i = 0` ne part pas seul. Critère de push : aucune
classe de vecteurs ne recule, le rappel@10 ne baisse pas, le bâti ne ralentit
pas de plus de 20 %. Les index existants gardent leur graphe : à recréer.

## 5. Le produit

Série tenue du 5 octobre (dépôt entier, moteur ≥ `eb2d78e46`, médianes de
trois passes) :

| Réglage | Durée | Pic | Première chose cherchable |
|---|---|---|---|
| fichiers, 2 048 × 4 (contrôle, une passe) | 74 s | — | 57 s |
| fichiers, 2 048 × 1 | 78 s | 9,1-9,4 Go | 8-10 s |
| fichiers, 1 024 × 2 | 77 s | 8,6-8,8 Go | 9 s |
| blobs, 2 048 × 1 | 89 s | 14,2-14,7 Go | 8-9 s |

« Cherchable » est lu dans l'avancement, pas par une recherche en parallèle.
La cible de 90 s de Lucie est tenue.

**En attente de Lucie** (branche `defauts-bascules`) :

1. la bascule : plein texte d'une base neuve dans `<base>.fts/`, transaction
   par paquet, 2 048 fichiers validés un par un (recommandée) ;
2. bases existantes en blobs : coexistence (recommandée) ou migration à
   l'ouverture ;
3. les 8-10 s avant la première chose cherchable : acceptées (recommandé), ou
   un premier paquet plus petit (ni codé ni mesuré).

Sur master côté rag3weaver : filet « COPY refusé par le moteur = base à
rouvrir » (`f0f7f0951`, gardé sans date de retrait, avec celui du tampon
plein) ; filet « fermer sans point de reprise après un paquet défait » posé
puis retiré sur preuve (`74df74276`, `52f58815d`) ; lecture seule après une
fermeture sans point de reprise témoignée (`6cb5923c0`) ; marques du plein
texte (`72fa1f452`).

Le chargement journalisé du moteur **ne fait pas gagner de temps** sur cette
mesure (80 s contre 78) : son bénéfice est la stèle (plus de point de reprise
forcé, plus d'attente à la validation). Sondé : 466 Mio de journal pour 62 Mo
de texte — relations 168, texte écrit dans deux tables ~124, coût par cellule
~175 ; plus grosse transaction : le chargement final des relations, 158 Mio.
Seuil du repli : un huitième du tampon, plafonné à 256 Mio. En mode blobs,
rag3weaver demande lui-même le point de reprise forcé (sur la branche).

Options notées, non lancées : le texte d'un morceau relu du parent par
jointure (changement de schéma, choix de Lucie) ; le chargement final des
relations validé par table.

## 6. Deux marches sans retour

- Journal : depuis `1c232f318` (tableaux en octets bruts), un moteur plus
  ancien refuse un journal neuf. **Pas de retour à un ancien bâti sur une base
  dont le journal n'est pas vide.**
- Stockage : le correctif de la fuite de pages portera un numéro de version ;
  une base touchée par le nouveau moteur sera refusée par un plus ancien.
  Accepté par l'orchestration (aucune base à protéger), dit à Lucie.

## 7. Ce qui attend Lucie, hors §5

- Les deux reclassements du §3 et l'aval donné au changement de règle de
  l'élagage (§4) : rendus, renversables.
- Anciens : changement dans lucivy pour une fermeture synchrone (optionnel) ;
  retirer `content_boost` et `special_ops` ; lancer la marche 1 des visions
  (fiches de contexte) ; réécriture de l'historique (`5ea14e1aa`) ; « envoie »
  pour tracel-ai ; ménage des branches distantes ; statut du dépôt.

## 8. Méthode : ce que ces deux jours ont corrigé

- Une mesure se rend en médiane de trois passes sous une seule tenue du
  verrou, avec une passe de contrôle quand le moteur a changé dessous.
- Un défaut déduit par lecture se rougit par exécution avant d'être corrigé ;
  une lecture se dit « lecture ». Deux lectures du banc ont été démenties par
  sa propre mesure dans la journée, et c'est la mesure qui a tranché.
- « Si le rappel baisse quelque part, on ne pousse pas » a évité de livrer
  `i = 0` seul.
- Un filet de rag3weaver qui s'arrête sur un refus du moteur vaut un témoin :
  c'est lui qui a trouvé la clé perdue.
- Une estimation de session en « jours » se compte en passes.
- Mes erreurs : j'ai décrit le filet 1 comme ce qui arrête l'indexation (c'est
  l'empoisonnement du 4 octobre) ; j'ai demandé une borne sur les doublons
  exacts avant que la cause soit prouvée.

## 9. Comment reprendre

1. Lire ce fichier, la stèle, puis `docs/journal-des-chantiers.md`.
2. `git log origin/master --since=<dernière lecture>` : tout se livre sur
   master, en avance rapide, sans force.
3. `ListAgents` ; relancer chaque session par un message qui nomme son lot du
   §2 — chacune a son propre rapport dans ce dossier du jour.
