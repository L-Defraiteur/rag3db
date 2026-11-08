# Un jeton absent du corpus rend des morceaux sans rapport à un score haut

- **État** : ouvert (8 novembre 2026, symptôme 4 du ticket fermé
  `closed/2026-10-11-un-backend-de-code-dont-la-racine-a-disparu-rend-une-erreur-brute-et-se-dit-ouvert.md`)
- **Famille** : produit (recherche)
- **Gravité** : réponse fausse — l'agent lit trois morceaux sans rapport comme des réponses sûres
- **Atteignable en service** : oui, toute recherche indexée sur un mot que le corpus ne contient pas
- **Touche rag3weaver** : oui (`search_code`, la fusion des signaux et le score rendu)

## Ce que c'est

Vu le 11 octobre 2026 vers 17 h 57 sur `code-bascule` (hôte de luciepc) : `search_code` (indexé)
sur un jeton absent du corpus rend trois morceaux sans rapport, à ★0,74-0,75. Le score dit une
proximité que rien ne porte : le dense trouve toujours un plus proche voisin, et la note ne dit pas
qu'aucun signal lexical n'a répondu.

## Attendu

Un jeton inconnu donne un score bas, ou la réponse dit « aucune correspondance lexicale » à côté
des voisins du seul vecteur.

## Le témoin

Pas encore : un corpus de quelques fichiers, une requête d'un mot qui n'y figure pas, et la
réponse doit dire l'absence de correspondance lexicale (ou rendre une note sous un seuil à fixer
d'après une mesure, pas d'un coup).

## Ce qu'il faut pour le fermer

Mesurer d'abord sur une vraie base ce que rendent les signaux pour un jeton absent, puis décider
avec la session recherche (le biais de fusion du 25 octobre est voisin) entre une note recalée et
un avertissement.
