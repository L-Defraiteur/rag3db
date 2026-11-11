# Le délai du service d'embarquement est un chiffre rond

- **État** : ouvert (8 novembre 2026), pour l'arrière-plan : la mesure dans la fenêtre de G.
- **Famille** : produit
- **Gravité** : lenteur ou faux rouge possibles (un délai trop court coupe un lot vivant sous charge ; trop long, il retient une passe derrière un service qui ne rend pas).
- **Atteignable en service** : oui (l'embarqueur, le relecteur et l'OCR d'un démon, en service comme en test).
- **Touche rag3weaver** : oui (`src/daemon/embeddings.rs`, `default_embed_timeout`).

## Ce que c'est

Le délai d'un appel au service d'embarquement — l'embarqueur, et depuis le lot du délai le
relecteur et l'OCR, par la même variable `RAG3WEAVER_EMBED_TIMEOUT_SECS` — vaut **60 s** par
défaut, un chiffre posé le 3 octobre sur une seule mesure (256 morceaux en 2 s par le tunnel,
« marge ×30 ») et que son commentaire donne pour « mesure par service à venir ». Le lot du délai
ne l'a pas changé : un seuil ne se touche pas sans mesure (c7).

## Ce qu'il faut mesurer

Le **p99** d'un `/embed`, d'un `/rerank` et d'un `/ocr` à luciepc **sous charge** (une passe
complète d'une autre session et une indexation en même temps, comme entre 0 h 45 et 1 h 46 le
8 novembre), par service (granite-278m, granite-107m, bge-m3) et par taille de lot conseillée.
Le délai devient alors ce p99 avec une marge dite, par service si les écarts le demandent.

## Le témoin

`daemon::embeddings::tests::le_relecteur_et_l_ocr_d_un_demon_muet_rendent_une_erreur_nommee_dans_leur_delai`
tient le comportement (une erreur nommée dans le délai) ; la valeur, elle, se juge par la mesure.

## Ce qu'il faut pour le fermer

La mesure, sa page, et le défaut remplacé par la valeur mesurée (un commit qui la cite).
