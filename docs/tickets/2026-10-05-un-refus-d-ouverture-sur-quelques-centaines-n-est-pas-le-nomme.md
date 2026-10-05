# Un refus d'ouverture en lecture sur quelques centaines n'est pas le refus nommé

- **État** : ouvert.
- **Gravité** : réponse fausse (un refus légitime qui ne dit pas sa raison — et un test d'intermittence).
- **Atteignable en service** : oui — tout lecteur read-only ouvert pendant qu'un écrivain checkpoint souvent.
- **Touche rag3weaver** : oui (Rag3dbConnection::read_only, le refus borné).

## Ce que c'est

Sous un écrivain qui checkpoint toutes les cinq écritures, un lecteur qui
ouvre en boucle (borne RAG3WEAVER_READ_ONLY_CROSSED_MS=1) reçoit presque
toujours le refus nommé (CHECKPOINT_CROSSED_READ_ONLY_OPEN + « fois en ») —
mais ~1 refus sur quelques centaines prend une AUTRE forme, non identifiée.

## La recette minimale

e2e_prise_atomique::un_lecteur_affame_est_refuse_par_son_nom_dans_sa_borne,
en boucle : mesuré le 5 octobre 2026 (master frais, moteur e1049934e+) —
REFUS=70/NOMMES=62, puis 455/454, puis 158/158 (vert). L'écart varie avec la
charge de la machine.

## Le témoin

Le test lui-même, mais il est intermittent — et l'enfant compte Err(_) sans
LOGGER le texte du refus non-nommé : première amélioration, faire imprimer
le premier refus d'une autre forme pour identifier le message fautif.

## La cause

Non établie. Fenêtre pendant le checkpoint où l'ouverture échoue par un
autre chemin (le moteur a changé la nuit du 4 au 5 : e1049934e, 34bde7eea),
ou second message légitime à ajouter à la liste des nommés.

## Ce qu'il faut pour le fermer

Logger le refus fautif, l'identifier, puis : le nommer (s'il est légitime)
ou fermer la fenêtre (si c'est une erreur brute qui fuit).
