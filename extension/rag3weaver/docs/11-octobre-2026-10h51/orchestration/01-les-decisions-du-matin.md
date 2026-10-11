# Les décisions de Lucie au matin du 11 octobre 2026

Prises au réveil, une par une, sur les questions que la nuit avait laissées
(`../../10-octobre-2026-18h20/orchestration/01-rapport-de-session.md`, §9 à
§19). Pour les sessions et pour la reprise. Chaque ligne dit la décision, puis
où elle vit.

## 1. Le moteur

- **I2 (la maintenance de l'index au commit) : plus tard.** « Non bloquant
  pour avancer sur la condition 3, un luxe qu'on prendra après coup. » I1
  reste le produit ; la page 10 du cœur reste prête (tranchée sur la mesure du
  banc : pertes seules filtrée, et la contrainte du pont coupé).
- **Condition 3, les écritures parallèles : A d'abord, puis B par la voie
  Neo4j.** « Je veux plusieurs processus, mais si c'est plus facile de
  commencer par les écrivains dans le processus, ok ; l'un n'empêche pas
  l'autre. » A dans l'ordre du cœur (T0, A7, A3′ reprise, A9 mesure puis
  bascule du défaut sur son coût dit, A8, C7) ; puis B2 (le second processus
  écrivain attend au fichier) et B3 (le démon) ; le multi-processus simultané
  sur un même fichier (mémoire partagée, PostgreSQL) reste écrit comme refonte
  non engagée. Page `3-octobre-2026-23h31/coeur-cpp/11`.
- **T0, un écart pris par le cœur** : au COMMIT d'une transaction en échec,
  refus avec erreur (Neo4j), pas de ROLLBACK silencieux (PostgreSQL). Non
  renversé.
- **La stèle du moteur se juge sur ses tickets à elle** (cinq ouverts ce
  matin). Les tickets « perte ou réponse fausse » du produit (21) ne la
  retiennent pas : ils vont dans **une stèle du produit, distincte**, à
  écrire ; l'index des tickets se scinde en deux tables d'ouverts, moteur et
  produit.

## 2. Le produit

- **Le serveur MCP n'est qu'une interface vers un démon** : la forme C de la
  page `11-octobre-2026-arbre-principal/01-le-demon-possede-le-catalogue.md`,
  ses treize choix tranchés au §9 : un seul démon pour le moment (à moyen
  terme un démon routeur et des démons spécialisés, en vision) ; lancé par le
  premier client s'il n'est pas là, unité système en option du paquet ; un
  outil « état du démon » quand il n'est pas là ; `serve` devient le démon,
  `127.0.0.1:7970`, `daemon.json`, jeton local sur chaque requête,
  `--daemon` ; les clés liées à la requête ; le client MCP = un rôle du même
  binaire ; **le client s'enregistre auprès du démon et reçoit une session
  qui porte sa cellule et ses clés** (plus de « cellule courante » dans le
  catalogue, marche nommée de C ; la sérialisation par backend comme filet
  dans la marche 1) ; les lots de l'ancre avant C.
- **Le constructeur de decks : on l'oublie pour l'instant**, « ça s'est fait
  en une nuit, on le refera après nos avancées ». Aucune exception nommée pour
  lui ; l'ouverture directe d'une base reste pour les tests, la CLI et `serve`.
- **Les noms réservés : échappement systématique** de chaque identifiant dans
  chaque dialecte (accents graves rag3db, guillemets PostgreSQL), bas niveau,
  invisible au concepteur de gabarits, aucune migration ; le préfixe `E_`/`R_`
  écarté (une table de correspondance à cacher partout) ; le refus de la nuit
  tombe. Page `8-octobre-2026-16h29/embarquements/07-echappement-des-noms.md`.
- **Un nœud qui relie deux entités gérées : oui, « toujours générique, et si
  le mécanisme doit être pensé plus loin dans l'abstraction, à faire aussi »**
  — la face dataflow de `Write::Link`, le témoin de la garde existante
  d'abord, puis le huitième refus des réactions.
- **Codeparsers avance au maximum** : un appel sur une locale non typée
  devient visible, non attribué ; T5 ; les receveurs restants ; la couverture
  mesurée par le banc à chaque lot.
- **Les e2e : bâtis une seule fois, le même cœur pour tous** — un seul binaire
  d'e2e lancé une fois par suite, un seul target d'e2e par poste ; mesure
  avant/après par la recherche ; la règle du target (effacé à chaque fusion)
  se tranche après cette mesure. « 100 Go pour un bâti, ce n'est pas normal. »
- **La séance npm et l'essai MCP : plus tard.** « On testera et publiera plus
  tard, pour l'instant on a déjà de quoi avancer. » Les quatre archives
  alpha.2 restent prêtes ; rien ne part.

## 3. Ce qui en découle pour les sessions

| Session | Son chantier après ce matin |
|---|---|
| coeur c++ | condition 3, A : T0 (en passe), A7, A3′ reprise, A9, A8, C7 ; puis B2 |
| coeur c++/hotfixs/tickets | le COPY de blobs sous petit tampon (reproduction, cause, correctif) ; l'index des tickets en deux tables d'ouverts (moteur / produit) |
| rag3db-73 (arbre principal) | la marche 0 de C, puis les marches ; la stèle du produit (page) avec codeparsers |
| rag3db-96 (mémoire) | le nœud Link générique, le huitième refus ; la page C avec A ; la mémoire partagée par le démon |
| rag3db-6f (embarquements) | l'échappement des identifiants ; Tx côté PostgreSQL quand le conteneur existe |
| rag3db-ab (codeparsers) | la locale non typée visible, T5, les receveurs restants, le banc |
| rag3db-97 (recherche) | la mesure des e2e et le binaire unique ; la relecture de dataflow |
| rag3db-10 (banc) | C7 témoin minimal ; les marches de la condition 3 au banc |
| rag3db-90 (optimiseur) | en pause : les archives restent prêtes ; l'alpha suivante portera le correctif ALP et la fin du filet |
| everything declarative (proto) | en pause : la montre sur la porte est faite ; retour en arrière, mermaid embarqué, plusieurs sessions quand C avance |

## 4. Ajouts de la matinée

- **Le lien entre deux identités déclarées** (mémoire, page
  `11-octobre-2026-memoire-longue/01-…`) : le témoin a montré que la garde du
  nœud de liens ne protégeait aucune garantie (copiée sans raison le
  20 septembre) ; la réponse « plus loin dans l'abstraction » est **une garde
  de moins, pas un nœud de plus** — `RelationBatchNode` devient le nœud de
  lien général, face dataflow de `Write::Link`. Tranché par l'orchestration,
  renversable : fondre ; `Catalog::link` refuse lui-même un bout absent, en le
  nommant, avec l'existence vérifiée par lot et le coût mesuré sur
  l'indexation du code ; une seule garde déclarée (`from_states`/`to_states`) ;
  le huitième refus par une déclaration dans le schéma des nœuds. Un défaut
  trouvé en chemin et ticketé : un lien vers un bout absent rendait `Ok` sans
  rien écrire.
- **Le COPY de blobs sous petit tampon** (hotfixs) : reproduit sans harnais,
  c'est le COPY lui-même (un groupe en mémoire qui tient tout un groupe de
  lignes avant d'écrire), pas le point de reprise ; forme générique à décider
  avec le cœur (vidage par morceaux bornés en octets) ; en attendant, un filet
  côté produit chez A : **un paquet est borné en lignes et en octets**.
- **WASM, l'avis de l'orchestration à Lucie** : ni fork de tokio ni réécriture
  de luciole en « vrai tokio » ; ce qui manque dans un navigateur est
  l'entrée-sortie, pas l'ordonnanceur ; le navigateur devient un client mince
  du démon, et ce qui y tourne (graphes, ports, nœuds purs, clics) tient
  derrière un exécuteur de poche. Pas maintenant.
- **Les e2e en un binaire** (recherche, en fusion) : target 23 → 5,5 Go, 97
  binaires → 1, une retouche de `src/` relie 2 fois au lieu de 98 ; 19,5 Go
  de restes d'e2e effacés ; un dossier par suite effacé en fin de passe ;
  plafond du target à 20 Go (accepté par l'orchestration). **La règle du
  target, tranchée par Lucie** : « gardé entre les lots, une fois qu'on a
  fini d'investiguer et d'optimiser la taille » — la règle « effacé à chaque
  fusion » tient jusqu'à la ventilation des 5,5 Go et son lot ; ensuite,
  gardé entre les lots d'une même session, effacé en fin de session, plafond
  20 Go.
- **Les receveurs non typés** (codeparsers, sur master) : visibles, non
  attribués ; les 45 liens de rag3weaver posés par le nom et marqués « sûrs »
  passent à « nom » (le chiffre de 1 492 venait d'un mode que rag3weaver
  n'utilise pas) ; aucun lien perdu.
- **Le COPY de blobs borné en octets** : page écrite, forme retenue, **lot
  reporté après la condition 3** (plusieurs jours sur des invariants du
  stockage pour un chemin de secours couvert par le filet produit) ; hotfixs
  rejoint le cœur sur la condition 3 (A7 + A8).
- **La marche 0 du démon est sur master** (`75de6618f`) : un serveur qui se
  ferme arrête ses indexations au prochain paquet validé et la relance
  reprend sans reprise forcée ; deux défauts attrapés par le témoin
  (une passe en masse interrompue repartait par paquet ; les liens en file
  oubliés à la suspension).
- **Le COPY de blobs : la prémisse ne tenait pas** (A) — le produit n'écrit
  pas ses blobs par COPY ; le point commun des rouges sous petit tampon est
  **le point de reprise forcé des COPY en mode blobs**, imposé par
  rag3weaver depuis le 5 octobre, sur un gros volume. Tranché d'avance par
  l'orchestration, renversable : si la sonde sans forçage est verte,
  rag3weaver ne force plus (le COPY journalisé est le défaut partout, corrigé
  cette nuit, et la raison du 5 octobre est fermée) ; le prix est dit (les
  lignes d'un paquet passent au journal) ; le ticket moteur se recentre sur
  « un COPY forcé de plus d'un quart du tampon échoue », forme (a) reportée.
- **T0 est sur master** (`82a5f4851`) : condition 3, première marche.
- **Le mode blobs sous petit tampon : le forçage n'était pas la cause** (A,
  sonde sur luciepc, table au ticket) — avec ou sans point de reprise forcé,
  deux COPY tombent sur la même assemblée en mémoire (les blobs dans la
  transaction du paquet au-delà de ~3 000 fichiers, puis les 860 000 liens),
  alors que tout passe en fichiers. Rien ne change dans le produit ; la
  limite est dite (mode blobs + petit tampon + gros dépôt) ; la seule forme
  qui couvre les deux est celle du moteur (un COPY borné en octets à
  l'assemblage), reportée après la condition 3.
- **La stèle du produit est posée** (A, `9bd5f1892`,
  `11-octobre-2026-arbre-principal/02-la-stele-du-produit.md`) : P1 rien
  n'est rendu comme sûr qui ne l'est pas ; P2 aucune perte ; P3 aucun refus
  muet ; P4 ce que l'outil ne sait pas est dit. Vingt tickets rangés dessous,
  avec ce qui ferme chacun ; codeparsers y écrit l'ordre de ses lots. Deux
  tickets ont bougé avec la marche 0 (l'indexation enfant du serveur :
  fermeture propre et SIGTERM réglés, reste le serveur tué ; la reprise par
  takeover : levée après un arrêt propre) — A met leurs états à jour.
- **Agents et DAG** (Lucie) : un agent est une boucle autour de graphes
  déclarés ; un agent comme outil d'un autre agent n'est pas encore déclaré —
  même forme que `GraphNode` (un nœud « lance cet agent », ports question /
  réponse / trace, politique et bornes), à faire quand un gabarit en a
  besoin ; avec le démon, « appelle l'agent du backend X » est un appel sur le
  fil.
- **Un DAG est un outil, les agents se parlent : chantier ouvert chez
  « everything declarative »** (Lucie, midi). Contrainte : « un agent est un
  DAG, un DAG est potentiellement un outil quand on a besoin qu'il le soit
  (dont les agents) ; ce que ça veut dire n'est pas forcément d'attendre son
  retour : peut-être juste le lancer en fond avec un message initial et la
  référence de qui je suis, qu'il puisse me parler ». Le relevé de ce qui
  existe (plomberie du 26 août) et de ce qui manque (aucun outil exposé au
  modèle, aucun lancement d'agent par un agent) est au §3 de
  `02-rapport-de-session.md`, avec la forme en cinq points ; page de
  conception courte d'abord, témoins rouges à faux modèle, lots bornés ; le
  côté démon / serveur MCP reste à l'arbre principal.
- **L'échappement systématique est sur master** (F, `4cc23f781`) : chaque
  dialecte cite tout nom du schéma qu'il écrit ; personne au-dessus du dialecte
  ne voit l'échappement sauf la trace et le Cypher brut ; trois refus nommés
  (délimiteur, nom intégré, souligné en tête). Précision pour Lucie : le champ
  intégré est `_uuid` ; un champ déclaré `uuid` sans souligné est un nom
  ordinaire et passe. PostgreSQL traduit Select et Count dans le même lot.
- **La marche 1 du démon est sur master** (arbre principal, `deb423288`,
  `src/daemon_host.rs`) : plusieurs backends par processus, chacun chargé une
  fois et sérialisé ; sessions d'abord (`POST /session`, en-tête sur chaque
  requête, refus nommé sans session) ; une base à rouvrir ne lâche que son
  backend. Suite : marche 1 bis (cellule par session) chez l'arbre principal,
  marche 3 (client MCP mince par HTTP) chez la mémoire, go donné.
- **La boucle d'agent est un graphe à ports** (Lucie, début d'après-midi) :
  « à tout moment pouvoir faire un appel structuré : selon ce qu'a dit
  l'utilisateur une classification, un routage vers un autre agent ou une
  action ; selon ce que quelqu'un lui a envoyé, transmettre ou non ; selon un
  retour d'outil ou une réponse d'agent, faire telle chose en plus ; que tout
  soit traduisible dans le graphe de l'agent, comme un flot de conversation
  mais beaucoup plus permissif, en synergie avec le moteur ». Forme confiée à
  « everything declarative » : la boucle (aujourd'hui `agent.rs:1283`, boîte
  drainée avant chaque appel au modèle `:1339`) devient un graphe déclaré par
  défaut à ports nommés (`user_in`, `peer_in`, `tool_result_in`, `model_out`,
  `tool_call_out`, `answer_out`, `turn_end`) ; sur chaque port un sous-graphe
  fait de quatre nœuds génériques (`decide` appel structuré, `route`,
  `filter`, `act`) ou d'un nœud scripté ; pas d'états ; `send` / `spawn`
  verbes du modèle et nœuds posables ; `interrupt` et la politique de
  livraison réglages de `peer_in`. Les quatre choix de la page
  `11-octobre-2026-everything-declarative/03-les-agents-se-parlent.md` sont
  pris ; l'ordre des lots est revu (branchement conditionnel du dataflow s'il
  manque, puis la boucle à ports avec témoin de parité, puis decide/route/
  filter, puis send, AgentNode, spawn, atelier/MCP, vrai modèle).
- **Le chaînage des formes** (F, page `11-octobre-2026-embarquements/01-chainage-des-formes.md`,
  `26216e3be`) : deux questions à Lucie, arête d'ordre seul (recommandé non,
  avec une arête à zéro ou plusieurs colonnes) et graphe non atomique
  (recommandé : exécuté et dit, refus seulement si l'appelant exige
  l'atomicité). **Tranché par Lucie vers 13 h : oui aux deux**,
  l'arête portant zéro ou plusieurs colonnes ; premier lot lancé chez F.
- **Conteneur PostgreSQL** (Lucie : « mettons un docker postgres ») : recette
  d'`e2e_postgres` (`pgvector/pgvector:pg17`, port 5433) ; l'utilisateur
  n'étant pas dans le groupe `docker`, Lucie le lance elle-même ; la mémoire
  rejoue `e2e_postgres` dès qu'il tourne.
- **L'enveloppe d'un outil d'écriture** : la mémoire a posé ses sections
  (`d03fbfe31`) ; forme A (dire `atomic` et ce qui est resté sur tout outil
  d'écriture) lancée sans décision à attendre ; B (envelopper un `put`,
  invalidation des seuls caches de la ligne) attend deux vérifications de
  l'arbre principal et les quatre points de sa §6 pour Lucie.
- **Les ports portent des flux** (Lucie, après-midi) : un seul mécanisme,
  tout port porte un flux (éléments dans le temps, et une fin), une valeur
  est un flux de un ; le nœud dit sa posture (élément par élément ou flux
  complété) ; `interrupt` = fermer un flux ; vie d'un agent lancé = tant
  qu'un flux d'entrée est ouvert. Consigne : « complexité on s'en fout,
  jamais un argument, l'important c'est que ce soit au top ». Noté en vision
  (`visions/00-vision-generale.md`, section « L'agent est un graphe à
  ports ») et transmis à « everything declarative » comme règle de sa page.
- **Après-midi, sur master** : la marche 3 du démon (mémoire, `d25cad18b`,
  client MCP mince par HTTP, `src/daemon_client.rs`, deux sessions MCP sur un
  même backend vertes, `--demon` refusé en nommant son remplaçant) ; T5 de
  codeparsers (`48ae8e4b5`) ; la §3 de la page de l'enveloppe (arbre
  principal, `9f07e4575` : un outil n'empoisonne que sur ce que le moteur
  impose, un échec qu'il sait défaire invalide ses caches, pas le catalogue) ;
  la page C complétée (`c24328dd8` : réacteur, budget de tampon, cellule et
  clés non appliquées restent hors de la marche 1).
- **La marche 1 bis a sa page courte** (arbre principal, `7900577ad`,
  `11-octobre-2026-arbre-principal/04-la-marche-1-bis.md`) : la cellule passe
  par la requête, les index sont par cellule, le réacteur prend la cellule de
  l'événement ; coût 3 à 3,5 jours d'agent. Trois points : `_uuid` ne dépend
  pas de la cellule (témoin d'abord ; si collision confirmée, défaut P1,
  ticket et mot de Lucie avant migration) ; marques de progression préfixées
  par cellule (oui) ; `set_scope` cellule par défaut pour la ligne de commande
  et les tests seulement, refusé en service (oui). Pas a lancé.
- **La page des agents relue** (proto, `3b1a03f49`) : trois écarts pris
  parce que meilleurs : le branchement conditionnel vient des flux (un flux
  qui finit vide saute le nœud aval, `Skipped` dit au rapport ; le moteur n'a
  aucun branchement aujourd'hui) ; deux nœuds neufs au lieu de quatre
  (`decide` sur le Decider à un jeton ou la sortie structurée, `route` ;
  `filter` est un `route` dont une sortie ne mène nulle part, `act` est
  l'appel d'outil existant) ; pas de cycle, le tour est le graphe et la
  boucle sa répétition à la frontière ; d'abord des points d'accroche dans
  `agent.rs`, puis le tour comme exécution du graphe quand la parité tient.
  **Le lot 0, l'exécuteur de flux dans `src/dataflow/runtime.rs`, est porté
  par la recherche** (exécuteur par niveaux inchangé pour les graphes de
  valeurs, exécuteur concurrent seulement quand un port de flux existe,
  parité prouvée d'abord) ; le proto fait le lot 1 (points d'accroche +
  parité) en parallèle et relit.
- **L'identité par cellule, variante (a)** (Lucie, vers 13 h) : le
  témoin de l'arbre principal est rouge et c'est une perte (la même fiche
  écrite dans une seconde cellule déplace la ligne de la première ; ticket
  `2026-10-11-la-meme-fiche-dans-deux-cellules-n-a-qu-une-ligne.md`). Filet
  d'abord sur master : l'écriture d'une identité qui vit déjà dans une autre
  cellule est refusée en le disant (la perte fermée, le ticket partiel). Puis
  la migration (a) : `_uuid` dérivé de (cellule, identité déclarée), la
  cellule par défaut garde ses identités d'aujourd'hui, aucune base existante
  ne bouge ; raison : une base sans cellules ne change pas d'identité à une
  mise à jour. Le lien générique de la mémoire dit sa cellule, un lien entre
  cellules devient explicite.
- **Début d'après-midi, sur master** : A7 (hotfixs, `c0f50f453`, la file des points de
  reprise ; banc 7 rouges attendus, 251 verts) ; le filet contre la perte des
  cellules (`06980485f`) et le pas a de la marche 1 bis (`9dd4baedd`, la
  cellule par la requête, deux sessions sur deux cellules ; ticket court pour
  le plein texte qui tombe encore dans l'index de la cellule du catalogue,
  fermé par le pas b) ; le pas 1 du lot 0 (recherche, `522fa6780`, une branche
  non prise est sautée et dite au lieu de tuer le run ; `required` des sorties
  respecté au pas suivant, le saut jamais ambigu) ; la forme A de l'enveloppe
  (mémoire, `cd919b053`). En cours : C7 chez le cœur (cause : vecteurs du
  détachement sur l'état du nœud, même défaut dans la pierre tombale corrigé
  avec ; ticket fermé dans le commit du code) ; A8 chez hotfixs dans la forme
  relue (le dernier à partir écrit, plafond par mesure).
- **PostgreSQL pour de vrai** : conteneur lancé par Lucie, `e2e_postgres` 19
  verts, 1 rouge antérieur à l'échappement ; cause trouvée par la mémoire : le
  plancher pg_trgm posé une fois sur une seule session du pool (ticket
  `2026-10-11-postgres-une-requete-a-deux-fautes…`). Forme demandée à F :
  `SET LOCAL` du plancher dans la transaction de la requête sur la connexion
  épinglée, opérateur indexé gardé, le banc par le même chemin ; petit lot
  avant le chaînage.
- **La mémoire en attente de B** : la recette MCP mise à jour de la marche 3
  (pour le `claude mcp add` de Lucie), puis une page de conception « les
  fiches mémoire classées par sortie structurée » (le sujet qui intéresse
  Lucie le plus ; `decide` du proto, pplx-decider candidat à évaluer, Gemini
  pour juger), aucun code avant son mot. Les quatre points de la §6 de
  l'enveloppe sont rendus à Lucie (recommandation : oui aux quatre).
- **Les fiches mémoire classées : un régime d'expérimentation** (Lucie, vers 13 h,
  « je parle des fiches mémoire hein ») : « on essaie, on ne fige pas, on tente
  avec ce qu'on a dans les visions, au mieux et rentable en temps / mémoire GPU
  / efficience, ensuite on regarde les briques posées, l'assemblage, les
  retours, on ajuste ; on garde les gabarits de chaque tentative, datés ». Les
  cinq réponses proposées par la mémoire sont prises ; chaque tentative vit
  dans un dossier daté avec son gabarit, son graphe, sa note de retours et ses
  mesures. Ce régime ne s'étend pas aux autres chantiers.
- **Sur master, vers 13 h 15** : le pas b de la marche 1 bis (`bac552c22`,
  index et recherche par cellule, ticket fermé) ; le lot 1 des agents qui se
  parlent (proto, `d3aa6f281`, les sept ports de la boucle, parité à ports
  transparents) ; le chaînage lot 1 (F, `76cf86cbc`, le type du graphe dans
  l'IR, la vérification commune, le repli découpé, quatre témoins) ; le
  plancher pg_trgm par la requête (F, `56a680169`, `e2e_postgres` 21/21 sur le
  conteneur) ; la recette MCP de la marche 3 jouée de zéro (mémoire,
  `aeddf002a`). La course de `port_libre` des tests du démon (ticket du 10,
  trois sessions touchées) confiée à G. Règle de forme diffusée : « poussé »
  après `git status` propre, aucun rebase en cours, hash lu sur origin/master.
- **Vers 13 h 40, sur master** : C7 après arrêt brutal (cœur, `2c7704b27`, ticket
  fermé dans le commit, stèle du moteur à quatre tickets ; condition 3 :
  T0, A3′ reprise, A7, C7 faits, restent A8 chez hotfixs, A9 mesure puis
  bascule, B2) ; le pas c de la marche 1 bis (arbre principal, `8bb2af2d7`,
  un réacteur par cellule, contre-épreuve rouge sans lui) ; la fusion rag3db
  du chaînage (F, `7da725287`, groupes sous cinq conditions, une instruction
  par groupe, chaque scénario fondu puis découpé avec les mêmes lignes ; suite
  Hop PostgreSQL puis CTE) ; la course de `port_libre` des tests du démon
  fermée (G, `2d3009efc`, l'écouteur du port 0 donné au démon ; suite : les
  binaires de service disent leur port) ; la recette MCP jouée de zéro et le
  binaire de Lucie rafraîchi (mémoire).
- **Trois défauts trouvés en route**, tickets produit : master rouge en
  `cargo check --tests` sous les quatre features (recherche) et sans la
  feature `code` sur `backend.rs:2897` (arbre principal) ; les gabarits livrés
  cherchent l'extension vector relativement à eux-mêmes (`754e863a9`, arbre
  principal : le gabarit nomme l'extension, l'ouverture la résout par le bâti
  ou le binaire et refuse en nommant) ; `e2e_impact` rouge depuis le pas b
  parce qu'il exige un chemin exact à travers une fonction vivante (corrigé en
  rendant au code sa profondeur ; ticket sur la forme du témoin : un
  module-corpus stable, à faire après la migration).
- **La marche 4 avant l'essai MCP de Lucie** (« tu as dit marche 4, tu as
  raison ») : confiée à la mémoire (daemon.json, lancement par le premier
  client avec un seul lancement, jeton, outil « état du démon », cinq témoins
  rouges d'abord, recette jouée de zéro sans rien lancer à la main) ; côté
  hôte relu par l'arbre principal. Le banc des fiches classées passe après
  (branche `banc-fiches-classees` poussée sous son nom).
- **Vers 14 h 45** : **A9 mesuré, coût du mode nul** (cœur, `87137a766` :
  COPY 200 000 lignes, 100 000 insertions ligne à ligne, mille SET de vecteur
  sous HNSW, trois répétitions sur luciepc, artefact prouvé par l'identifiant
  de bâti ; écarts sous le bruit). La bascule du défaut est donc décidée par la
  règle du matin (« A9 mesure puis bascule du défaut sur son coût dit ») :
  lot à part chez le cœur, `enable_multi_writes=true` par défaut, l'ancienne
  sémantique par `CALL enable_multi_writes=false`, renversable par le même
  réglage ; e2e du produit rejoués par l'arbre principal sur la lib neuve
  avant le hash. Puis B2.
- **Le chaînage est au complet** (F : saut PostgreSQL `d0aacfcf7`, fusion en
  CTE `e5a93a913`, scénario commun aux deux moteurs, « jamais une écriture en
  amont » prouvé sur les deux ; témoin des transactions corrigé `b10b49300`,
  trois tickets fermés). Suite : le relevé des vrais appelants (allers-retours
  par outil : code, mémoire, rechercher, réactions), puis le premier appelant
  réel mesuré avant/après avec le propriétaire du fichier en relecture.
- **La garde du target** (recherche, `2306d30f9`, `.arbre-du-target`, refus
  nommé d'un autre arbre) et **son remède de reprise pris en défaut par G**
  (vert vide : `cargo clean -p` n'a rien effacé, un `touch` non recompilé).
  `RAG3WEAVER_TARGET_REPRENDRE=1` retiré de l'usage jusqu'à preuve ; seul
  remède le sous-dossier par arbre ; contrôle avant tout hash : après un
  `touch` d'un fichier modifié, la ligne Checking/Compiling doit apparaître.
- **Marche 4** : côté hôte relu par l'arbre principal, deux corrections
  prises par la mémoire, passe sur la lib C7 en cours, push ensuite.
- **Piste pour après, pas un chantier** (Lucie, 15 h) : un embarqueur
  multimodal local (EmbeddingGemma-2 : texte, images, audio, vidéo dans le
  même espace de vecteurs), « à expérimenter après ça pour des images ». Chez
  nous : un service de plus derrière `RAG3WEAVER_EMBED_SERVICE` qui accepte
  des octets et déclare ce qu'il prend, un gabarit à champ fichier dont le
  vecteur vient du même modèle que les textes ; usages : fiches avec images,
  documents, tisserande. Règle inchangée : local pour éprouver, Gemini pour
  juger, aucune promesse de tout-local.
- **La marche 4 essayée par l'orchestration sur la base réelle de Lucie**
  (trois clients MCP sur stdio) : vert de bout en bout (hôte lancé par le
  premier client, dix outils, rappel des fiches en 0,1 à 0,2 s) ; un défaut
  P4 en ticket chez la mémoire : la session d'un client mort sans fermer reste
  listée par `daemon_status`.
- **La bascule du défaut moteur** : permission levée par Lucie dans la session
  du cœur ; branche `verrous-a9b`, liste complète et banc chez le cœur, e2e du
  produit chez l'arbre principal sur son propre bâti de la branche.
- **15 h 30, après la limite de session de Lucie** (l'orchestration bloquée
  un moment) : les dix sessions pingées, toutes actives, rien n'attend Lucie.
  Recherche : canal et détecteur d'interblocage (12 témoins), puis
  l'exécuteur concurrent (`fd10b8c27` poussé). Banc : protocole de la bascule
  sur luciepc, partie lourde verte. Mémoire : review_anchored 40 drains → 1
  (1 318 → 94 instructions ; une transition d'état ne change plus l'empreinte
  du contenu), ticket « la réaction n'applique pas reviewed_at / revision /
  updated_at ». G : relais de luciepc vers le verrou d'ici. Proto : lot 3
  (send + annuaire) et le `prompt` de Decide verts, en push. Hotfixs : A8
  relu, plafond mesuré (master 210-358 Kio, correctif 74-91). Cœur : la
  bascule a fait sortir un vrai défaut (`CREATE TABLE … AS` sous le mode : le
  verrou d'index pris avant que la table existe), correctif + témoin, tête
  finale à confirmer ; banc et arbre principal rejouent sur la tête finale.
  Codeparsers : anonymes C++ 287 → 0, appels reliés entre fichiers 41,5 →
  50,7 %, sa part de la stèle faite ; **suite : le premier index réel de
  rag3db par l'hôte sur ce poste** (pas 4 de la page 04), la question du cœur
  par le client MCP, puis une recette `claude mcp add code-rag3db` pour Lucie.
  Arbre principal : migration (a) commitée (`34c8ad67b`), push par poste push ;
  file : codeparsers, relecture d'impact, rendu stable, A9 dès la tête finale,
  extension vector.
- **Les éventails, tranchés sur mesure** (F) : en local la fusion ne gagne
  sur aucun moteur (rag3db dans le processus ; PostgreSQL sur la boucle locale
  à 0,1 ms l'aller-retour) et coûte 1 à 20 % ; le gain n'existe qu'avec un
  aller-retour cher (projection, pas mesure). Capacité déclarée `fanout`,
  fausse par défaut sur les deux dialectes, levée par la configuration d'un
  montage distant ou une mesure réelle ; les traductions restent testées ;
  impact reste sur run_plan (−35 à −42 % d'allers-retours sur rag3db, rendu
  identique, temps égal). Les sauts directs de usages_of ne sont pas une
  redondance (arêtes devinées, scope le plus étroit) : gardés.
