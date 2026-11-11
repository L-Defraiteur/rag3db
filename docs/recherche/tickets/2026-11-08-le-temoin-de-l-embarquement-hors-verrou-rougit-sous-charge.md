# Le témoin de l'embarquement hors du verrou rougit sous charge, une fois sur deux

- **État** : ouvert (8 novembre 2026), pour recherche après le lot residus-de-suite (c7), cause à départager.
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
