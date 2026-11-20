# Embarquement : l'autovérification de l'embarqueur au démarrage

Décision de Lucie, transmise par c7 le 20 novembre : **on ne fait pas confiance à un adaptateur, on le vérifie.** La
cause est le ticket `2026-11-20-sur-l-igpu-granite-rend-des-vecteurs-faux-en-lot`. L'iGPU Intel (ARL, anv) de luciepc
calcule des vecteurs faux en lot, sans erreur, alors que l'APU AMD de la tablette calcule juste. À terme, le garde-fou
par type (`check_embedder`) et sa variable d'autorisation deviennent inutiles.

## La forme (convenue avec c7)

- **Contre quoi** : une **référence enregistrée**, les vecteurs de l'ONNX original par onnxruntime (CPU, f32). Ce n'est
  pas un calcul CPU au démarrage, pour trois raisons : un adaptateur de plus, qui peut lui-même mentir ; des dizaines
  de secondes sur llvmpipe ; un adaptateur absent de certains postes.
- **Où** : `reference.json` à côté de `model.bpk` (`~/.cache/rag3weaver/<modèle>/`, ou le dossier des poids). Le
  fichier porte :
  - les phrases, leurs jetons et leurs vecteurs ;
  - la tolérance du modèle ;
  - **l'empreinte blake3 du `model.bpk`** qu'elle couvre.

  Un modèle changé ne se compare donc jamais à la référence d'un autre.
- **Quoi** : chaque phrase **seule**, puis **en lot**. Le lot fait la taille réelle `lot_max` du service, en recyclant
  les phrases, et il est découpé comme le service découpe une requête (`budget_batches` sous `embed_char_budget()`).
  Le défaut de l'Intel se voit en lot, pas à deux.
- **Les phrases** : celles des références de granite (24 phrases, dont une de 182 jetons et deux de 512, la fenêtre).
  La forme qui a paniqué le 19 y est.
- **Le seuil** : l'écart absolu maximal à la référence, sous la tolérance du modèle.
  - granite : 5e-4, le seuil de `contre_la_reference_onnx`, six fois la plus grande mesure en Flex32.
  - bge-m3 : mesuré à la production de sa référence, sur card2.
  - L'Intel donne de 0,07 à 0,13 : la marge est de deux ordres de grandeur.
- **Au-dessus** : la carte est **refusée**, et la raison est dite : le pire écart, la phrase, seule ou en lot. Pas de
  repli sur le CPU, qui serait lent au point de passer pour une panne.
- **Le résultat dans `/sante`** : un champ `verification` de l'identité, par modèle :
  - l'état (`verified`, `refused`, `not_verified`) ;
  - les deux écarts (seul, en lot) ;
  - la tolérance, l'empreinte et la durée.
- **Sans référence, ou une référence d'autres poids** : `not_verified`, dit au journal et dans `/sante`. Le service
  démarre, et le garde-fou par type reste en place. C'est la seule raison de le garder.
- **Les deux chemins** : le binaire `rag3weaver-embeddings`, avant sa chauffe ; `local_embedder`, qui rend une erreur
  quand la carte est refusée.

## Les témoins, rouges d'abord

1. **Unité, avec un factice qui calcule faux en lot** : la vérification refuse, et dit l'écart et la forme. Un factice
   juste passe. Une référence d'autres poids donne `not_verified`.
2. **Le chemin de l'hôte, à luciepc** : le binaire sur l'iGPU Intel, réglage d'autorisation posé, est refusé par la
   vérification, avec son écart.
3. **Verts** : le même binaire sur card2, et sur l'APU de la tablette.

## La production des références

`generated/reference_granite.py` s'étend à bge-m3 (le vecteur dense : CLS normalisé) et écrit l'empreinte blake3 du
`model.bpk`. L'ONNX de bge-m3 est à luciepc (`~/.cache/bge-m3-weights/model.onnx`). Les vecteurs de granite existent
déjà (`tests/fixtures/granite/`) : il ne leur manque que l'empreinte.

## Taille

Une demi-journée, références comprises.
