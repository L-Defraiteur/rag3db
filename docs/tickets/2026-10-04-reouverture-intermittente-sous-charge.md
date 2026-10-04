# La réouverture d'une base échoue par intermittence, sous charge seulement

- **État** : ouvert — **quatre hypothèses écartées**
- **Gravité** : plantage (une réouverture qui échoue ; la base n'est pas perdue)
- **Atteignable en service** : **non établi** — vu uniquement sous charge de
  batterie, jamais au repos
- **Touche rag3weaver** : oui (le test est un test rag3weaver ; la lecture qui
  échoue est celle du moteur)

## Ce que c'est

À la réouverture d'une base fermée proprement, le moteur refuse de lire le
journal d'écriture anticipée — `numBytesRead: 0` sur un `read` de 4 096 octets à
une position non nulle — et seulement quand beaucoup d'autres suites tournent en
même temps.

```
reopen DB: IO exception: Cannot read from file: /tmp/.tmp0qwdKX/persist_kb.db.wal
fileDescriptor: 15 numBytesRead: 0 numBytesToRead: 4096 position: 40960
```

## La recette minimale

**Inconnue, et c'est le cœur du ticket.** Aucune recette ne le reproduit : il ne
se produit **que** pendant une passe complète de toutes les suites `e2e_*`. Ce
qui est établi, c'est qu'il ne se reproduit pas sans cette charge.

## Le témoin

`extension/rag3weaver/tests/e2e_idempotent_registration.rs`,
`kb_and_relation_persist_and_reopen`.

Depuis `8a022da9e`, les quatre sites de réouverture de cette suite disent
pourquoi ils échouent — « le journal de la base rouverte est illisible — la base
a-t-elle été fermée proprement ? » — avec la note « vu sous charge de batterie le
4 octobre, vert isolé ». Sans cette phrase, le message du moteur envoie son
lecteur chercher une fermeture sale qui n'a pas eu lieu.

## Ce qui est mesuré

| Condition | Côté | Rouges |
|---|---|---|
| batterie complète (toutes les suites `e2e_*`) | avec la sonde d'ouverture de `ensure_vector_index` | **1 / 1** |
| repos, machine calme | idem | **0 / 100** |
| repos, machine calme | sans cette sonde (worktree au point de branche) | **0 / 100** |
| sous charge d'une ingestion de dépôt (charge moyenne 11,2 à 17,6 ; `io some avg10` 4 à 5 % ; phase d'analyse et d'ingestion, écritures de journal **sans aucun point de reprise**) | avec la sonde | **0 / 40** |
| même charge, alternés tour par tour | sans la sonde | **0 / 40** |
| mesure antérieure de la session cœur C++, au repos | avant la sonde | 2 / 20 |

Les deux séries de cent sont passées par **le même script**, avec la **somme du
contenu** de `librag3db.so` relevée au début et à la fin de chacune et vérifiée
identique — sans quoi la mesure se déclarait invalide.

**Ce que ces chiffres établissent** : la sonde d'ouverture n'introduit pas le
défaut, et le taux de 2 sur 20 n'est pas un taux de repos. À 10 %, tomber sur
zéro en cent tirages arrive une fois sur trente-sept mille ; le phénomène dépend
donc de la charge.

**Ce qu'ils n'établissent pas** : que la sonde ne l'aggrave pas **sous** charge.
Le repos ne peut plus rien en dire, puisqu'il ne produit plus le phénomène.

## Ce qui a été essayé sans succès

**Le tampon étroit du moteur** (`RAG3DB_BUFFER_POOL_SIZE`). L'hypothèse venait
de la session embarquements : avec un tampon de 2 Gio, un point de reprise
échoue en pleine poussée des blobs (« A checkpoint of this database failed, so
it must be closed and reopened »). Pression mémoire → point de reprise qui
échoue → base « à rouvrir » → et c'est à la réouverture que ce test tombe. Le
mécanisme était plausible, et il aurait donné une recette en une variable
d'environnement au lieu d'une charge empruntée.

**Il ne reproduit pas.** Sept tailles essayées, cinq exécutions chacune :

| Tampon | Résultat |
|---|---|
| 256 Mio, 64 Mio, 48 Mio | 0 rouge sur 5 |
| 32 Mio | 1 rouge sur 5 |
| 24 Mio, 20 Mio, 16 Mio | 5 rouges sur 5 |

(et aucun de ces rouges n'est le défaut cherché : voir ci-dessous)

Mais **tous** les rouges sont un **autre** échec : « Buffer manager exception:
Unable to allocate memory! The buffer pool is full », à la ligne 1051 (une
recherche BM25), jamais la lecture de journal de la ligne 1019. Le tampon étroit
casse le test sans produire le phénomène.

À noter pour quiconque reprend : un compte seul aurait conclu l'inverse. « 5
rouges sur 5 » ressemblait à une reproduction nette ; c'est en ouvrant le
message qu'on voit que ce n'est pas la même panne. **Un rouge se lit, il ne se
compte pas.**

Produit secondaire, sans gravité (un tampon de 24 Mio n'est pas une
configuration de service) : entre 20 et 32 Mio, une recherche BM25 sur cette
base échoue par épuisement du tampon de façon reproductible.

**Un point de reprise échoué, sur deux bases réelles.** La session
embarquements a gardé deux bases laissées dans l'état « A checkpoint of this
database failed, so it must be closed and reopened » — tampon de 4 Gio, seuil
de point de reprise à 512 Mio, extension vector chargée, schéma de code,
1,7 Go chacune. C'était la condition la plus proche du mécanisme supposé :
pression mémoire → point de reprise qui échoue → base « à rouvrir » → et c'est
à la réouverture que ce test tombe.

**Il ne reproduit pas, et il apprend autre chose de rassurant.** Sondées par le
chemin du produit, sur des copies reflink :

| | base 1 (tampon 4 Gio) | base 2 (les deux réglages) |
|---|---|---|
| ouverture (rejeu du journal) | **ok** | **ok** |
| chargement de l'extension | ok | ok |
| lignes (File / Scope / Symbol) | 5042 / 47737 / 39373 | 4018 / 41807 / 35617 |
| les trois index HNSW au catalogue | présents | présents |
| montage complet du catalogue | **ok** | **ok** |

Donc **« must be closed and reopened » ne laisse aucun dégât durable** : c'est
une poignée morte, pas une base cassée. Et la réouverture **consomme le
journal** — la copie est passée de 1,577 à 2,135 Go, le `.wal` de 162 Mo et le
`.shadow` ont disparu. Un point de reprise échoué ne perd rien ; il remet le
travail à la réouverture suivante.

Sonde : `tests/sonde_reouverture.rs` (`SONDE_BASE=<chemin>`), qui ne juge rien
et ne s'arrête pas au premier échec — chaque étage dit ce que le suivant ne dira
pas.

## La cause, si elle est connue

Non connue. Deux pistes, aucune vérifiée :

1. **Le moteur** : une lecture de journal qui rend zéro octet à une position
   valide ressemble à une lecture concurrente d'un fichier en cours d'écriture,
   ou à une page non encore visible.
2. **Le harnais** : la base est dans `/tmp`, qui est un **tmpfs de 61 Gio** sur
   ce poste. Aucune trace de place manquante dans le journal de la passe, et
   `/tmp` était à 23 % après coup — mais une pression mémoire pendant la passe
   (les suites `burn` chargent des modèles) peut faire partir des pages tmpfs en
   zram.

## Ce qu'il faut pour le fermer

**Le témoin sous charge est joué** (quarante de chaque côté, alternés, pendant
une ingestion de dépôt), et il ne reproduit pas davantage. **Donc la question de
la sonde est tranchée** : elle n'est pas distinguable de son absence, ni au
repos ni sous cette charge. Ce qui reste à fermer est le défaut lui-même.

**L'hypothèse qui tient encore, et le témoin qu'elle demande.** Le phénomène ne
s'est produit qu'une fois, pendant une **batterie complète**. Or une batterie
n'est pas seulement de la charge : c'est **vingt-huit processus moteur
simultanés**, chacun avec sa base et son journal. La charge d'une ingestion de
dépôt est comparable en CPU et en écritures, avec **un seul** moteur — et elle ne
reproduit pas. Le facteur serait donc la **concurrence d'instances**, pas la
charge.

Le témoin qui suit, et qui ne demande rien à personne : **le même test lancé en
parallèle avec lui-même**, huit ou seize copies en boucle, sur une machine dont
on connaît l'état. Pas encore joué.

Et quel que soit le résultat, **dire quelle charge on a eue** : « zéro sous
charge » ne vaut que si l'on nomme la charge. Les deux lignes du tableau
ci-dessus la nomment ; c'est le minimum à reproduire.
