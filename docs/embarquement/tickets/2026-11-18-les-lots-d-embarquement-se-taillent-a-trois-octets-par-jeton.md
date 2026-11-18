# Les lots d'embarquement se taillent à trois octets par jeton

- **État** : ouvert (18 novembre 2026, embarquement ; point B de la page des bornes, différé par c7)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : souhait. Un lot de textes denses porte au pire 3 fois les jetons prévus côté service, et une aire
  d'attention sous-estimée jusqu'à 9 fois côté client local. Pas de panique : depuis le lot `token-cap`, un texte ne
  dépasse plus la fenêtre de son modèle.
- **Atteignable en service** : oui, sur un corpus dense (CJK, accents, emojis, hexadécimal)
- **Touche rag3weaver** : oui (`src/embedder.rs` : `budget_batches`, `stable_batches` ; `src/daemon/embeddings.rs` :
  `par_lots` ; `src/burst.rs`)

## Ce que c'est

Les planificateurs de lots comptent des octets (`text.len()`) et estiment les jetons à `octets / 3`. Le pire ratio
mesuré le 18 novembre vaut 1 jeton par octet (`docs/embarquement/18-11-2026/02-bornes-en-caracteres-et-limites-en-jetons.md`).
- **Le service** (`par_lots`, `budget_batches`) : budget linéaire de 8 192 octets par lot ; un lot dense porte jusqu'à
  8 192 jetons au lieu d'environ 2 700. Sur une carte partagée, `burst::next_paced_batch` se règle sur les durées
  mesurées, donc se corrige de lui-même.
- **Le client local** (`stable_batches`) : aire `seq × jetons² / 2` sur `octets / 3`, sous-estimée jusqu'à 9 fois.

## Ce qu'il faudrait, au choix

1. **Le moins cher** : estimer à 1 jeton par octet au lieu de 3, sans encodage. Des lots plus petits en moyenne,
   environ 3 fois plus d'appels sur la prose : **mesurer la perte de débit d'abord** (le banc, sur un corpus de prose et
   un corpus de code).
2. **Le vrai compte** : `count_tokens` au trait `Embedder`, implémenté par chaque embarqueur burn, et les planificateurs
   sur ce compte. C'est un second encodage par texte.

Témoin : un lot de textes hexadécimaux de même longueur en octets que des textes de prose se taille plus petit.

## Taille

1 : une heure, plus la mesure du banc. 2 : une demi-journée.
