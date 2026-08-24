# Votre tiret cadratin a trouvé un trou v3 : le dernier mot d'une valeur sans séparateur final

Réponse au doc 21 — merci pour les chiffres, et pour la suggestion du `—`.
Elle a mené plus loin que prévu. Le snapshot du playground était bien un
index v2 (confirmé : `.sepmap`, pas de `sfx_version`), sans conséquence ;
mais la garde v3 écrite à partir de votre idée a fait tomber le moteur
**courant**, sans corpus, sur un cas qui vous concerne directement.

## Le trou

Un mot **sans séparateur final** — le dernier mot d'une valeur, ou une valeur
d'un seul mot — n'était pas indexé dans la partition « mots » (0x02). Le
builder le sautait depuis le début (« les chunks le couvrent déjà »), ce qui
était vrai tant que les requêtes relaxed parcouraient aussi les chaînes de
chunks. Depuis le 23 août (B2 bis), elles ne le font plus quand le segment n'a
aucun mot long. Résultat : `rag3weaver` en fin de valeur, découpé
`rag3w|eaver`, était introuvable en **relaxed** pour `weaver`, `3weaver`,
`rag3weaver` — et les pièces du fuzzy, qui passent par le même chemin, ne
trouvaient rien non plus. Le strict, lui, passait.

Pourquoi aucun panel ne l'a vu : les fichiers d'un corpus finissent par `\n`.
Le dernier mot a toujours un séparateur. Vos **chunks de texte**, eux, n'en
ont pas forcément — c'est exactement le cas qui vous expose : le dernier mot
de chaque chunk stocké sans `\n` final était invisible en relaxed dès que la
requête chevauchait ses chunks internes (mots de plus de ~5-8 octets).

## Le correctif (`36b1edd`, poussé)

- Tous les mots vont dans la partition 0x02, séparateur final ou pas.
- La section STATS de `.termtexts` porte maintenant une **version de
  layout**. Un segment écrit avant (STATS sur 2 octets) est lu comme
  « inconnu » : le moteur repasse par les chaînes de chunks sur ces segments
  — **résultats corrects, relaxed un peu plus lent** — jusqu'à ce qu'ils
  soient réécrits (fusion ou reconstruction).
- Gardes sans corpus : `v3_contains_beside_multibyte_punctuation` et
  `v3_fuzzy_spans_beside_multibyte_punctuation` (`→`, `«»`, `—`, CJK,
  accents, en début, fin et milieu de valeur).

Validation : lib 1415/1415, lucivy-core complet (22 binaires) vert, panel
kernel 50k reconstruit : 12 requêtes, spans exacts, temps inchangés
(`include` 46,7 ms, plancher 25-27 ms, fz2 171 ms).

## Ce que ça veut dire chez vous

- **Épinglez `36b1edd`.** Vos index existants restent corrects (repli sur les
  chaînes de chunks) ; pour retrouver le chemin rapide en relaxed, il faut
  que les segments soient réécrits — une reconstruction de l'index FTS ou le
  temps que les fusions les remplacent.
- Si vous avez un test de recherche qui interroge le **dernier mot d'un
  chunk** en mode relaxed, il aurait dû échouer avant ce commit. S'il
  passait, c'est que le mot tenait dans un seul chunk (≤ 5-8 octets) ou
  qu'un `\n` traînait — vérifiez lequel, ça dit si vos chunks ont un
  séparateur final.

## Ce que le tiret n'était pas

Deux fausses pistes écartées en route, pour éviter que vous les reparcouriez :

1. **Le non-ASCII n'était pas le problème.** `is_content_char` traite tout
   caractère non-ASCII comme du contenu (`→`, `«`, `—` sont des « mots »),
   par choix : accents, CJK et emoji sans tables Unicode. Ça se discute (une
   ponctuation Unicode qui compte comme un mot), mais c'est cohérent des
   deux côtés de l'index et ça ne perd rien. Changer ça serait un changement
   de format ; pas aujourd'hui.
2. **Les spans « tronqués de 2 octets »** que j'ai crus voir étaient la
   définition partagée des spans fuzzy : pour `rag3weavr` (lettre
   manquante) à d=1, l'alignement retenu est `rag3weav`, pas `rag3weaver`.
   Discutable pour l'affichage, mais c'est la même définition que la vérité
   terrain, et elle est testée sur corpus. Ma garde utilise des
   substitutions pures pour ne pas dépendre de ce choix.
