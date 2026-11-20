# L'équipier crée une section non déclarée au lieu de le dire

- **État** : ouvert, le 21 novembre 2026 ; pour archi
- **Famille** : produit
- **Pour** : archi
- **Gravité** : faible ; le correctif élargit son effet au-delà de ce que le ticket demandait
- **Atteignable en service** : oui, toute tâche de correctif
- **Touche rag3weaver** : oui, le texte d'un pas de l'équipier (les gabarits de la tâche)
- **Signalé par** : banc, au jugement du rejeu de nuit C, le 21 novembre 2026

## Ce qui s'est passé

Le rejeu C a réussi : même fichier, même intention que le correctif atterri, témoin vert. Une différence
reste : l'équipier fait `setdefault("sandbox", {})`, qui crée une section `sandbox` dans un gabarit qui
n'en déclarait pas. Le correctif atterri (e3f6e10313) ne touche qu'un bac déjà déclaré.

## Le pas exact qui bloque

Rien dans la consigne d'un pas ne dit qu'une déclaration absente reste absente. Le modèle complète par
défaut.

## Ce qu'il faut faire

Une règle dite au pas, dans le texte déclaré de la tâche, pas dans le code : « on ne crée pas ce qui
n'est pas déclaré ; une section absente se dit dans le rapport, avec ce qu'elle aurait changé ». Le banc
la vérifie au prochain rejeu de C.

## Comment le prouver

Rejouer C après la règle : le diff ne crée plus de section `sandbox`, ou le rapport dit pourquoi il le
faudrait.
