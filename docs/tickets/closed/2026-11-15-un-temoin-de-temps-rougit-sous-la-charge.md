# Un témoin de temps rougit sous la charge : usages pendant un écrivain, 107 ms pour 100

- **État** : fermé (15 novembre 2026, recherche) : la forme en rapport, avec une garde absolue dite par ce qu'elle sépare.
- **Ouvert le** : 15 novembre 2026, relevé par archi.
- **Famille** : produit
- **Pour** : recherche
- **Gravité** : faux rouge de passe. Un témoin vrai sur le fond rougit sous une machine chargée, et fait douter d'un lot qui n'y touche pas.
- **Atteignable en service** : non : c'est un témoin, pas un comportement du service.
- **Touche rag3weaver** : oui (`tests/e2e_lectures.rs`, les lectures pendant un écrivain, lot L6).
- **Signalé par** : archi, sur la passe du lot du nom de l'agent (repose sur `6142131cb`).

## Ce que c'est

`e2e_lectures::usages_repond_pendant_qu_un_ecrivain_tient_le_catalogue` mesure le temps d'un
`usages` pendant qu'un écrivain tient le catalogue, et exige un absolu : `assert!(ms < 100, …)`
(`tests/e2e_lectures.rs:121`).

Le 15 novembre vers 20 h 20, sur le portable, la machine était chargée par d'autres passes (charge
moyenne autour de 5). Le témoin a rougi : « usages n'attend pas l'écrivain : 107 ms ». Rejoué seul
juste après, sur le même binaire, il est vert ; le test entier tient en 2,46 s.

## Ce qui est à trancher

La borne ne se relâche pas sans preuve. Mais un absolu mesure aussi la charge de la machine, et pas
seulement ce que le témoin veut prouver : que `usages` n'attend pas l'écrivain. Deux formes
possibles, à choisir par recherche :

- **un rapport** : `usages` pendant l'écrivain, au plus k fois `usages` sans écrivain, mesuré dans
  le même témoin, sur la même machine, au même moment ;
- **ou l'absolu, en disant la charge** : le témoin note la charge moyenne et le temps, pour qu'un
  rouge sous la charge se lise comme tel.

L'écart qui compte, entre « n'attend pas » et « attend l'écrivain », se compte en secondes, pas en
millisecondes : un rapport le garde, sans dépendre de la charge.

## Ce qui est tranché (recherche, 15 novembre 2026)

**La forme en rapport, plus une garde absolue**, dans le même témoin
(`tests/e2e_lectures.rs`, lot `exclusion-apres-coup`) :
- **la garde : moins de 500 ms.** Attendre l'écrivain coûte au moins le délai d'occupation du
  catalogue. Pour le voir, la vue de lecture a été retirée à la main : `usages` rend alors « Catalogue
  occupé » au bout de 1 001 ms, et la garde rougit. Elle sépare donc bien « n'attend pas » de
  « attend » ;
- **la vitesse : au plus 4 fois `usages` sans écrivain.** La référence est mesurée dans le même témoin,
  après un appel qui chauffe, sous la même charge. La borne ne descend jamais sous les 100 ms d'avant :
  sur une machine au repos, elle reste 100 ms, et rien ne se relâche là où l'absolu tenait. Sous une
  charge qui ralentit tout, la référence ralentit aussi, et la borne avec elle.

Joué le 15 novembre à luciepc, lib 373b5477b :
- tel quel : 10 ms pendant l'écrivain, pour une référence de 12 ms, sous une borne de 100 ms ;
- sans la vue de lecture : 1 001 ms, rouge par la garde.

L'absolu avec la charge dite n'a pas été retenu : il aurait laissé le témoin rougir sur la charge
seule, en le disant, au lieu de juger ce qu'il prétend juger.
