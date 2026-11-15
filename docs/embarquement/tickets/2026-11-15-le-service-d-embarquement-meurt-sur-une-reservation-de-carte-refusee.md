# Le service d'embarquement meurt sur une réservation de carte refusée, sans le dire

- **État** : ouvert (15 novembre 2026, embarquement ; relevé par G, ticket demandé par c7)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : défaut. Le bge-m3 de luciepc (`rag3weaver-embeddings@bge-m3`, 7879) a redémarré 21 fois
  entre 11 h 39 et 15 h 11. Chaque redémarrage coupe les clients une vingtaine de secondes : à 15 h 09, il a
  coupé le tunnel 7980 → 7879 du portable.
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (`src/daemon/embeddings.rs`, le calcul burn/cubecl du service)

## Ce que c'est

Le journal de l'unité est vide parce que le drop-in `journal.conf` envoie la sortie vers
`~/.cache/rag3weaver-unites-embarquement/journal-bge-m3.log`. C'est là qu'il faut lire. Le noyau ne dit ni
OOM ni reset amdgpu depuis 11 h 30, et `coredumpctl` ne montre aucun cœur du service.

Chaque redémarrage suit le même enchaînement :
1. `failed to reserve <N> bytes of device memory: can't allocate buffer of size: <N>`, panique du fil
   `DSD-0-0` de cubecl-wgpu (`compute/server.rs:318`). N va de 2,4 à 4,1 Go ;
2. les fils de calcul suivants paniquent : « The server is in an invalid state … Memory location was never
   initialized » (`burn-std conversion.rs:232`), ou « Tensor Operation Error » (`burn-tensor base.rs:3050`) ;
3. le processus sort, et systemd le relance (`Restart=always`). L'unité garde `ExecMainStatus=0`, et rien ne
   dit pourquoi il est parti.

De 12:53Z à 14:11Z, les tailles refusées reviennent dans le même ordre, cycle après cycle : 4102146304,
3553113664, 2890997824, 3240797184, 3737610496, 3363072064, 2437199424, 3836068096, 2562789376,
2744083456, 3544535296, 2607532096. C'est donc une même suite de demandes, rejouée : les lots d'une passe
du banc qui appelle le 7879 à chaque décision. Le vrai jeu de l'embarquement, de 14:03:55Z à 14:04:30Z,
s'est joué entre deux pannes (14:00:16Z et 14:08:56Z) sans en déclencher.

Le 7879 partage card2 avec le 7878 (granite-278m). Une demande qui veut 2,4 à 4,1 Go de plus ne les trouve
pas, et le service meurt au lieu de refuser cette seule demande.

## Ce qu'il faudrait

- **Refuser la demande, pas mourir** : une réservation refusée rend une erreur nommée au client (503,
  « carte card2 : <N> octets demandés, <M> libres »), puis le lot se recoupe en plus petit, ou se refuse. Le
  serveur cubecl est empoisonné après une panique : à vérifier d'abord s'il se rétablit, ou s'il faut le
  recréer dans le processus ;
- **à défaut, mourir en le disant** : une dernière ligne nommée au journal (la réservation, la taille, la
  demande), et un code de sortie non nul, pour que `NRestarts` et `ExecMainStatus` se lisent ;
- **borner la demande** : un lot qui demande 4 Go à une carte partagée est à recouper avant l'appel. C'est la
  piste du `lot_conseille` (32 × 512).

Témoin, rouge d'abord : une carte factice, ou un plafond de mémoire de calcul, et un lot trop gros. Le service
répond une erreur nommée et reste à l'écoute ; le lot suivant, plus petit, passe.

## La cause des tailles : l'attention d'un seul texte long (15 novembre, 15 h 3x)

Le banc l'a expliqué. Ses passes N, F et F 25 % rangent les mêmes 244 tours dans le même ordre, par l'outil
`archive` de la mémoire, et embarquent la Source (le tour entier), ses L1 et les fiches. Les tours sont longs :
le plus long fait 43 137 caractères, 21 dépassent 16 000 et 2 dépassent 32 000.

Les tailles refusées sont celles de la matrice d'attention d'**un seul** texte : 4 102 146 304 octets font
exactement 16 têtes × 4 octets × 8 006². C'est un texte de 8 006 jetons, au plafond de bge-m3 (8 192). Les
autres tailles (2,4 à 3,8 Go) sont celles de textes de 6 000 à 7 700 jetons. La mémoire de l'attention croît
comme le carré de la longueur.

`RAG3WEAVER_EMBED_CHAR_BUDGET` n'y peut rien. Il range les textes en lots, mais un texte plus long que le budget
part seul, entier (`embedder.rs`, « le plancher est un chunk »).

**Correction de 96 (15 h 4x) : le texte long est une requête, pas une écriture.** Les écritures sont déjà
découpées en morceaux de 1 500 caractères. Mais `archive_decide` cherche ses fiches candidates avec, pour
requête, la section entière. `embed_query` (`src/search.rs:968`) embarque cette requête en un seul texte, sans
la couper. 96 borne désormais la requête de l'archiviste à 4 000 caractères dans son gabarit. Toute autre
recherche à requête longue refera le même coup : un agent qui colle un fichier dans `recall`, par exemple.

Ce qu'il faudrait, en plus du refus nommé :
- **dans `embed_query`** : un plafond de longueur de la requête embarquée (de l'ordre du budget, 8 192
  caractères, soit environ 2 048 jetons), dit dans les avertissements de la recherche quand il coupe. Le plein
  texte garde la requête entière ;
- **côté service** : un plafond de jetons par texte, déclaré dans l'identité, avec la troncature comptée et
  dite (`troncatures`), plutôt qu'un plafond qui n'existe que dans le modèle.

Témoin de `embed_query`, rouge d'abord : une requête de 40 000 caractères arrive à l'embarqueur bornée, et la
recherche le dit.
