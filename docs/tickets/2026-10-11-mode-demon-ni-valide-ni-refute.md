# Le mode `--demon` du serveur MCP n'est ni validé ni réfuté — trois questions ouvertes

- **État** : ouvert, à trancher au matin ; témoin sur la branche `memoire-temoin-demon-2` (`fee32ed40`)
- **Gravité** : inconnue, et c'est le sujet. Deux des trois questions peuvent être un défaut du produit, une est sûrement un défaut de mon témoin
- **Atteignable en service** : seulement par `rag3weaver-backend mcp --manifest <m> --demon <adresse>`, qui n'est utilisé par personne aujourd'hui
- **Touche rag3weaver** : oui — c'est la forme prévue pour une mémoire partagée entre sessions

## Ce que c'est

Le serveur MCP a deux modes (`templates/mcp/README.md`). Sans `--demon`, il
**possède** sa base — bon pour un dépôt de code, une session. Avec `--demon`, il
ne possède plus rien : son backend reste local et sa **connexion** passe par
`rag3daemon`, le processus unique qui tient la base. C'est la forme prévue pour
qu'une mémoire soit partagée entre plusieurs sessions Claude Code, puisque le
client lance **un processus MCP par session** et qu'une base rag3db n'a qu'un
écrivain.

**Le trou que j'avais nommé moi-même avant d'écrire une ligne de ce mode** : les
N backends gardent chacun **leur** `Catalog`, donc leur `initialize`, qui repose
les DDL. N ouvertures concurrentes les reposent ensemble. `IF NOT EXISTS` est
partout, mais « partout » est une impression. Et je m'étais fixé la condition :
**si le témoin est rouge, le mode ne tient pas et le pont — relayer
`describe`/`call` vers un hôte unique — redevient la bonne réponse.**

**Cette condition ne s'applique pas**, parce que le rouge obtenu ne parle pas du
produit. D'où ce ticket plutôt qu'une conclusion.

## Les trois questions, dans l'ordre où je les trancherais

**1. `initialize` crée-t-il les tables quand quatre backends s'ouvrent en même
temps par le démon ?** La passe où les quatre ouvertures ont rendu `Ok`
affichait `tables : []`. Deux hypothèses, **les deux ouvertes** :

- ma lecture était fausse (le plus probable) : elle prenait la **première
  colonne** de `CALL SHOW_TABLES()` pour le nom de la table, alors que c'est un
  identifiant, donc elle ne voyait rien ;
- ou `initialize` n'a réellement rien créé, et **c'est un défaut du produit**.

Ce qui les sépare : un `MATCH (n:Note) RETURN count(n)`, qui rend un compte si
la table existe et un refus nommé sinon. Ne dépend d'aucun ordre de colonnes.

**2. Pourquoi une lecture pend-elle après le départ des écrivains ?** La garde
corrigée (le `MATCH` ci-dessus) **pend** — vingt minutes, arrêtée à la main —
sur une **cinquième** connexion au démon, ouverte *après* que les quatre
backends ont été lâchés (les fils sont joints avant la lecture). Un lecteur qui
bloque alors que les écrivains sont partis est soit un `Drop` de `Backend` qui ne
rend pas la main au démon, soit un verrou du démon qui survit à son client.
**Celle-ci serait un défaut du produit, et elle dépasse le mode MCP** : elle
toucherait tout usage de `rag3daemon` par plusieurs clients successifs.

**3. Pourquoi le journal du démon annonce-t-il une autre adresse que celle
demandée ?** Le témoin affiche `127.0.0.1:37273` et le journal du démon lancé
par `DaemonConnection::assurer` dit `127.0.0.1:41969`, alors que
`Attache::adresse` (`src/serveur.rs:225`) rend l'adresse **demandée**. Tant que
cet écart n'est pas compris, **on ne sait pas à quel démon les quatre ouvertures
ont parlé** — et le « 4 réussies » n'est donc pas une preuve.

Vérifié avant de s'inquiéter : **aucun démon étranger n'écoutait sur ce port**,
et aucun n'était vivant après coup. Aucune base d'une autre session n'a été
touchée.

## Le témoin

`extension/rag3weaver/tests/e2e_demon_backends_concurrents.rs`, sur la branche
**`memoire-temoin-demon-2`** (`fee32ed40`), **pas sur master**. Son en-tête porte
le même état que ce ticket. Il monte un `rag3daemon` hermétique
(`Fin::Arreter`), ouvre quatre backends derrière une **barrière** — sans elle le
premier aurait fini de reposer ses DDL avant que le second commence, et le vert
ne prouverait rien —, rend **tous** les échecs et non le premier, puis vérifie
que la table est interrogeable.

Joué sur luciepc, lib et extension du même bâti (10 octobre, 23 h 32), jeu de
features complet.

## Ce qu'il faut pour le fermer

1. répondre à la question 3 (l'adresse), parce qu'elle conditionne la lecture des
   deux autres ;
2. répondre à la question 1 par le `MATCH`, une fois la 2 débloquée ;
3. si la question 2 est un défaut du produit, **elle sort de ce ticket** et
   devient le sien : elle ne concerne pas MCP mais `rag3daemon` ;
4. et alors seulement : `--demon` validé, ou le pont écrit — pour une raison
   mesurée.

## Ce que ça ne bloque pas

Rien d'urgent. Le mode sans `--demon` fonctionne et est éprouvé
(`e2e_mcp_stdio`) ; la recette « une base par session » (`templates/mcp/`) ne
dépend d'aucune de ces trois questions. **Ce ticket ne retient que la mémoire
partagée entre plusieurs sessions**, qui n'a pas d'utilisateur aujourd'hui.

## Et une leçon de méthode, puisqu'elle a coûté trois passes

J'ai composé le manifeste du témoin **de zéro**, en devinant ce qui était
obligatoire : un `schema` d'entité omis, puis une déclaration d'embarquement
omise (`load` l'exige quel que soit le signal), puis la garde fausse. La
procédure qui l'évite n'est pas « lire avant d'écrire » mais **copier un gabarit
livré et en retirer** : un gabarit est la liste exhaustive de ce qui est exigé,
et `src/gabarits.rs` la vérifie à chaque passe de la lib.

Et le défaut le plus coûteux des trois n'était pas une omission : **une garde
fausse qui masquait un vert**. Elle annonçait « l'entité Note doit exister une
fois et une seule » et concluait qu'elle n'existait pas, alors que les quatre
ouvertures avaient abouti. Lue sans la ligne au-dessus, elle m'aurait fait
condamner le mode `--demon` et proposer un chantier de pont. **Un contrôle qui
masque un vert coûte plus qu'un contrôle absent.**
