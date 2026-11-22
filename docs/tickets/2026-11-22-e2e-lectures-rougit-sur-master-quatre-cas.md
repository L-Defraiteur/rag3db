# e2e_lectures rougit sur master : quatre cas

- **État** : ouvert (22 novembre 2026, 5 h 15), rag3weaver archi, à la demande de c7.
- **Famille** : produit
- **Gravité** : la suite des lectures ne juge plus quatre de ses cas sur master. Le défaut peut aussi bien venir d'un
  lot de la nuit que de l'environnement de luciepc.
- **Atteignable en service** : à départager. Si c'est un lot, oui : un outil de lecture tiendrait la base ouverte en
  écriture.
- **Touche rag3weaver** : oui (`tests/e2e_lectures.rs`, et le lot coupable).
- **Pour** : à départager entre les lots listés plus bas ; G pour l'environnement.
- **Signalé par** : c7 (le pousseur, 05:00), puis le départage d'archi à 05:08.

## Ce qu'on voit

À luciepc, la suite `e2e_lectures` est rouge **sur master seul** : 0ae74fecf8, jouée de 05:01 à 05:08 par la voie du
pousseur, charge basse, 25 passés sur 28. Trois cas sont rouges :

- `un_outil_de_lecture_ne_prend_jamais_le_mutex_du_catalogue` (e2e_lectures.rs:420) : « code.rag3db is already open
  for writing… », avec 6 références encore tenues après 10 s ;
- `le_creux_ne_s_embarque_qu_au_surlignage` (:1515) : « l'index n'a pas fini » ;
- `la_requete_entiere_passe_devant_les_mots`.

La passe d'ED de la même nuit en a vu un quatrième : `le_creux_se_surligne_par_le_chemin_de_l_hote`.

Le journal est à luciepc : `~/.cache/rag3weaver-build/departage-lectures-master/e2e_lectures.log`.

## Ce qui est hors de cause

- **La porte du mode auto** (mode-auto-v1) : master seul est rouge sans elle.
- **Le (A) de lucivy** : au verdict de G à 05301576ba, qui vient après (A), e2e_lectures était VERT 28/28.

## Les suspects

Entre 05301576ba et 0ae74fecf8 ont atterri, en Rust :

- pin-lib 2c4bddcd3c ;
- dangling-links (règles 16) ;
- emb-shared-text-l1 ;
- user-token-2 et tty-card-2 (le crate tty seul) ;
- maintenance-witness-tagged (le témoin seul) ;
- fermeture-garde-la-tete-3.

Une cause d'environnement à luciepc reste possible : G la cherche depuis 04:40.

## Pour départager

Jouer e2e_lectures sur 05301576ba, puis sur chaque atterrissage suivant, dans l'ordre, sous la même lib et au même
poste. Le premier rouge désigne le lot.

## Les rouges connus

Les quatre cas sont dans `tools/poste/known_red_e2e.txt` et citent ce ticket. Le lot qui corrige un cas retire sa
ligne.
