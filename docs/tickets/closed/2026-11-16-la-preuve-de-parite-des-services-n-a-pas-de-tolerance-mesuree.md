# La preuve de parité des services n'a pas de tolérance mesurée

- **État** : fait (19 novembre 2026, embarquement, branche `embed-parity`) ; ouvert (16 novembre 2026, embarquement ; demande de c7)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : manque de preuve. La pose d'un binaire de service se prouve aujourd'hui au bit près, sur un
  seul vecteur témoin calculé dans un seul lot ; cette preuve ne dit rien de l'écart entre lots.
- **Atteignable en service** : oui (toute pose d'un binaire `rag3weaver-embeddings` sur 7878 ou 7879)
- **Touche rag3weaver** : non pour le code du produit ; oui pour un script de preuve et sa page

## Ce que c'est

Le 15 novembre à 20 h 36, la pose du binaire f85977d09e sur 7879 puis 7878 s'est prouvée par un vecteur
témoin égal au bit près avant et après (écart 0,0). Le 16, le banc a comparé 412 morceaux communs à deux
index de la même source : 217 identiques au bit près, 195 différents d'au plus 5,86e-5 en absolu, cosinus
minimal 0,99999991. Une sonde sur le 7878 l'a départagé : un même texte rend les mêmes bits dans une même
forme de lot (écart 0,0, deux fois), et diffère d'au plus 3,77e-5 (cosinus 0,99999995) quand il passe seul
puis au milieu de 7 textes plus longs. L'écart vient du lot, pas du binaire.

## Ce qu'il faut faire

1. Un script de mesure, `scripts/embed_parity.py` (anglais), qui prend une adresse de service et un jeu de
   textes fixe (une centaine de morceaux de code et de prose, de 50 à 1 400 caractères, versé dans
   `tests/fixtures/embed_parity/texts.json`). Il les embarque **seuls**, puis **en lots de 8, 32 et 128 de
   compositions différentes** (tri par longueur, ordre inverse, mélange à graine fixe), et rend, par
   service : l'écart absolu maximal, le cosinus minimal, et la part de vecteurs identiques au bit près. Il
   n'appelle que `/sante` et `/embed` (client seulement).
2. La mesure sur le 7878 (granite-278m) et le 7879 (bge-m3), notée dans la page de règles de l'embarquement
   (`docs/embarquement/13-11-2026/01-activation-et-residence-livre-et-manquant.md`, une section « La
   tolérance de parité entre lots ») : les chiffres, la date, le binaire (sha256 de
   `~/.local/bin/rag3weaver-embeddings`).
3. La règle de preuve d'une pose, écrite dans la même section : **bit près** pour un même texte dans un
   même lot entre l'ancien et le nouveau binaire ; **sous la tolérance mesurée** (avec une marge d'un
   facteur 2) entre lots.

**Ce qui ne se touche pas** : les unités `rag3weaver-embeddings@` (aucun arrêt, aucun `.env`) ; le script
est un client. Jamais un essai qui charge un modèle sur card2.

## Comment le prouver

- Le script se joue sur la fixture `tests/fixtures/residence/fournisseur.py` (déterministe : écart 0,0
  attendu partout) : un test `tests/e2e_embed_parity.rs`, `#[ignore]`, nommé
  `embed_parity_on_the_fixture_is_bit_exact`, qui lance la fixture, appelle le script et lit son JSON.
  Rouge d'abord : le script absent.
- Puis la mesure réelle, à la main, sur 7878 et 7879, notée dans la page.

## Taille

3 à 4 heures.

## Ce qui le bloque

Rien. La mesure réelle appelle 7878 et 7879 : l'annoncer aux sessions qui mesurent (banc), quelques
secondes de charge.

## Fait (19 novembre 2026)

- `scripts/embed_parity.py` (client seul) et le jeu fixe `tests/fixtures/embed_parity/texts.json` (100 textes) ;
  témoin `embed_parity_on_the_fixture_is_bit_exact` (e2e), vu rouge (script absent) puis vert sur la fixture.
- **La mesure**, le 19 novembre à 9 h 16 sur le binaire b3764125fba4cc73 (Flex32), après la pose de G : entre lots,
  granite-278m (7878) au plus **5,51e-5** (cosinus 0,99999993), bge-m3 (7879) au plus **1,34e-3** (cosinus 0,99993) ;
  même forme de lot : au bit près sur les deux.
- **La règle de preuve d'une pose** est écrite dans `docs/embarquement/13-11-2026/01-activation-et-residence-livre-et-manquant.md`,
  section « La règle de preuve d'une pose » : au bit près pour un même texte dans une même forme de lot ; entre lots,
  sous la tolérance mesurée avec une marge d'un facteur 2.
