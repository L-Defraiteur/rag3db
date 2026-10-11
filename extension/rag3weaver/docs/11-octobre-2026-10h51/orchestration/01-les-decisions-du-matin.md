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
