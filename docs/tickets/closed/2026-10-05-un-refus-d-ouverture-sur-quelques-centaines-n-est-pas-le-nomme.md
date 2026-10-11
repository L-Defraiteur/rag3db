# Un refus d'ouverture en lecture sur quelques centaines n'est pas le refus nommé

- **État** : corrigé `f5de0a3f0` — doublon, voir le ticket
  [lecteur affamé par les points de reprise](2026-10-04-lecteur-affame-par-les-points-de-reprise.md),
  section « Deux messages pour le même croisement » (f2fdc8bab).

Le refus non nommé était « Couldn't replay shadow pages under read-only
mode » : le même croisement d'un point de reprise, sous un second message du
moteur. `is_checkpoint_crossing` reconnaît les deux, les deux sont repris
sous la borne de 2 s, et l'enfant du test dit son premier refus non nommé
(`AUTRE_REFUS=…`). Après correctif : 13/13, 661 refus, 661 nommés.

(Ouvert par la session recherche le 5 octobre à 2 h 24 sur trois mesures —
70/62, 455/454, 158/158 — fermé dans l'heure par l'arbre principal, qui
l'avait trouvé en parallèle. Leçon déjà connue : regarder l'index des
tickets avant d'en ouvrir un.)
