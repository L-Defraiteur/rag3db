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
- **Le vrai but du MCP** (Lucie, 15 h 45) : « qu'avec ce MCP vous vous
  sentiez déjà comme dans un IDE adapté à vous ; s'il y a des choses à revoir,
  on reverra ; comme des malvoyants qui ont d'autres sens ». Noté en vision
  (section « Un IDE adapté aux agents »). Application dès le premier index
  réel : les sessions du code posent leurs vraies questions par le MCP et
  tiennent une page « ce qui manque à un agent dans cet IDE », chaque retour à
  grep dit avec sa raison.
- **Vers 16 h, sur master** : la migration (a) (`cc1254ac9`, marche 1 bis
  complète en quatre pas) ; impact sur run_plan avec la capacité `fanout`
  fausse par défaut (F, `0f62e1d60`) ; le recompte de l'index des tickets par
  script (`check_tickets.py`, appelé par check_features, prouvé rouge sur une
  copie faussée) ; la page de `poste push` (G, `6c6a12d82`, quatre règles :
  la commande se protège elle-même, un essai ne prend jamais le verrou réel,
  une avance de .md seuls ne demande pas de rejeu, on rejoue dans le
  worktree et le sous-target chauds) ; luciepc passe par le verrou d'ici par
  ssh ; la page de la surveillance des fichiers (proto, `00618a043` :
  réindexer et retirer un fichier existent déjà, il manque qui les appelle ;
  montres au manifeste, balayage date+taille puis hachage, réaction par lots
  de 300, retard dit par l'état ; 2,6 ms pour 1 653 fichiers).
- **Le premier index réel de rag3db par l'hôte** (codeparsers, `5e944b791`,
  `c41e53986`) : 12 min 27 s, 135 592 morceaux, base 1,2 Go + 728 Mo de plein
  texte, pic de l'hôte 9,8 Go, aucun mur ; la question du cœur par le client
  MCP : 2 appelants sur 3, le verrou de l'index pris par `update` lui-même et
  non montré par impact (locks_via sans lockIndexForWrite, ajouté
  `87cb11f95`), 101 usages par le nom venus de tout le dépôt ; quatre grep et
  dix lectures à côté. Page des manques
  `10-octobre-2026-18h20/codeparsers/04-ce-qui-manque-a-un-agent-dans-cet-ide.md` ;
  cinq manques répartis (nom de la classe et appelant manqué : codeparsers ;
  portée par dossier ou langage, verrous du départ dans l'ordre, verrous des
  appelants : arbre principal après A9). Quatre tickets : l'hôte ne relit pas
  un manifeste modifié et rien ne ferme un backend ; le workspace en lecture
  seule ; on ne peut pas dire ce qu'on indexe (liens symboliques, generated) ;
  on ne peut pas arrêter un index. Recette `claude mcp add code-rag3db …`
  donnée à Lucie. Instantané reflink de la mémoire longue avant l'index.
- **Surveillance des fichiers** (Lucie : « Claude Code ne nous dira pas quand
  il fait un edit, donc il faut surveiller ») : chantier confié au proto après
  son lot 3, l'arbre principal relit l'état d'index ; le crochet Claude Code
  en recette seulement.
- **G, suite** : l'unité systemd utilisateur de l'hôte, en option du paquet
  (page C), jamais par défaut, SIGTERM, refus d'un second hôte.
- **Le Cypher brut de l'indexation du code, relevé et quatre lots** (F,
  `0e69ebbfe`, `11-octobre-2026-embarquements/05-cypher-brut-de-l-indexation.md`) :
  sept endroits, fichier et ligne ; les outils du gabarit n'ont plus de texte
  brut. Décidé par l'orchestration, renversable : (1) les traductions exactes
  par `Select::by_uuids` ; (2) deux prédicats de plus dans l'IR,
  `Predicate::AllOf` et `Predicate::In`, sur les deux dialectes ; (3) une forme
  `Write::Detach { relation, table, side, filter }` (une instruction par
  relation ; la composition Select → Hop → Unlink prouvée équivalente comme
  témoin, pas comme seconde voie) ; (4) les réglages du moteur par des
  crochets du dialecte, pas des formes. Relevé d'avant d'e2e_code sur le
  corpus vivant avant tout code ; l'arbre principal relit en branche ; la
  mesure d'impact sur PostgreSQL ferme le ticket.
- **Le serveur `code-rag3db` branché par Lucie** (`claude mcp add`, 16 h) et
  essayé depuis l'orchestration : `usages {name: update, path:
  src/storage/table/node_table, usage: call}` en 0,65 s rend la définition,
  deux appelants et 88 par le nom seul ; le nom qualifié `NodeTable::update`
  rend zéro sans un mot : manque mis en tête de la liste de l'arbre principal
  (comprendre `Classe::methode`, et le dire sinon).
- **Le serveur code-rag3db dans les sessions du cœur** (vers 16 h 30) : une
  session ouverte avant le `claude mcp add` ne voit pas le serveur ; Lucie a
  relancé le cœur (nouvelle session `rag3db-1f`, contexte entier) ; hotfixs
  attend un `/mcp`. Premiers retours réels du cœur (page 04 des codeparsers,
  `a50256f39`) : sur sa question (les appelants d'une méthode virtuelle qu'il
  venait de changer) grep a gagné, l'outil a manqué l'envoi virtuel et la
  récursion (cause commune trouvée par codeparsers : le nom du scope courant
  exclu de toutes les références, correctif double chez lui et dans la garde
  de code.rs) et a dit « 0 tests » là où les tests passent par Cypher (à dire
  « non suivis », arbre principal) ; mais « verrous pris sur le chemin » lui a
  donné en une seconde ce que grep ne donne pas. Ni retard de la montre ni
  échec complet : un échec de compréhension, indépendant de la fraîcheur.
- **L'index doit suivre l'arbre que l'agent édite** (en tête de la page 04,
  ticket `e7fcbe938`) : l'index d'ici est master à 15 h 22, le cœur édite sa
  branche sur luciepc. Formes : une base par arbre (reflink de celle de
  master puis delta), un hôte par poste et un client qui sait lequel ; au
  minimum chaque réponse dit l'arbre et le commit indexés.
- **L'hôte sur luciepc par un tunnel ssh : oui de Lucie** (17 h). Ordre : G
  installe l'hôte sur luciepc (binaire release hors target, lib commune,
  unité en option, 127.0.0.1:7970 là-bas, tunnel `ssh -N -L
  7971:127.0.0.1:7970`) ; la mémoire donne au client le jeton d'un autre poste
  (`--daemon-json` ou équivalent, jamais en argument), marche 5 de la page C
  « le manifeste dit son hôte » ; codeparsers indexe les arbres du cœur sur
  luciepc, témoin : le cœur voit sa propre modification par le client d'ici.
- **Master rouge sous `rag3db-native` seul** (restes de W1/W2 sous -D
  warnings) : corrigé par le proto en tête de file ; ce jeu devient le
  cinquième de check_features.sh avec le recompte des tickets.
- **L'unité systemd de l'hôte** (G, page `b816c75ae`, lot sur la branche
  paquet-npm `e58e53c94`, non fusionné, non publié) : `npx rag3weaver-unit
  install|uninstall|status`, SIGTERM, relance sur arrêt brutal, le refus d'un
  second hôte ne boucle pas ; cinq témoins sur une unité d'essai.
- **Vers 17 h 20** : le gabarit code s'indexe sur PostgreSQL (F, quatre
  lots `68dbcfc4b` → `c8fae292a`, ticket du Cypher brut fermé ; témoin :
  src/daemon indexé sur le conteneur, impact y rend les mêmes empreintes que
  sur rag3db dans les six cas, 5 à 11 ms contre 42 à 67) ; l'exécuteur
  concurrent des graphes à flux (recherche, `439ca3c9c`) ; la session d'un
  client mort (mémoire, `e9387d545`, pid et instant de démarrage) ;
  review_anchored (`bfec631cc`) ; la mesure des marques de codeparsers
  (`339ebb412` : chaînes et commentaires hors références, nom propre par
  position, gabarit explicite `b473c3a` ; ticket « `new` mot vide en Rust »).
- **La bascule prouvée par le banc** (sur 4ee70e386 contre master : mêmes
  résultats des deux côtés, 260 transactions, 27/27, probabilistes et banc
  identiques) après le « fini » des 116 e2e du produit : le cœur pousse par
  poste push ; l'arbre principal rebâtit la lib d'ici une fois après
  (`RAG3WEAVER_MOTEUR_ANCIEN=1` déclaré d'ici là pour les lots du produit
  seul). Trois rouges 10/10 sur master hors bascule en tickets (banc).
- **L'hôte de Lucie redémarré deux fois** (16 h 55 : binaire neuf, montre
  absente parce que sa copie du manifeste datait d'avant ; 17 h 10 : copies
  complétées avec sauvegardes, la montre tourne : fichier modifié vu en 5,8 s,
  créé en 4,7 s, retiré en 4 à 6 s). Un fichier créé est vu (la montre relit
  la liste de la source). Ticket : l'hôte hérite de l'environnement du
  premier client (forme : une configuration à lui, lue au démarrage).
- **L'hôte sur luciepc** (G, page `3f7ef3102`, unité utilisateur, RUNPATH
  inscrit dans le binaire, jeton dans le daemon.json de là-bas qui change à
  chaque démarrage, tunnel `ssh -f -N -L 7971:127.0.0.1:7970 lucied@luciepc`,
  rien d'ouvert sur le réseau : la sécurité est ssh plus le jeton, Tailscale
  n'est que le réseau) ; l'index des deux arbres du cœur y tourne
  (codeparsers) ; le client désigné avec son jeton chez la mémoire. G :
  archives npm alpha.3 sans publier.
- **poste push, précisions** : les pushes de docs seules restent dispensés du
  verrou ; sous le verrou, une avance de .md seuls se rebase et se repousse
  dans le même tour sans rendre le verrou ; la règle 3 dite par les chemins
  du bâti (hotfixs, `481afb550`).
- **17 h 40 : le mode multi-écrivains est le défaut du moteur** (cœur,
  `a0a79016e`, par poste push ; preuves lues avant : sa liste complète et le
  banc sur la tête rebasée sur A8, le banc sans écart entre master et la
  bascule, les 116 e2e du produit verts sur l'artefact prouvé).
  `CALL enable_multi_writes=false` rend l'ancienne sémantique, renversable par
  Lucie. Condition 3, voie A faite : T0, A3′ reprise, A7, A8, C7, A9 ; reste
  B2 puis B3. L'arbre principal rebâtit la lib d'ici sur cette tête.
- **La montre, deux défauts trouvés sur la base de Lucie** : pas de
  réconciliation au démarrage (hotfixs : une fonction d'A8 introuvable, la
  montre ayant pris le disque de 17 h 10 comme point de départ ; forme de
  Lucie : la référence est ce que l'index a enregistré par fichier, reprise
  de ce qui a bougé depuis, pendant un index entier les événements attendent)
  et 0,7 cœur en continu au repos (mémoire, ticket `db65e8c99`) : les deux au
  proto, en tête de sa file. La lenteur de l'hôte de 17 h 10 était passagère
  (réouverture et réindexations sous le verrou du backend).
- **Les sessions du cœur et d'hotfixs ont les outils** (relancées :
  `rag3db-1f`, `rag3db-cc`) ; retours réels dans la page 04 ; les deux index
  des arbres du cœur sur luciepc faits (450 et 490 s, pics 7,9 et 12,1 Go) ;
  la branche envoi-virtuel de codeparsers passe la question du cœur (trois
  appelants, « par envoi virtuel ») et met « index : racine @ commit, indexé
  il y a N min » en tête de chaque réponse.
- **Les correctifs du produit restés sur paquet-npm** (G) : la porte de
  l'embarqueur absent (trois portes sur quatre refusent un gabarit à vecteurs
  sans service, sur master), puis le lot B par relecteur (B1 backend et
  connexions → arbre principal, B2 dialect.rs → F, B3 build.rs → cœur ; avec
  la règle des chemins normalisés en « / », question de Lucie), puis le lot C :
  la marche 4 ne tient que sous Linux (ticket `2307f4470` ; macOS lance un
  second hôte parce que pid_alive lit /proc ; forme portable, « inconnu » dit
  là où une plateforme ne sait pas, CI Windows/macOS du paquet). L'alpha.3
  Linux se fait ; Windows et macOS après C.
- **18 h 10** : le client vers un hôte distant est sur master (mémoire,
  `e60f2342e`, `--daemon-json-command`, jeton jamais en argument, relu sur un
  403 ; README §7, marche 5 à la page C) ; l'hôte de Lucie redémarré à 17 h 50
  sur ce binaire, lib neuve prouvée ; Lucie a branché `code-rag3db-luciepc`
  (sécurité : ssh plus le jeton, l'hôte n'écoute que sur l'adresse locale de
  luciepc, Tailscale n'est que le réseau). Premier retour réel du cœur par le
  tunnel : usages juste et complet (un appelant qu'il n'aurait pas cherché),
  search_code pertinent, « l'outil a gagné sur grep ».
- **Puis l'essai de Lucie a trouvé quatre défauts** : le serveur était lié à
  `code-bascule`, dont la racine (le worktree du banc) venait d'être effacée
  (search_code rend une erreur brute ; la montre retire 6 000 fichiers par lots
  en tenant le catalogue : « catalogue occupé » pour une lecture ; l'arrêt
  propre de l'hôte attend la fin de la réaction, plus de six minutes), et une
  recherche indexée sur un nom inconnu rend trois résultats confiants sans
  lien. Les trois premiers au proto en un lot (racine absente → montre en
  pause et dite, rien retiré, filet contre un retrait massif ; lecture qui
  réessaie une seconde au lieu de refuser ; arrêt qui coupe la réaction entre
  deux fichiers), le quatrième à l'arbre principal (« aucune correspondance
  lexicale » dit avant les résultats vectoriels). Le fichier d'essai de Lucie
  était dans l'arbre de `code-principal` : l'essai est à refaire dessus.
  Règle : un worktree et sa base vivent et meurent ensemble, la session qui
  tient le worktree fait créer et retirer le manifeste (page 04, `64c6e351d`).
- **Autres** : les témoins de B2 écrits par le banc et donnés au cœur
  (branche, rouges sur master pour la bonne raison) ; les correctifs du
  produit restés sur paquet-npm reviennent sur master par lots (G : A
  embarqueur absent relu, B1-B3 par relecteur, B4 les chemins relatifs en
  « / » décidé, C la marche 4 portable) ; F mesure les 2,2 ms par aller-retour
  par phase avant tout code de cache ; la recherche clôt le pas 3 du lot 0 par
  la batterie complète ; les sessions du cœur et d'hotfixs ont été relancées
  (« coeur c++ », `rag3db-cc`).
- **18 h 40, l'hôte de luciepc arrêté de force par Lucie** après 42 min
  d'arrêt propre qui n'en finissait pas (la réaction retirait les 6 000
  fichiers de l'arbre effacé) : preuve avant (les deux bases gardées fermées
  sans journal, seule code-bascule, à jeter, encore ouverte), codeparsers
  refusé par sa permission et sans contournement, les quatre pas faits par
  Lucie, l'unité a relevé l'hôte, code-principal saine, le fichier d'essai vu
  par la montre (définition et appelant), édition à chaud ≤ 11,2 s. Ligne au
  ticket de l'arrêt (`9895c7254`). L'arbre principal de luciepc est sur
  mtg-experiments du 20 sept. : `code-lourd` (l'arbre de la lib commune, qui
  suit master) indexé tel quel, réponse « @ a0a79016e + modifications non
  commitées » ; son sous-module était extrait hors du pointeur (piège n° 1),
  le cœur et hotfixs le remettent au prochain rebâti, l'arbre n'est jamais
  effacé.
- **La priorité de Lucie, 19 h : « que le cœur ait son worktree, n'importe
  lequel, indexé, et à jour quand il édite »**. Forme et ordre : la
  réconciliation de la montre (proto, en cours) = le delta d'une branche ; une
  base par arbre née d'un instantané reflink de celle de code-lourd ; deux
  routes de l'hôte, ouvrir et fermer un backend par manifeste (arbre
  principal, après les lots de codeparsers, ferme trois tickets) ; une
  commande `code-tree add|remove` et un manifeste stable `code-coeur`
  repointé sur l'arbre courant (codeparsers, page puis code, arbre principal
  en relecture) ; condition : l'identité relative des fichiers, lot B4 de G,
  qui passe avant B1-B3. Résultat visé : un seul serveur MCP pour le cœur,
  jamais recréé, qui suit son worktree.
- **Autres** : `poste push --lourd` (ordre fixe des verrous, G) après un
  verrou de push tenu 28 min derrière une mesure ; check_tickets avant tout
  push de docs qui touche l'index (un push de docs du cœur l'avait cassé) ; le
  pas 3 du lot 0 clos (recherche, 586 verts) ; les témoins de B2 ont attrapé
  une vraie perte dans la première forme du cœur (journal lu avant l'attente
  du verrou), corrigée ; le témoin du visited set donné à hotfixs ; retrait
  par ensemble demandé par Lucie (« repérer un îlot et le retirer d'un coup »)
  : au proto par les formes de l'IR (Select In, Detach par relation, Delete),
  F en relecture.
- **Plusieurs worktrees sans faire attendre les autres** (Lucie, 19 h 15,
  « pour après ») : ce qui couple les sessions aujourd'hui et son remède :
  l'index (une base par arbre, en cours) ; master (séquentiel par nature, le
  verrou tenu quelques minutes si le rejeu court est court) ; le poste (le
  partagé, les mesures sur luciepc, la limite est le matériel) ; **la lib
  commune du moteur**, dernière chose qui couple tout le monde : un magasin de
  libs par identifiant de bâti (une lib par commit du moteur, rangée une fois,
  jamais réécrite, chaque arbre charge celle de son commit, le contrôle de
  bâti fait le reste). Noté pour G après ses lots, pas avant.
- **Vers 20 h** : nom-seul (`cad384056`) et les anonymes C++ (`66752a44e`)
  sur master, toute la part de codeparsers passée par l'arbre principal ;
  envoi-virtuel en push ; les routes de l'hôte (`636c4b392`) en file ; le
  banc des fiches classées avec ses deux tentatives datées (mémoire,
  `30ce3aadc` : le petit modèle se trompe sûr de lui, Gemini tronqué par le
  plafond de Decide) ; le ticket des verrous du cœur fermé (`b4655da39`) ;
  `CALL build_id()` (hotfixs, `48b74ead1`) ; la mesure de F (`821d70a63`,
  page 06) : **l'exécution domine, et dans l'exécution les fils** : une
  lecture d'outil de quelques lignes reçoit 32 fils, impact à 1 fil va 47 %
  plus vite ; suite : l'ingestion mesurée à 1/4/32 fils, puis une connexion
  de lecture à peu de fils (capacité du dialecte), puis l'instruction
  préparée ; ticket moteur à ouvrir « fils proportionnés à la cardinalité
  estimée ».
- **Master rouge une demi-heure** (30ce3aadc, deux jeux de features) :
  cause dite par la mémoire, son script comptait « 4 lignes VERT » (cinq jeux
  depuis l'après-midi) et lisait `$?` derrière un tube ; corrigé `519ae1d9f`.
  Règle diffusée : vert = aucune ligne ROUGE dans la sortie écrite dans un
  fichier ET le vrai code de sortie lu sans tube, jamais un compte ;
  check_features.sh rend une ligne finale et un code (recherche). Autres
  règles du soir : master lu par le FETCH_HEAD de son propre fetch (les
  worktrees partagent origin/master, G) ; check_tickets avant tout push de
  docs qui touche l'index ; `poste push --lourd`, ordre fixe des verrous ;
  B4 de G : une racine locale par base, la seconde nommée.
- **luciepc, soirée** : l'hôte arrêté de force deux fois par Lucie (18 h 40
  après 42 min ; 19 h 45 après un figement de l'index de code-lourd pendant
  un checkout et un rebâti de la lib sous lui, mémoire à 30 Go ; la lib avait
  bien été remplacée par un fichier neuf, suspect écarté ; piles perdues ;
  ticket du figement chez le proto : « finir proprement et reprendre ») ;
  règle : prévenir codeparsers avant tout rebâti de la lib commune et tout
  checkout de rag3db-lourd, surtout pendant un index ; code-principal saine,
  code-lourd à réindexer, chemin
  `/home/lucied/.config/rag3weaver/code-lourd/templates/backends/code/backend.json`.
- **21 h – 21 h 40, nuit** (Lucie dort depuis 21 h 05 ; le fil continue ici
  toutes les 30 minutes). Le binaire de Lucie (`~/.local/bin`, md5
  69adbfe94735) est **réparé** par la mémoire et vérifié à l'écart du réel :
  ses sessions neuves démarrent de nouveau ; son hôte d'ici sera redémarré
  à 23 h, annoncé aux sessions MCP, puis code-rag3db réindexé par
  codeparsers. Orchestration compactée à 21 h 10 (sa mémoire pleine) :
  les dix sessions pinguées, dix réponses, aucune n'attendait quelque chose
  d'elle. État des files : hotfixs, visited set en rejeu court sur B2 puis
  le rebâti unique de la lib commune ; G, lot A en push, B4 derrière ;
  banc, file vide, plus aucun worktree ; proto, un push de quatre commits
  (fichier créé indexé, Route et Decide en flux, réconciliation et balayage
  par dates, les trois défauts de luciepc) puis la mesure par phase des 6 s ;
  recherche, batterie de clôture du lot 0 ; F, saut MENTIONS de 122 ms à
  13-22 ms selon la forme, ticket du semi-masque `edb7e8738` ; cœur, lot ABI
  en bâti, mesure des fils avec la trace ; arbre principal, délai
  d'embarquement écrit et en compilation (60 s au manifeste, lot expiré
  rejoué morceau par morceau, muet nommé et exclu, état visible même
  occupé) ; mémoire, client qui se rétablit seul en test ; codeparsers,
  réindexation de luciepc lancée à 21 h 07 derrière le verrou, code-tree
  avec son squelette. **Décision (renversable)** : le magasin de libs par
  identifiant de bâti (page de la recherche `2a41f171c` : aucun binaire n'a
  de rpath, le pont cxx ne surveille ni la lib ni l'en-tête, l'identifiant
  existe sans être comparé au binaire) est **porté par la recherche**, pas
  par G dont la file est pleine ; d'abord deux petits correctifs (rerun-if
  du pont, refus nommé binaire ↔ lib qui ferme le ticket du cœur), puis le
  magasin, G consulté avant ; aucun rpath sans passer par l'orchestration.
  Observation sur le `poste` : la mesure du cœur tient la porte exclusive de
  luciepc depuis 20 h 44 en attendant la place partagée, occupée par son
  propre bâti ; codeparsers puis hotfixs attendent derrière ; la
  réindexation de luciepc partira après 22 h 30, « au plus tard 1 h » tient.
  Question de forme à proposer à G si ça se reproduit : la porte prise
  avant la place fait attendre tout le monde derrière un bâti.
- **21 h 40 – 22 h 05, nuit**. **La montre est sur master** (proto,
  `94354595c`, quatre commits depuis `5c44db6a0`) : réconciliation au
  démarrage, balayage par dates avec ses compteurs dans index_state, fichier
  créé indexé, racine disparue qui met la montre en pause, lecture qui attend
  un catalogue tenu brièvement, arrêt qui coupe une réaction longue, registre
  d'indexation par racine, Route et Decide en flux ; sept tickets fermés.
  Correction de la mémoire : la montre au repos coûtait **7 %** d'un cœur, pas
  69 % (facteur dix), et le lot ramène un tour au repos à 9-22 ms. Pour que
  les hôtes la portent : binaire de Lucie rebâti sur master ≥ 94354595c par
  la mémoire (bâti vers 22 h 15, md5, nm, hôte d'essai sur 7990, installation
  par renommage, redémarrage annoncé à 23 h, décalage dit si non prouvé à
  22 h 50, remesure du coût ensuite) ; hôte de luciepc rebâti et relancé par
  G après la réindexation de codeparsers, avec les annonces.
  **Le poste a perdu une heure au cœur** : sa mesure des fils a expiré à
  21 h 45 sans courir, la porte prise mais la place partagée tenue par des
  lourds qui « entraient quand même » après 600 s. Petit lot donné à G,
  **sur master en moins d'une demi-heure** (`74f760801` ticket, `e70a78a72`
  correctif, déployé sur les deux postes, md5 00406a82, `scripts/poste` entre
  dans le dépôt avec `test_poste.sh`, six cas, trois rouges avant) : dès
  qu'une mesure attend, aucun lourd n'entre ; le délai d'une mesure ne court
  qu'une fois la place prise (`POSTE_MESURE_MAX=<s> poste mesure …`, 124 au
  dépassement, plus jamais `timeout N poste mesure`) ; `poste --qui` dit qui
  tient et qui attend depuis quand ; le script garde ses verrous pour lui.
  Le cœur relance sa trace après son push ABI. **Le lot 0 de la recherche est
  clos** : 596 passés, 1 échoué hors du lot (le banc des fiches classées
  refuse sans ses services, à raison) ; choix : une liste « bancs à
  services » de `run_e2e.sh`, la suite nommée au départ, au résumé et à la
  dernière ligne (« complète hors bancs à services »), pas une feature, pour
  qu'elle reste visible (mémoire, `edfe2aa9d`, §8 bis de la page des e2e).
  La recherche est sur le magasin de libs (rerun-if du pont d'abord). G :
  conflit de son lot A avec master dans backend.rs, résolu et repoussé, puis
  B4. Codeparsers : page code-tree du soir (`f5018ed4b`), réindexation de
  luciepc en cours après le « fini » du cœur.
- **22 h 05 – 22 h 35, nuit : le gel de luciepc expliqué**. code-lourd s'est
  figé une **quatrième fois** à 22 h 02 (pid 132823, 28 369 / 138 579
  vecteurs, 20 %), **sans la montre** : l'hypothèse tombe. Codeparsers ne
  peut pas arrêter l'hôte (permission refusée) ; décision : **personne ne
  l'arrête cette nuit**, le refus d'une session ne se contourne pas par une
  autre, et l'état figé est une preuve ; section « Pour Lucie » en tête du
  rapport de codeparsers (`de35f8036`, mis à jour depuis), note de hotfixs
  (`c89f24285`). L'enquête, sans gdb (ptrace_scope), par relevés en lecture
  seule de codeparsers sur /proc et le journal, et par la carte des verrous
  de l'arbre principal : (a) « interblocage » : non, les fils du moteur sont
  immobiles mais 24 fils d'indexation avancent (~1 cœur) ; (b) « lucivy
  bloqué dans la reconstruction du plein texte lancée par search_code » :
  non, aucune recherche après le démarrage (le cœur n'a rien appelé depuis
  20 h 29), un seul fragment, et la base écrit encore ; (c) **la boucle de
  réclamation** : oui, prouvée : les 24 fils sont nés à 22:00:55 de l'appel
  `index` de codeparsers à la seconde près, et le `.wal` croît régulièrement
  de 255 à 376 Ko toutes les 10 s (les mêmes lignes de réclamation
  réécrites à chaque tour), vidé par un point de reprise à 22:21:13 puis
  repart. Le chemin, lu dans le code par l'arbre principal : `EmbedNode`
  saute sans le marquer un morceau réclamé dont `_text` est vide ou nul
  (record_nodes.rs:2063), la passe compte les réclamés comme faits
  (catalog.rs:3835) et reprend toujours ses propres morceaux ; dès que les
  512 premiers morceaux dus sont tous de ceux-là, `run_index` tourne sans
  fin, zéro progrès, aucun appel au service, le catalogue repris et rendu à
  chaque tour (d'où « catalogue occupé » pour les lecteurs). **Deux défenses
  écrites** (arbre principal, sur son lot du délai, push dès témoin rouge
  vérifié) : un morceau sans texte n'est jamais dû (dette, sélection,
  réclamation, les deux dialectes) ; une passe qui réclame sans faire
  baisser la dette s'arrête en erreur nommée. Le moteur est hors de cause
  (cœur et hotfixs : toutes ses attentes sont bornées). **Cause encore
  ouverte** : d'où viennent des morceaux sans texte (ni codeparsers ni la
  découpe n'en créent, deux garde-fous) ; candidats : lignes abîmées de la
  base de code-lourd (deux arrêts brutaux, le COPY refusé après réservation
  du 5 oct.), binaire lu en lossy ; à compter base fermée après l'arrêt de
  Lucie (`MATCH (n:Scope_Chunk) WHERE n._text IS NULL OR n._text = ''`),
  F compte sur sa copie fermée de code-rag3db-depot. **Pour Lucie au
  réveil** : arrêt propre (`systemctl --user stop rag3weaver-daemon`), la
  pile gdb facultative, le comptage, la montre remise, le binaire prêt de G
  (`~/.local/bin/rag3weaver-backend.pret`, master b226c57fe avec la montre,
  md5 251f757a…, RUNPATH vers rag3db-lourd ; G le rebâtit avec le correctif
  si le push arrive avant). Hotfixs ne rebâtit pas la lib de luciepc sous
  l'hôte figé (sa forme par déplacement d'inode refusée par son classifieur,
  il n'insiste pas, attend l'arrêt de Lucie). **Ici, 23 h** : redémarrage de
  l'hôte de Lucie par la mémoire sur master ≥ 94354595c, attendu jusqu'à
  23 h 30 si le correctif de la boucle arrive ; **pas de réindexation
  entière** ensuite, la montre réconcilie au démarrage et on mesure ce
  qu'elle reprend ; si l'hôte d'ici se fige, personne ne l'arrête. Lots
  donnés : banc, le témoin rouge du balayage sous masque (lignes lues ≤ c·k,
  à 1 et 32 fils) pour le cœur ; hotfixs, le CHECKPOINT explicite ; proto,
  son lot suivant poussé sans `--lourd` local (rejeu sur luciepc sous le
  `poste lourd` de là-bas, règle à noter pour G). Magasin de libs : les
  binaires de luciepc ont un RUNPATH (recette de G), ceux d'ici aucun ;
  avec le magasin, le RUNPATH visera le dossier d'un identifiant (page 07
  corrigée). Sur master : visited set (hotfixs `3e0f98f08`, H4 stable après
  trois semaines, 40/40 au banc), code-tree au soir (`f5018ed4b`).
- **22 h 35 – 23 h 05, nuit : 20 % des morceaux de code sont sans texte**.
  F a compté sur sa copie saine de code-rag3db-depot (20 h 24, aucun arrêt
  brutal) : **26 092 Scope_Chunk à `_text` vide sur 128 732**, des centaines
  par gros fichier (lucivy sharded_handle.rs 301, tree-sitter-scss parser.c
  211, catalog.rs 141, dialect.rs 94…). C'est le point exact où luciepc se
  fige à chaque fois (18-20 % : les pleins finis, restent les vides, repris
  sans fin) : la boucle est générale, pas un accident de base abîmée. Et
  codeparsers n'en produit aucun en mémoire (130 228 morceaux, 0 vide) : la
  perte est à l'écriture. **Ticket P1** `6cdef9585`, pris par l'arbre
  principal : piste le chargement CSV en masse (`cellule_csv`,
  record_nodes.rs:695, un `\` non protégé sous escaped_newlines ; les
  fichiers touchés en sont pleins), témoin rouge (un fichier à `\` passé en
  masse), correctif, puis un chemin « réécrire les morceaux sans texte » que
  l'index lance de lui-même (la montre ne les reverra pas, le fichier n'a pas
  changé). En attendant, les défenses (`fa652a320`, en push depuis 22 h 57)
  disent le compte partout : journal de fin d'index, index_states, et
  l'en-tête « index : racine @ commit » du MCP (« N morceaux sans texte »),
  pour que l'agent sache que son IDE est borgne ; une ligne à la page des
  manques (04) : des faux négatifs de search_code et usages jusqu'au
  correctif. **Question pour Lucie**, pas tranchée cette nuit : un morceau
  sans texte doit-il exister (un scope sans embarquement mais avec ses
  relations) ou ne jamais naître.
  **luciepc bloqué jusqu'au réveil** : codeparsers ne peut pas terminer son
  propre client d'index (pid 132918, place partagée de luciepc tenue depuis
  22 h, adressé à l'hôte figé), son refus de 22 h 12 le couvrait ; personne
  ne le fait à sa place. En premier au réveil : terminer ce client libère la
  place, avant même l'hôte ; code-principal n'a pas été réindexé (sa demande
  n'a jamais été envoyée) ; section « Pour Lucie » à jour (`1f2771d5d`).
  **La nuit se réorganise ici** : la place d'ici était tenue pour rien de
  21 h 54 à 22 h 45 par F (son script lançait un `poste` sous un `poste`,
  qui a attendu son propre verrou 40 min ; faute dite et corrigée) ; le
  proto a rendu le verrou de push tenu 30 min en attendant la place de
  luciepc et a proposé la **forme optimiste** (rejeu sur une base notée hors
  du verrou ; sous le verrou seulement fetch, comparaison égale ou docs
  seules, push ; sinon rendre et rejouer), acceptée et écrite dans la page
  du poste push (`fdef26450`) ; son lot attend le matin et luciepc, rien ne
  bâtit ici devant l'écran de Lucie sauf le cœur. Trois défauts du poste
  pour G après B4 : le rejeu distant qui n'a pas besoin de `--lourd` local
  (le cœur refusé vingt fois, code 75) ; le poste sous un poste ; `--qui` qui
  n'a pas montré un tenant du push vivant (le cœur). **Ordre des pushes de
  code** : l'arbre principal (défenses), puis G (lot A, quatre courses
  perdues contre le moteur : fenêtre accordée, rejeu sous le verrou ~20 min,
  les docs passent), puis le cœur (lot ABI, bâti ici, tour de 23 h 00
  interrompu avant tout push). **Redémarrage de l'hôte de Lucie décalé** :
  la mémoire guette `fa652a320` jusqu'à minuit, bâti prouvé (md5,
  --describe, hôte d'essai 7990, client de boutique), installation,
  redémarrage, cinq minutes de surveillance, aucun arrêt en cas de gel ;
  sinon à minuit le binaire déjà prouvé (6d4f4f5e0039, sur c89f24285 avec
  la montre). Le binaire prêt de luciepc sera rebâti une fois par G sur
  code-tree + défenses, sans l'installer.
  **Sur master** : code-tree (codeparsers, `a601b8308` : add et remove, une
  base ne se copie que fermée, témoin par un vrai hôte ; troisième brique
  de la priorité) ; le témoin du balayage sous petit masque (banc,
  `b808a3bee`, known_red : 69 632 lignes lues pour 117 sur 72 000, le
  contrôle du plan contre un faux rouge sur petite base) ; le client mince
  qui se rétablit seul (mémoire, `1266adef0` : coupure dite et non rejouée,
  démarrage ≤ 4 s, rebranchement toutes les 10 s, list_changed) ; les
  rerun-if du pont cxx (recherche, `b8facf434`, lot 1 du magasin) ; ticket
  du témoin de réconciliation qui rougit une fois sur cinq (`5a1b1df11`,
  correctif avec le lot du matin du proto : le reçu occupé porte `watches`).
  **Décisions** : le refus nommé binaire ↔ lib (recherche, lot 2) prend la
  forme (b) + (a) : nos binaires sont liés en BIND_NOW, un contrôle dans le
  processus arrive après le « undefined symbol » ; donc l'identifiant de
  bâti dans une section ELF nommée des deux côtés, lu avant tout chargement
  par une sous-commande du backend et par l'ExecStartPre de l'unité (le
  PG_MODULE_MAGIC de PostgreSQL), la ligne dans src/ relue par le cœur, le
  filet dans le processus en plus, le RUNPATH par identifiant au lot du
  magasin ; la trace des fils du cœur ici en `poste mesure` (« mesure prise »
  23 h 00), la mesure 1/32 fils du banc derrière avec le même instrument.
- **23 h 05 – 23 h 40, nuit**. Le rouge qui a arrêté le push des défenses
  (23 h 05) était le témoin de réconciliation de la montre du ticket
  `5a1b1df11` ; l'arbre principal a corrigé la cause plutôt qu'écarter le
  test (`cce19d031` : index_states lit les montres avant de prendre le
  catalogue, le reçu occupé les porte, comme l'embarquement en cours) ; le
  proto garde son témoin déterministe pour fermer le ticket au matin
  (`2abe3a54d`), conflit d'index_states à résoudre en gardant les deux
  formes. La place d'ici a été tenue de 22 h 56 à 23 h 32 par la sonde sur
  disque de codeparsers (la cause des morceaux sans texte : un index du
  dépôt sans puis avec vecteurs ; trois variantes jouées suffisent, la
  quatrième terminée proprement à ma demande, 25 min épargnées) ; depuis,
  la mesure du cœur (trace des fils, `e2e_mesure_impact`) tient la place en
  exclusif, le banc (petit masque) attend derrière, puis les lourds (le
  cœur, codeparsers), puis le rejeu des défenses et G sous le verrou de
  push. Le poste corrigé fait ce qu'on lui a demandé : plus aucun lourd
  n'entre devant une mesure. Rien n'est sorti ; master vert ; l'hôte de
  Lucie attend minuit (guet de `fa652a320`).
- **23 h 40 – 0 h 10, nuit (12 octobre)**. **Deuxième mesure perdue au
  poste** : la trace des fils du cœur a tenu la place exclusive de 23 h 32 à
  0 h 02 sans courir, parce que `run_e2e.sh` appelle lui-même `poste lourd`
  pour son bâti et qu'avec la priorité de la mesure ce lourd intérieur
  attend sa propre mesure sans fin. Le ticket « un poste sous un poste »
  passe devant B4 chez G (forme : le poste reconnaît un ancêtre tenant par
  la chaîne des parents de /proc et passe, sans variable ; témoin dans
  test_poste.sh) ; d'ici là, une commande de mesure n'appelle pas le poste.
  Le cœur relance sa trace sous le seul lourd de run_e2e.sh (on lit les fils
  donnés par tâche, pas les millisecondes). **La mesure du petit masque est
  faite** (banc, ici, sur b808a3bee, page 12 §8 du cœur `a452a0054`) :
  67 584 lignes lues sous le masque à 1 fil comme à 32, 5,72 ms contre
  5,25 ms : les fils ne changent rien au balayage, et le balayage est
  petit ; donc les 360 ms d'`usages` sur l'index réel (F) sont ailleurs
  (l'ordonnancement des tâches × fils, plusieurs pipelines par instruction,
  le plein texte) : le cœur lit sa trace par tâche sur la requête de F avant
  de choisir la forme, la règle des fils reste en brouillon. Le banc a
  retiré sa mesure et son worktree de luciepc : rien de lui ne tombera sur
  Lucie. **Le push des défenses** a eu le verrou à 0 h 02 ; les e2e ont
  refusé la lib d'ici, plus vieille que le visited set (`3e0f98f08`, dans
  l'extension vector) : relancé avec `RAG3WEAVER_MOTEUR_ANCIEN=1` déclaré
  (lot produit seul), verrou tenu depuis 0 h 06. **Le rebâti de la lib
  d'ici attend le matin, à mon signal** : la mémoire bâtit le binaire de
  Lucie à minuit contre la lib actuelle ; une lib neuve derrière obligerait
  à le rebâtir une seconde fois avant le redémarrage suivant ; au matin, les
  « fini », le rebâti par l'arbre principal (ABI dite par nm), puis le
  binaire de Lucie dans la foulée, annoncés ensemble.
- **0 h 10 – 0 h 40, nuit**. **Sur master** : les trois lots de l'arbre
  principal (`1b8d48dc0` le délai d'embarquement, 60 s au manifeste, lot
  expiré rejoué morceau par morceau, muet nommé et exclu ; `ab12bd4c2` les
  défenses contre la boucle, le compte des morceaux sans texte dit au
  journal, dans l'état et en tête des lectures ; `e5c53d2e2` le reçu occupé
  qui porte les montres) ; le lot A de G (`068356677`, l'embarqueur absent
  refusé aux quatre portes, rebâti du moteur et rejeu sous le verrou, 20 min,
  la fenêtre a marché) ; le poste sous un poste corrigé (G, `ced1f2722`,
  onze cas, déployé md5 aef06aae : le poste remonte ses parents et passe
  sous un tenant, sans variable) ; la section ELF du moteur relue par le
  cœur (`f5fdc7e9a`) et le refus nommé binaire ↔ lib avant le chargeur
  (`f3d2290ad`, `rag3db-check-lib`, ticket du cœur fermé) ; le magasin de
  libs 3a (`7927672f8`, `scripts/rag3db_libs.py`, les consommateurs 3b après
  le rebâti de la lib) ; la règle « au repos = journal soldé » dans la page
  du démon (`d96efde9b`). **L'hôte de Lucie** : redémarré une première fois
  à 0 h 00 sans les défenses (la mémoire guettait l'ancien hash, rebasé ; et
  son contrôle de repos au CPU seul a coupé un rattrapage qui attendait le
  service), puis **à 0 h 26 sur le binaire avec les défenses** (md5
  3e95f0302023, eabc53e86), arrêt propre en 17 s, jugé comme la boucle (25
  min sur 2 096 morceaux, journal muet, un cœur, .wal régulier) ; depuis, il
  reconstruit son index vectoriel (coupé par l'arrêt, le prix du remède) et
  répond, lentement ; le rattrapage viendra ensuite avec les défenses ; la
  mémoire regarde vers 1 h. Manque trouvé : l'état des index n'est pas
  exposé par l'hôte (seulement par le binaire autonome) : petit lot à
  l'arbre principal, propriétaire des routes. **La trace par tâche du cœur**
  (mesure d'impact de F, 3 620 tâches, 3 038 ms) : 91 % du temps dans les
  tâches dotées de 32 fils, dont 63 % à cardinalité estimée ≤ 2 048 et 37 %
  sans estimation (le balayage factice sous `UNWIND $uuids`, remarque de
  F) ; les fils proportionnés couvrent la quasi-totalité des lectures
  d'outil, « moins de tâches » n'est pas le levier. Règle approuvée (celle
  de PostgreSQL) : un fil par 2 048 lignes estimées, inconnu = plafond,
  l'ingestion à 32, `threads=1` explicite jamais défait ; avant/après sur la
  même trace, parité par les e2e et le banc ×10. Cause profonde trouvée en
  l'écrivant : l'optimiseur ne remettait les cardinalités à jour que pour
  EXPLAIN LOGICAL, la jointure née de la descente des filtres n'en avait
  aucune ; le remetteur tourne après toute optimisation. Le push ABI du
  cœur (verrou depuis 0 h 34) a été retardé par un rouge connu du banc passé
  vert par hasard (fuite probabiliste sous sa borne). **luciepc** : toujours
  figé et gardé ; la mesure de hotfixs attend aussi derrière le client de
  codeparsers ; personne n'y touche ; le bâti du binaire prêt de G attend là
  aussi. Le knowledge dump 06 est à jour pour le réveil (`c50d7b454`).
- **0 h 40 – 1 h 15, nuit**. **B4 est sur master** (G, `6e3561268`, identités
  relatives : une racine locale par base, la seconde nommée, chemins en « / »,
  ticket fermé, relectures après coup par l'arbre principal et codeparsers) :
  **les quatre briques de la priorité de Lucie sont là** (montre, routes,
  code-tree, B4). Conséquence à savoir : **toute base de code existante est
  « d'avant la règle »** ; un binaire ≥ 6e3561268 refuse sa synchro, ses
  rééditions et son ingestion en le disant (« à réindexer : effacer la base,
  puis relancer l'indexation », warning dans index_state) ; code-principal
  et code-lourd sur luciepc, code-rag3db ici : à effacer et réindexer ; la
  mémoire longue n'est pas une base de code, pas touchée. **Ordre du matin**
  accordé avec la mémoire et codeparsers : le rebâti de la lib d'ici par
  l'arbre principal à mon signal (visited set, section ELF, « ABI inchangée,
  section ajoutée ») ; le binaire de Lucie ≥ B4 prouvé à l'écart, installé
  seulement au moment de l'arrêt (sinon un client neuf de Lucie serait
  refusé) ; l'arrêt propre de l'hôte au journal soldé ; code-rag3db effacé
  et réindexé par codeparsers avec ce binaire en autonome (~12 min),
  annoncé ; le redémarrage ; puis la recherche publie la lib au magasin, G
  pose l'ExecStartPre, lot 3c. Sur luciepc : « effacer puis réindexer » dans
  la section « Pour Lucie » (`ba63aa135` : ce qui attend derrière le client
  de codeparsers). **Lot de nuit à codeparsers** : le témoin de bout en bout
  « une session voit sa propre modification », ici, en autonome (hôte à lui
  sur un autre port, bases sous ~/.cache, binaire ≥ B4, code-tree add, index,
  édition d'un symbole, temps par phase jusqu'à ce que usages et search_code
  le voient, code-tree remove) ; un rouge serait le résultat de la nuit.
  **Sur master aussi** : le nom qualifié (`bc51b0aec`, premier manque de la
  page 04 fermé) ; l'état des index par backend dans /etat et daemon_status
  (`318e37776`, /etat ne fait plus la queue derrière un appel long) ; les
  consommateurs du magasin (`f846db8e6`, `RAG3DB_LIB=courant`) ; la page 06
  de F et sa suite de mesure (`dc4919150` : usages 367-399 ms, la forme en
  deux temps 57 ms, mêmes lignes ; et **les 26 092 morceaux vides portent
  tous le hash d'un vrai texte : le texte est perdu à l'écriture**, preuve
  pour le ticket P1). **L'hôte de Lucie est sain** à 1 h 01 (usages 0,51 s,
  search_code 0,84 s, au repos, reconstruction finie sans ligne au journal :
  manque noté, couvert par /etat au matin). **Décision page 06** : ni
  attendre le semi-masque ni poser tout de suite : remesurer usages dès que
  le lot des fils du cœur est sur master ; si la médiane reste > 100 ms,
  poser la forme en deux temps comme chaînage générique de l'IR, ticket du
  semi-masque gardé, forme retirée quand le moteur n'en a plus besoin
  (renversable par Lucie). Le cœur : push ABI (verrou depuis 1 h 11, après
  un rouge connu du banc passé vert par hasard, remis aux probabilistes
  `94a09d84e`), le lot des fils en bâti (fils.sh), la passe du produit sur
  sa branche par l'arbre principal.
- **1 h 15 – 1 h 45, nuit**. **Le lot ABI du cœur est sur master**
  (`2bc7b4a3b` : la signature d'avant B2 du constructeur de SystemConfig reste
  en surcharge qui délègue, témoin par dlsym ; un binaire d'avant B2 charge la
  lib neuve sans rebâti ; hotfixs a le hash pour la lib de luciepc après
  l'arrêt de Lucie). **La passe du produit sur le lot des fils est verte**
  (arbre principal : 131 tests en 12 groupes, 0 rouge, aucune attente,
  e2e_code 160 s contre 198 ; artefact prouvé par ldd et md5) : le cœur
  pousse les fils proportionnés. Le rebâti de la lib d'ici se fera en une
  fois après ce push (visited set, section ELF, ABI, fils). **Le témoin de
  bout en bout tourne** (codeparsers, ici, isolé : hôte à lui sur 7975 sous
  ~/.cache/codeparsers-temoin, worktree détaché de 8610eaec6 avec les quatre
  briques et l'ABI, binaire bâti contre la lib d'ici, moteur ancien déclaré ;
  code-tree add, index entier, `name_of_tree` renommé sur disque, le temps
  jusqu'à ce qu'usages et search_code le voient, en-tête des morceaux sans
  texte, code-tree remove). La section « Pour Lucie » est à jour pour B4
  (`8610eaec6` : les bases de luciepc mises de côté dans avant-b4/ après la
  copie de comptage, le binaire neuf, la réindexation à heure accordée).
  Deux mesures attendent la place d'ici derrière le témoin (F, le banc).
- **1 h 45 – 2 h 15, nuit : la priorité de Lucie est prouvée de bout en
  bout.** Témoin **vert** de codeparsers (`93090770a`, en tête de la page des
  manques), ici, isolé, master 8610eaec6 : code-tree add 1,2 s ; index
  entier 22 min 50 s en régime doux (102 469 Scope_Chunk, 6 303 File_Chunk) ;
  `name_of_tree` renommé sur disque → `usages` voit le nouveau nom en
  **7,26 s**, `search_code` en **7,67 s**, l'ancienne définition partie ;
  code-tree remove et arrêt 4 s, propres ; **0 morceau sans texte**. Dans
  les 7,3 s : trois appels rendent l'ancien état, puis un appel attend 5,7 s
  derrière la réindexation du fichier qui tient le catalogue : c'est là que
  se gagnera la fraîcheur sous la seconde (proto, mesure par phase au
  matin ; manque : les phases ne se lisent nulle part, une ligne [montre] par
  fichier au journal de l'hôte ajoutée à son lot). **Le lot des fils du
  cœur, mesuré** (page 12 §9) : impact 2,1 à 2,5 × plus court (joindre 55 →
  27 ms, est_local 80 → 32), execute 2,04 → 0,90 ms par instruction,
  préparation 0,78 → 0,41 (les 32 fils se disputaient le poste avec elle),
  temps cumulé des tâches 54,4 → 27,1 s, résultats identiques ; banc ×10 et
  131 e2e du produit verts ; push en cours (verrou depuis 2 h 02). Après ce
  push : le rebâti de la lib d'ici en une fois, puis F remesure usages
  (règle des 100 ms). Hypothèse retirée : « QUERY_VECTOR_INDEX sans
  estimation » (hotfixs a mesuré : l'estimation existe, fausse à 1 au lieu
  de k ; les card=? de F sont d'autres CALL) ; les deux corrections courtes
  (k, dimension du semi-masque sur la table) faites quand même. **Cause du
  P1 resserrée** (`a7a2374d2`, `ff2f8a5e7`) : un index entier neuf donne 0
  vide ; sur la base de Lucie (F), **le vide se fait par scope entier**
  (14 724 scopes tous vides, 65 442 aucun, 1 232 mêlés), tout présent sauf
  `_text` (hash du texte, hash et vecteur d'embarquement, bornes) : le texte
  existait à l'embarquement puis la ligne a été réécrite sans lui ; petits
  scopes (rang moyen 4,7 contre 51,6) ; lecture « chemin » (réingestion par
  la montre, redécoupage, synchro) plutôt que « binaire » ; au proto de
  relire la réécriture d'un Scope_Chunk dans les deux dialectes (témoin :
  réingérer un fichier inchangé, 0 vide attendu), à l'arbre principal la
  sonde du matin. Lots donnés : banc, le témoin de l'index vectoriel
  reconstruit sous un lecteur en face (correctif de hotfixs) ; hotfixs, les
  deux corrections de QUERY_VECTOR_INDEX après son témoin du point de
  reprise.
- **2 h 15 – 2 h 50, nuit**. **Le lot des fils est sur master** (cœur,
  `85f4f7f9c` l'instrument, `6ffec5a82` la règle : un fil par 2 048 lignes
  estimées, inconnu = plafond, le balayage factice sous UNWIND compté, les
  cardinalités remises à jour après toute optimisation ; `dda63783a` deux
  tickets fermés ; page 12 §9 `dbb3c71a8`). **La lib d'ici est rebâtie à
  2 h 34 sur 7b54c5f84** (arbre principal : md5 librag3db 116acb25…,
  libvector 0dc728b8… ; 4 755 → 4 757 symboles, ABI inchangée, section
  `.rag3db_build_id` = rag3db-build-6ffec5a82… ; nouvel inode, l'hôte de
  Lucie garde l'ancienne image). Réserve écrite au ticket ABI : la surcharge
  fait se charger un binaire d'avant B2 mais pas tourner (l'ancien binaire
  de Lucie plante en malloc : la disposition de SystemConfig a changé) ;
  « un changement de disposition d'un type exporté est un changement d'ABI,
  le refus nommé fait foi ». Derrière l'annonce : F remesure usages (règle
  des 100 ms), la recherche publie au magasin (lot 3c écrit : 97 sites de
  82 suites par `common::vector_extension()`), la mémoire bâtit le binaire
  de Lucie ≥ B4 sans l'installer. **La réécriture des morceaux sans texte
  est sur master** (`7b54c5f84`, depuis leur entité) : elle répare n'importe
  quelle base. **La cause du P1 bascule vers le moteur** : la sonde
  « binaire » de l'arbre principal rend 0 vide sur 1 989 avec le binaire
  neuf sur vingt fichiers dont dix tout vides chez Lucie, mais la relecture
  du code de 13 h 20 (`5422fa937`) montre que `_text` passait déjà par COPY
  ou paramètres liés, jamais par du Cypher brut : l'hypothèse « binaire »
  tombe, dite ; le texte était en base à la passe de vecteurs (run_index le
  relit) et se perd **après** (SET de réclamation, embed_set, point de
  reprise, rejeu), par plages qui ressemblent à des groupes de nœuds
  (fichiers entiers + partiels aux frontières). F vérifie l'alignement sur
  2 048 ; le banc écrit le témoin moteur (COPY de plusieurs groupes, SET
  colonne par colonne, points de reprise, arrêt propre et rejeu, deux
  modes) ; le cœur lit le code de l'UPDATE d'un groupe. **Et le SIGSEGV de F
  se reproduit en une seconde** sur la copie : `MATCH (s:Scope) RETURN
  s.content` suffit, une chaîne annoncée de 1 944 551 octets dont le pointeur
  tombe hors de la région des tampons (décalage de dictionnaire abîmé) :
  une colonne de chaînes abîmée dans la même base, peut-être la même cause ;
  le cœur cherche (mémoire ou disque), gdb en file. Refus tenu : l'arbre
  principal ne pouvait pas retirer sa propre file de push, je ne l'ai pas
  fait à sa place (ordre : codeparsers, lui, puis le rebâti : une demi-heure
  de plus). Hotfixs rappelé à la règle : `mesure` pour la mesure seule, le
  bâti sous `lourd` (sa mesure qvi tenait l'exclusif pendant un cmake, quatre
  travaux derrière).
- **2 h 50 – 3 h 05, nuit : la cause du P1 est trouvée, et les textes
  n'ont jamais été perdus.** Le cœur, sur la copie de F : `MATCH (s:Scope)
  RETURN s._uuid, s.content` relu ligne à ligne rend pour 5 lignes sur
  81 407 **la chaîne d'une autre ligne** (celle 2 048 plus tôt, même position
  de fenêtre) ; la relecture par clé est juste ; le SIGSEGV n'est que le cas
  où le bloc de la chaîne périmée a été rendu au système. Dans le code :
  `StringColumn::scanFiltered` (string_column.cpp:240), chemin pris quand le
  groupe porte des versions (lignes supprimées ou écrites sous transaction),
  ignore `numValuesToScan` et borne par la fin du segment ; quand la fenêtre
  de 2 048 chevauche deux segments d'un groupe (COPY puis points de reprise
  qui complètent), les positions de fin de fenêtre ne sont pas écrites dans
  le vecteur, qui garde la valeur d'avant ou rien (« vide »). Trois
  conditions : colonne STRING, sélection filtrée, fenêtre à cheval. Ça
  explique tout : les « vides » par plages dans les seize premiers groupes
  (écrits puis complétés), 0 sur un index neuf (un segment, pas de
  versions), le texte présent au vecteur, **et la boucle de luciepc**
  (run_index relisait `_text` vide par ce balayage, sautait, reprenait).
  Correctif en cours (cœur, branche scan-filtre-chaines, témoin gtest,
  amont ou nous au blame) ; le banc oriente son témoin exactement là (un
  groupe en deux segments + une sélection filtrée, balayage contre relecture
  par clé) ; l'arbre principal rebâtira la lib à ce hash ; F recomptera sa
  copie sans rien réécrire (0 attendu) et dit « lu vide », pas « perdu ».
  Conséquences : **la réécriture des morceaux vides est suspendue par
  défaut** (elle relit `content` par le même balayage ; journal avec le
  compte et la raison, `RAG3WEAVER_REECRIRE_LES_VIDES=1` pour forcer, un
  commit d'une ligne après la lib corrigée ; pas de garde par hash, un hash
  ne s'ordonne pas : si garde un jour, une fonction interrogeable du moteur) ;
  les défenses et « complet compte `_text` » restent (ticket `11c585c48`) ;
  le témoin de réingestion du proto n'est plus nécessaire ; la remesure
  d'usages de F après les fils reste à 382-389 ms (le semi-masque est le
  coût, les fils ne l'atteignent pas) → **la forme en deux temps est posée**
  par F (54 ms contre 109, mêmes lignes, chaînage générique de l'IR). **La
  séquence B4 attend Lucie** : le classifieur de la mémoire a refusé l'arrêt
  TERM de son hôte à 2 h 56 (il l'avait permis deux fois) ; rien exécuté,
  l'hôte tourne et répond (binaire 3e95f0302023, lib de 2 h 34, ABI
  inchangée) ; une seule séquence au matin, par sa main, avec le correctif
  du balayage dedans : rapport de la mémoire `419b87a36` (commande exacte,
  pid lu dans daemon.json). Codeparsers refuse de réindexer tant que l'hôte
  répond. **Sur master** : le lot QUERY_VECTOR_INDEX de hotfixs
  (`db1044912` : estimation = k, semi-masque sur la table, cardinalité des
  appels de fonction de table) ; le magasin de libs fini (recherche,
  `5e08ca07b`, 3c : 97 sites de 82 suites, prouvé de bout en bout, publié ;
  puis `80f4f47ae` le magasin par défaut dans run_e2e.sh) ; la section
  « Pour Lucie » (`5811a4a2a`, `776602468`). Décisions pour la recherche
  (renversables) : magasin par défaut si le rebâti publie lui-même ; RUNPATH
  vers le dossier d'un identifiant **pour les binaires installés seulement**
  (la mémoire l'adopte pour le binaire du matin) ; puis le premier vrai flux
  (la sortie du modèle en jetons), forme à me dire avant le code.
  **Deux incidents de forme, sans dégât** : (1) vers 3 h, les `poste push` en
  attente de codeparsers (trois), de F (2 h 57) et de la recherche (3 h 01)
  sont morts en 143 : **le témoin test_poste.sh de G**, joué ici, terminait
  par motif les `flock … master-push.lock` (cas 5) et `flock -x 8` (cas 6),
  six passages entre 2 h 57 et 3 h 08 ; G l'a dit, corrigé (pids propres),
  règle ajoutée : un témoin joue sur ses propres fichiers de verrou, jamais
  les réels ; tous les autres ont répondu « pas moi » avec leurs pids ; rien
  de perdu, morts avant tout git push ; (2) ~/.cache/rag3weaver-build/wt-docs
  était partagé : l'arbre principal l'a remis sur FETCH_HEAD à 2 h 57 et a
  abandonné un commit non poussé de la mémoire (repris par cherry-pick,
  `419b87a36`) ; règle : un worktree de docs par session, `git status` et
  `git log -1` avant tout checkout ; wt-docs reste à l'arbre principal.
  Lot poste-tenants de G en cours (le verrou de push tenu par le script, une
  attente annulable, un flock mort qui arrête, `--qui` qui nomme le tenant,
  19 cas, + le témoin « deux attendants survivent au tenant qui finit »).
- **3 h 05 – 3 h 25, nuit**. **Le correctif du balayage filtré est écrit**
  (cœur, branche scan-filtre-chaines `6bcf4dbae`, chaînes **et listes** :
  même borne fausse dans ListColumn::scanFiltered ; suites vertes, parité
  0 écart sur la copie de F, les tickets du SIGSEGV et des morceaux sans
  texte fermés dans le lot) ; le témoin exact du banc (`52b042632`, un groupe
  en deux segments, second segment court ou long, sélection filtrée, cinq
  cas dont mort et rejeu) est repris par le cœur par cherry-pick, joué rouge
  puis vert, puis push ; les témoins P1 du banc (MERGE…SET, passe de
  vecteurs, premier COPY avec mort et rejeu) sont tous verts en lecture non
  filtrée : la cause est confirmée par l'absence ; une version courte reste
  au banc pour garder la réécriture des chaînes, les longues dans ses notes.
  Sur master : « complet compte `_text` » et la suspension de la réécriture
  (arbre principal, `772d4e9d6`, `1a31fea71`, rejeu pris sur la lib du
  magasin par défaut) ; **le magasin par défaut** (recherche, `02d67d191` :
  run_e2e.sh lit le courant et le dit, `RAG3DB_LIB=bati|<id>`,
  `RAG3WEAVER_MOTEUR_DU_WORKTREE=1` pour un worktree du moteur, le rebâti
  publie et pose courant : plus de « fini » pour un rebâti, seuls les hôtes
  et binaires installés restent concernés ; diffusé aux neuf sessions) ;
  **les tenants du push** (G, `a94409139`, déployé md5 0229e1ad : le verrou
  tenu par le script et jamais rendu par un signal, la commande jamais
  tuée, attente annulable, `--qui` nomme le tenant, `--lourd` refusé
  rappelle le rejeu distant, test_poste 21 cas sur ses propres pids et
  verrous avec garde contre les réels, l'incident de 3 h écrit) ; la sonde
  des morceaux sans texte (codeparsers, `d2137b70e`, poussée par un script
  orphelin dont l'enveloppe avait été tuée : un trou du poste, l'enveloppe
  doit propager le signal, dit à G). **La forme du premier vrai flux**
  (recherche, `4aa8b683a`, `docs/12-octobre-2026-recherche/01` : LlmNode
  émet `tokens` Finite optionnel à côté de `text` et `llm`, un seul puits
  émet et accumule, `stop_when_unread`, `has_streams` sur les flux branchés
  seulement = parité par construction, Decide sur le texte entier ;
  consommateurs TokenEvents et Watch chez le proto ; témoins et mesure)
  approuvée, le mot du proto sur ses deux consommateurs avant le code.
  G passe à B1-B3, C, les archives alpha.3 sans publier ; sa recette luciepc
  passe au RUNPATH vers l'identifiant et l'ExecStartPre au premier rebâti
  de hotfixs.
- **3 h 25 – 3 h 55, nuit : le correctif du P1 est sur master et la lib le
  porte.** Cœur : `e6f626df4` (`f6179ade0` le correctif de
  StringColumn::scanFiltered et la borne par forme de ListColumn ; le témoin
  du banc `Windows/StringScanFiltered.*`, réécrit sur la recette minimale du
  cœur : 6 000 lignes à chaînes de 200 octets, cinq segments de ~1 250 lignes
  au point de reprise, une seule ligne supprimée dans la fenêtre → 801/886/800
  lignes fausses sans correctif, 0 avec) ; tickets du SIGSEGV et des morceaux
  sans texte fermés dans le lot ; banc 8 rouges attendus / 264 verts. **Lib
  d'ici rebâtie à 3 h 49** sur e6f626df4 (arbre principal : ABI inchangée,
  4 757 symboles ; publiée au magasin et posée en courant, identifiant
  `rag3db-build-f6179ade0…-Release-p12-v11-g17-s18…` ; md5 librag3db
  c2ddbb67…, libvector 545d634c…) ; les passes en cours gardent leur
  identifiant. Suite dans l'ordre : la mémoire bâtit le binaire de Lucie avec
  RUNPATH vers cet identifiant et met à jour la commande qui l'attend ; F
  recompte sa copie scellée (0 vide attendu) ; l'arbre principal remet la
  réécriture des vides en marche (une ligne) ; hotfixs rebâtit la lib de
  luciepc ≥ e6f626df4 après l'arrêt de l'hôte par Lucie, G son binaire prêt
  dessus ; codeparsers l'écrit dans « Pour Lucie » (aucune réindexation de
  luciepc avant). Banc : le témoin rouge du DROP/CREATE de l'index vectoriel
  sous des recherches (`6ae69c6b1`, known_red, « vector::reserve » en plus,
  ticket à hotfixs) et la version courte de la réécriture des chaînes
  (`79431d9c4`, vert, 1,5-1,9 s). Recherche : sa part du premier flux
  (`38a95fa9b` : flux branchés seuls, flux non branché jeté sans fusion,
  parité par construction dans le témoin) ; le proto porte `LlmNode.tokens`.
  F : la forme en deux temps écrite (branche saut-en-deux-temps, capacité
  `two_step_hops` vraie pour rag3db, fausse pour PostgreSQL) : usages 360 →
  218-241 ms, même rendu, encore > 100 ms (le balayage sous masque reste,
  lot suivant du cœur, page 12 §10 : lecture par décalages sous un petit
  masque par `lookupMultiple`, décidée sur la cardinalité exacte du masque à
  l'exécution) ; F remesure sur la lib corrigée avant de fusionner.
- **3 h 55 – 4 h 25, nuit : les « 6 secondes » expliquées, le P1 clos.**
  Mesure par phase du proto (index de src/, 180 fichiers, 15 631 morceaux) :
  réindexer un fichier 2,5 à 3,9 s dont **0,47 s d'ingestion** ; le reste
  est une seule instruction, le DETACH DELETE des morceaux du scope changé :
  **2,1 à 2,9 s pour une seule ligne qui porte un vecteur sous index HNSW**
  (11 ms sans vecteur ; retirer 45 morceaux : 112 s contre 259 ms) ; et à
  l'index initial 309 s sur 329 à écrire 15 631 vecteurs (**~20 ms par
  vecteur**). Contournement approuvé, côté dialecte et rag3db seulement
  (SchemaDialect::vector_columns_to_clear / clear_vector_by_parent : NULL
  sur les colonnes vecteur par lot avant le DETACH DELETE) : réindexer un
  fichier passe à **0,73-1,03 s** ; le défaut moteur part en ticket à
  hotfixs (l'extension vector), les témoins avant le code au banc
  (suppression sous HNSW avec et sans index, insertion par lot à 1/8/32
  fils). Le rendez-vous par les symboles de l'arbre principal attend la
  mesure finale ; il prend les manques de la page 04 dans son ordre (tests
  par requêtes suivis, portée, verrous du chemin). **P1 clos** : la
  réécriture des vides remise en marche (`2b0642b1e`), le ticket fermé avec
  les suites produit (`961dd7b05`), et **le recomptage de F sur la copie
  scellée avec la lib corrigée : 0 vide sur 128 732**, rien réécrit ; reste
  la base vivante de Lucie après le redémarrage du matin (0 attendu). Le
  banc : témoin du balayage prouvé des deux côtés, garde des grosses listes
  (`410092032`), règle « pas de rouge annoncé sans l'avoir joué » (son
  premier témoin ne touchait pas le défaut). **Le binaire du matin de Lucie
  est prouvé** (mémoire, `~/.cache/rag3weaver-build/binaire-matin-d2b83e5e5080`,
  md5 d2b83e5e5080…, master acdb91764 ≥ e6f626df4, RUNPATH vers l'identifiant
  f6179ade… du magasin, sans LD_LIBRARY_PATH : readelf, ldd, check-lib « même
  bâti », --describe, hôte d'essai, client de boutique ; rapport `4ac16805d`
  avec la commande d'arrêt pour Lucie) ; codeparsers a le triplet. **G** : B1
  sur master (`617839ede`, libc/windows-sys par cible), B2 relu par F avec un
  test de plus, B3 relu par le cœur ; sur luciepc, bâti complet dans son
  worktree à acdb91764 : lib et extension publiées au magasin de luciepc
  (créé), binaire et rag3db-check-lib liés par RUNPATH à cet identifiant,
  tout en `.pret`, ExecStartPre préparé inactif, derrière le client de
  codeparsers ; **décidé avec hotfixs : le bâti de G au magasin fait foi
  pour l'hôte de luciepc**, rag3db-lourd ne se rebâtit en place que si
  codeparsers nomme un outil qui le lie encore.
- **4 h 25 – 4 h 55, nuit**. **Le lot des « 6 secondes » est sur master**
  (proto, `30f1f7949` + `df1184d32`, fenêtre accordée par le verrou tenu
  pendant son rejeu après deux courses perdues ; puis `5fc38460d` les
  corrections de relecture de la recherche) : la montre laisse passer une
  lecture pendant une réaction (lock_for_read), le reçu occupé avec montres
  et reprise (ticket fermé), la ligne [montre] par fichier avec ses phases,
  la mesure par phase, Decide et max_tokens ; **le contournement de la
  suppression HNSW** (vecteur remis à NULL avant le DETACH DELETE, rag3db
  seulement) : réindexer un fichier **2,5-3,9 s → 0,56-0,68 s**, retirer un
  fichier de 32 scopes **96-114 s → 0,40 s**, invariant des vecteurs 8/8 ;
  ticket moteur à hotfixs. Codeparsers rejoue le témoin de bout en bout sur
  cette tête (hôte isolé, binaire RUNPATH, LD_LIBRARY_PATH vide) : on attend
  les 7,3 s sous 2 s. Le banc ne reproduit pas les 2,5 s par ligne au moteur
  nu (DELETE d'une ligne indexée 20-50 ms, indépendant de la taille de
  l'index, 40 × la voie NULL puis DELETE ; SET depuis NULL 1-4 ms par
  vecteur, aucun gain par les fils, coût qui croît avec les SET depuis le
  dernier point de reprise ; CREATE 15 000 vecteurs 9-12 s) : variante fidèle
  au produit en bâti (COPY sans vecteur, relation par morceau, SET par 64,
  aucun point de reprise, DELETE filtré sur une colonne non clé), aucune
  couleur annoncée avant de l'avoir jouée. **Le saut en deux temps de F est
  sur master** (`6f5554868`, capacité `two_step_hops`, Warning du catalogue
  pour les lignes écartées) : usages 355 → 212-239 ms, empreintes
  identiques ; recomptage sur la lib corrigée : 0 lu vide, 11 lectures, aucun
  SIGSEGV ; F remesure sur le balayage sous masque du cœur (en passe : 117
  lignes lues au lieu de 67 584 sur son témoin) et retire la forme si elle ne
  gagne plus. **G** : B3 (`71cdf4185`, relu par le cœur) : **le lot B entier
  est sur master**, paquet-npm ne portera plus que le paquet ; **le lot C
  (démon portable) écrit** sur portage-c : un module par système pour l'état
  d'un processus (Linux /proc, macOS proc_pidinfo, Windows OpenProcess…),
  « inconnu » laissé à /sante, daemon.json 0600 et ACL, détachement par
  groupe de processus, arrêt propre par console sous Windows ; vert sous
  Linux ; runners Windows et macOS sur une branche d'essai ci-portage-c, **à
  effacer du distant dès qu'ils ont parlé** ; relectures mémoire et arbre
  principal avant push. Le worktree de luciepc de G avancé à la tête pour un
  seul bâti (lib, magasin, binaire, check-lib), derrière le client de
  codeparsers. **Binaire du matin de Lucie à rebâtir une fois de plus** sur
  la tête vers 5 h 30 (mémoire, même lib f6179ade…, même RUNPATH, pas
  installé : il porte alors le saut en deux temps et les 6 s) ; G de même
  pour le binaire seul de luciepc. L'arbre principal : conception du manque
  « tests par requêtes » approuvée (`workspace.queries_via` au manifeste :
  tests par fichier par motif, noms enregistrés par expression, table
  mots-clés → symboles dans le gabarit du dépôt, datée et comptée ; relation
  QUERIES suivie par impact avec la certitude « par requête » ; « non suivis
  (N fichiers de test sans lien) » tant qu'un dépôt ne déclare rien ; un
  seul poseur de marques avec codeparsers). La mémoire joue la tentative 3
  des fiches classées (Gemini, max_tokens de Decide).
- **4 h 55 – 5 h 30, nuit**. Le témoin de bout en bout rejoué sur df1184d32
  (codeparsers, `d447364ce`, base entière de 102 469 morceaux, 0 vide,
  binaire RUNPATH) : **7,13 s** pour usages, 7,53 s pour search_code, pas de
  gain à cette taille ; mais la ligne [montre] découpe désormais le temps :
  verrou attendu 0 ms (la lecture ne bloque plus), ~2,1 s avant la première
  réaction (every_ms 1 000 + settle 300), puis **deux fichiers l'un après
  l'autre**, 2,2 et 2,7 s, dont « ingérer » **1,9 s chacun contre 0,5 s à
  15 k morceaux** : ça croît avec la base, c'est un balayage. File du proto
  réordonnée, la fraîcheur étant la priorité de Lucie : (1) le découpage
  d'« ingérer » par instruction (nommer le balayage ; le lot du masque du
  cœur se mesurera dessus ; rendu aussi à l'arbre principal pour son
  rendez-vous par les symboles) ; (2) LlmNode.tokens (la recherche l'attend) ;
  (3) reingest_files par lot de fichiers vus ensemble (une ingestion et une
  résolution pour le lot, la lecture entre deux lots : le vrai gain à cette
  taille) ; (4) Decide.labels_from pour la tentative 4 des fiches. Codeparsers
  garde son script pour un troisième rejeu. Binaire du matin de Lucie à
  rebâtir vers 5 h 30 par la mémoire sur la tête (≥ 5fc38460d).
- **5 h 30 – 6 h, nuit**. **Le balayage sous petit masque est sur master**
  (cœur, `41d282843` : mode décalages quand le masque retient moins d'une
  ligne sur 64, décidé sur sa cardinalité exacte ; tranches de ≤ 2 048
  décalages relues par lookup ; 117 lignes sorties pour 117 masquées sur
  72 000 contre 67 584 ; 0,26 ms contre 5 ms ; ticket de F fermé, le témoin
  du banc sorti du known_red ; deux passes complètes dont LDBC) : le
  planificateur est au point voulu pour l'IDE. **Programme du cœur vide →
  prochain lot : le coût d'écriture des vecteurs sous HNSW** (309 s sur 329
  d'un index entier à écrire 15 631 vecteurs, ~20 ms chacun dans le produit ;
  1-4 ms au banc nu ; 0,6-0,8 ms au CREATE) : pourquoi l'insertion
  incrémentale coûte 2 à 5 × le CREATE et 20 × dans le produit ; forme
  attendue : une insertion par lot au coût du CREATE ou proche (lecture de
  pgvector) ; avec hotfixs (lui le DELETE, 20-50 ms par ligne indexée), un
  seul instrument et un seul témoin au banc ; mesure avant/après sur l'index
  entier de codeparsers (1 384 s) ; ensuite I2 et le plein texte tenu par le
  point de reprise. **La lib d'ici se rebâtit sur 41d282843** par l'arbre
  principal dès que son push des tests par requêtes (`665636354`, sous le
  verrou) a rendu : ses `cargo test --lib` lisent encore `build/lecteurs-csv`
  directement (reste 3d du magasin pour la recherche : le pont et
  check_features résolvent `RAG3DB_LIB` comme run_e2e.sh). La mémoire a
  arrêté son bâti de 5 h 30 (visait f6179ade…) et bâtira le binaire de Lucie
  avec RUNPATH vers le nouvel identifiant à l'annonce ; G avance son worktree
  de luciepc à ≥ 41d282843 pour un seul bâti. Tentative 3 des fiches
  classées (mémoire, `01dd9ddb0`, Gemini 3.5 Flash sur Decide à max_tokens).
