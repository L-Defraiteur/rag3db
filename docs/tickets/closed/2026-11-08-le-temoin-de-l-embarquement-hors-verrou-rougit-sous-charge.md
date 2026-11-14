# Le témoin de l'embarquement hors du verrou rougit sous charge, une fois sur deux

- **État** : corrigé (14 novembre 2026, branche `surveillant`, recherche). Le témoin disait faux : le
  chemin n'appelle pas le service sous le verrou, ce sont les sondes du Surveillant qui se gênaient
  entre deux appels en vol. Le correctif est dans le témoin, avec deux témoins du Surveillant lui-même.
- **Famille** : produit
- **Gravité** : témoin instable ; s'il dit vrai, une lecture peut attendre le service d'embarquement
  sous charge, ce que le chemin hors du verrou devait empêcher.
- **Atteignable en service** : à départager (la réaction de la montre embarque par ce chemin).
- **Touche rag3weaver** : oui, `Catalog::embarquer_le_retard_hors_verrou` et son témoin.
- **Pour** : recherche (suivi et lot ; toute réattribution passe par l’orchestration).
- **Signalé par** : recherche ; sources et observations dans ce ticket et le [rapport de session](../11-11-2026/01-rapport-de-session.md).

## Ce que c'est

`e2e_invariant_des_vecteurs::l_embarquement_du_retard_appelle_le_service_hors_du_verrou`
(tests/e2e_invariant_des_vecteurs.rs:472) rougit par intermittence : « le service a été appelé sous
le verrou du catalogue ». Son `Surveillant` note un appel au service pendant lequel `try_lock` du
catalogue échoue, donc un appel pendant que quelqu'un d'autre tient le verrou.

## La mesure

À luciepc, le 8 novembre 2026, le test seul, dix essais de suite, chacun après le bâti de son
commit (lib cbc06c3e1 avec MOTEUR_ANCIEN) :

| commit | rouges | charge |
|---|---|---|
| master 5cc6f4770 | 5 / 10 | 17 à 23 |
| residus-de-suite bdeed8788 | 6 / 10 | 24 à 27 |

Seul, sur une machine moins chargée, il passait 4 fois sur 5 (le rouge au premier essai, le plus
lent, 3,35 s contre 0,28 s). Il était vert dans la passe entière de master du matin. Le défaut
précède le lot residus-de-suite et dépend de la charge. **Précision (c7)** : la « charge » de
luciepc ce jour-là était une attente disque (NVMe à 98 % en TRIM asynchrone, PSI des E/S à 22-30),
pas des cœurs pleins ; Lucie la règle, et les dix essais seront à refaire après.

## Ce qui reste

Trouver qui tient le verrou pendant l'appel sous charge : un fil de fond du catalogue, ou un
appel au service fait dans le temps (a) ou (c), sous le verrou, quand une étape est lente. Le
témoin dira alors vrai ou faux ; s'il dit vrai, le corriger dans le chemin, pas dans le témoin.

## Le départage et le correctif (14 novembre 2026, recherche)

**Les dix essais, sur disque calme** (luciepc, lib 0c41f9fe6, PSI io « some » avg10 de 1,7 à 5,7) :
4 rouges sur 10. Le Surveillant relevait ses appels en vol et chaque `try_lock` refusé. Chaque essai
fait deux appels au service, et ils sont en vol ensemble : le nœud d'embarquement lance deux fils
producteurs (`RAG3WEAVER_EMBED_THREADS`, 2 par défaut).

**Avec un seul fil producteur** (`RAG3WEAVER_EMBED_THREADS=1`), dans les mêmes conditions : 0 rouge sur 10.

**La cause** : dans ce témoin, seuls le test et le Surveillant tiennent le mutex du catalogue. La sonde
d'un appel (un `try_lock` réussi) tenait le verrou un instant, et la sonde de l'autre appel en vol, au
même instant, le voyait tenu. Le chemin hors du verrou n'est pas en cause.

**Le correctif**, dans le témoin :

- les sondes passent une à une, sous un mutex propre au Surveillant ; un vrai détenteur du catalogue ne
  passe pas par lui et reste vu ;
- **un `;` après le `if let` de la sonde.** Le premier correctif, sans lui, laissait le nouveau témoin
  rouge. Le `if let` était l'expression de fin du bloc, et ses temporaires (la garde du `try_lock`
  réussi) vivaient jusqu'après la garde de la sonde (édition 2021) : la sonde se libérait avant le
  catalogue. Un programme à part le montre : 334 757 refus sur 800 000 sondes sans le `;`, 0 avec.

**Les témoins** (`tests/e2e_invariant_des_vecteurs.rs`) :

- `deux_appels_en_vol_ne_se_denoncent_pas_l_un_l_autre` : quatre fils de 2 000 appels sur un catalogue
  que personne ne tient. Rouge avant le correctif, rouge encore avec le premier seul, vert avec les deux ;
- `le_surveillant_voit_un_verrou_tenu_par_un_autre` : le témoin du témoin, vert de bout en bout ;
- le témoin d'origine, après le correctif : 10 verts sur 10 (charge plus basse, 7 contre 13 : la preuve
  tient surtout au témoin déterministe).
