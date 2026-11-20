# Un lot que le service accepte peut dépasser la mémoire libre de la carte, et l'arrêter

- **État** : ouvert (20 novembre 2026, embarquement ; c'est ma requête qui l'a déclenché)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : haute. Une requête légitime, sous `max_tokens` et sous `max_chars`, arrête le service et coupe tous
  ses clients une douzaine de secondes, le temps que systemd le relance.
- **Atteignable en service** : oui (le 7879, bge-m3, sur card2 partagée avec le 7878)
- **Touche rag3weaver** : oui (`src/daemon/embeddings.rs` : le découpage d'une requête en lots, `par_lots`)

## Ce que c'est

Le 20 novembre, à 20 h 35 min 45 s, à luciepc, le 7879 sert bge-m3 sur card2 (`max_tokens_per_text` 4 096, sans
bail ni calcul en cours). Une requête client lui envoie **32 textes**, les 24 textes de la référence de granite
recyclés, de 4 à 1 742 jetons pour bge-m3.

`~/.cache/rag3weaver-unites-embarquement/journal-bge-m3.log` :
1. `failed to reserve 4357506816 bytes of device memory: can't allocate buffer of size: 4357506816`
   (cubecl-wgpu `compute/server.rs:318`, fil `DSD-0-0`) ;
2. les fils suivants : « The server is in an invalid state … Memory location was never initialized » ;
3. « ✗ arrêt : le calcul a paniqué (…) : serveur de la carte empoisonné (code 70) » ;
4. systemd relance à 20 h 35 min 57 s (compteur à 1), et le service revient sain.

Le refus nommé du ticket fermé `2026-11-15-le-service-d-embarquement-meurt-sur-une-reservation-de-carte-refusee` fait
son travail : la cause est dite, l'arrêt aussi. Ce qui manque, c'est d'éviter l'arrêt. **Le service accepte un lot qu'il
ne peut pas calculer** : le découpage d'une requête (`lot_max` textes, sous le budget d'octets) ne connaît pas la
mémoire que le lot demandera, ni celle que la carte a de libre (card2 : 20,1 Gio pris sur 32,6, dont le 7878).

## Ce qu'il faudrait

- **Borner un lot par sa mémoire, pas seulement par ses octets** : estimer la mémoire du lot (lot × jetons², pour
  l'attention) et la comparer à ce que la carte a de libre, ou à un plafond déclaré par service. Au-dessus, recouper
  le lot en plus petit avant de calculer.
- **À relever d'abord** : le budget d'octets du 7879 (`RAG3WEAVER_EMBED_CHAR_BUDGET` de son `.env`). Un lot de
  4,36 Gio suppose plusieurs textes longs dans un même lot.
- **Un témoin rouge d'abord** : un factice qui panique comme cubecl au-delà d'une taille de lot donnée. Une requête
  de textes longs l'arrête aujourd'hui ; elle doit être recoupée et servie.
- **Pour l'autovérification au démarrage** : son lot de `lot_max` textes recyclés passe par le même découpage, et
  hérite donc du même risque sur une carte pleine.

## Ce que la mesure a montré (20 novembre, 21 h 30)

- **Le 7879 n'a pas manqué de mémoire libre** : sous radv, une allocation unique est limitée à 0xfffffffc octets
  (`maxMemoryAllocationSize`, 4 Gio), sur card2 comme sur l'APU. Les 4 357 506 816 octets refusés la dépassaient, alors
  que card2 avait 12 Gio de libres.
- **Une allocation refusée empoisonne cubecl pour de bon** (essai jetable sur l'APU) : après le refus d'un tampon de
  5,95 Gio, « Parent device is lost », puis tout appel suivant panique. Recouper et réessayer est impossible.
- **La conception retenue**, page `docs/embarquement/20-11-2026/02-borner-chaque-lot-par-la-memoire-de-la-carte.md` :
  deux bornes estimées avant l'appel (le plus grand tampon sous la limite d'allocation, le total sous la moitié de la
  mémoire libre), le recoupage avant l'appel, et le 503 nommé pour un texte seul qui ne tient pas.

