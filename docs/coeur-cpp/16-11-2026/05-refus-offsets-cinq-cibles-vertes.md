# Refus des offsets : cinq cas ciblés verts sur033

Ticket fils [garantie d’exception du dictionnaire](../tickets/2026-11-16-un-ajout-au-dictionnaire-refuse-change-la-derniere-chaine.md). Branche cx-coeur-dictionary-append-atomic, correctif033ffb92068f31cdae6338db876d23e6bced3567. Admission C7 distincte1800 sur lanceur6e14180aba7dd607d559843a0333ad106c09daee publiée avant entrée, contrôle poste2/4sans mesureexclusive ; aucune lib du magasin, service ou base réelle.

## Qualification ciblée effective

Bâti1004étapes0, trois binaires construits sur la source033 propre dans tmpfs privé. Résultats :

| Cible | Cas joués | Résultat |
|---|---:|---|
| dictionary_append_exception_test | 1 | vert,0ms GTest |
| string_finalize_test | 2 | verts,5ms GTest |
| string_split_test | 2 | verts,344ms GTest |

Cinq cas, zéro failure/error/skipped/disabled dans les XML contrôlés, aucun filtre vide. Chaque appel unique, aucune répétition. Les assertions48octets/3offsets/3chaînes avant/après refus, hash/déduplication et réessai delta du témoin renforcé sont gardées et passent. La cible offsets a émis réellement :

```
BUILD_ID=rag3db-build-033ffb92068f31cdae6338db876d23e6bced3567-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF
```

Cet identifiant provient du header généré et est présent dans l’ELF, pas d’une annotation manuelle du lanceur. Le rouge acquis sur sourceb82/engine59ac reste conservé dans rouge-02, avec gamma→gammadelta/14→19 ; l’arrêt runner avanttest reste conservé dans arret-01, sans être réétiqueté comme test rouge.

## Preuves et limites

[Paquet de preuves ciblées](preuves/c1/vert-correctif-01/) : scripts/recette réels, sourcecheckout, cache, logs configure/build/3tests, XML, argv/env, manifestes/BUILD_ID, limites, inventaireSHA et quiescence. Les trois ELF/header/archive ont été sauvés horsscratch avant assertions puis rehashés après arrêt et rapatriement. Paquet complet37fichiers conservé sur les deux postes sous ~/.cache/rag3db-moteur-notes/dictionary-append-fix-01/ ; grosartefacts/sourcebundle/compile_commands absents du Git mais listés SHA/tailles dans inventaire-complet.json. Aucun cache partagé réutilisé.

Unitésuccess/exit0 en145,114s, CPU1088,971s, pic5,4G indiqué par systemd, swap0. Scratch dur32G/cgroup32G/j8/netnamespace sans route, usage final relevé dans scratch-final.json. Après sortie : inactive/dead, MainPID0/cgroupvide, aucun contrôleur/test attribué restant, place C1 rendue. Les travaux TSan/G indépendants n’ont pas été arrêtés. Aucun service/GPU/PG/basevivante/7970/installation/bibliothèque posée ni pushmaster.

Ce résultat qualifie seulement l’invariant BufferManager testé et les régressions finalize/split ciblées. Il ne qualifie ni les exceptions std::unordered_set::insert, ni les alias/concurrences possibles, ni les suites moteur complètes. Le SIGSEGV C1 n’est toujours pas attribué à ce défaut et son parent reste ouvert. Le ticket fils reste ouvert avant intégration et qualification finale pertinentes ; aucun ancien GO/délai n’est réactivé pour la suite.

## Suite à coordonner

Présenter ce vert ciblé à C7 avec la source033 et ses preuves, puis proposer une recette fraîche des suites moteur transférables, source/BUILD_ID réels et bornes explicites, avant exécution ; pas de master sans nouveau « à toi ». Ne pas relancer les ciblés sans changement/failure/raison nouvelle. Dernière lecture TSan après cette qualification : quatre tours finis15/18/16/19rapports0dictionnaire, tour5 toujours sans ligne finaleFIN ; résultat indépendant à relire sans relance automatique.
