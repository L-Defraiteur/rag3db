# Au journal, un vecteur coûte trois fois sa taille : douze octets par flottant

- **État** : ouvert — une optimisation, hors de la stèle ; à faire comme une étape à part du chargement journalisé, avant sa mesure finale (décision de l'orchestration, 4 octobre 2026)
- **Gravité** : lenteur et volume, sans perte
- **Atteignable en service** : oui, à chaque écriture d'un vecteur
- **Touche rag3weaver** : oui — l'embarquement en fond écrit ses vecteurs par des `SET`, qui passent déjà par le journal : ces douze octets sont payés tous les jours
- **Ouvert le** : 4 octobre 2026, session cœur C++
- **Pour** : cœur C++ (journal)

## Ce que c'est

Un enregistrement d'insertion ou de mise à jour sérialise chaque valeur d'un vecteur comme une valeur générique, avec son type (`ValueVector::serialize`, `src/common/vector/value_vector.cpp:383-398`). Un tableau de 768 flottants y prend 9,3 Kio au lieu de 3,1.

## Ce qui a été mesuré

Le 4 octobre, seul sur le poste (`extension/rag3weaver/docs/3-octobre-2026-23h31/coeur-cpp/annexes/mesure-du-journal-d-un-chargement.cpp`) : 79 000 lignes portant un vecteur de 768 flottants et ≈ 450 caractères de texte font **732 Mio** de journal, 4,0 s de validation, et l'essentiel des 17,2 s de rejeu d'un journal de 793 Mio. Sans les vecteurs, les mêmes lignes feraient de l'ordre de 40 Mio.

Son vrai argument n'est pas le `COPY` : le premier index « cherchable par mots » de rag3weaver n'écrit pas les vecteurs, ils arrivent ensuite par des `SET` en fond, déjà journalisés. C'est l'embarquement en fond qui paie ces douze octets, à chaque vecteur.

## Ce qu'il faut pour le fermer

Une forme compacte pour les tableaux de taille fixe de types numériques (les octets bruts, comme la colonne les range). Le format du journal change : il lui faut une version, un journal de l'ancien format doit être **refusé par son nom, jamais rejoué de travers**, et l'erreur doit dire le chemin de sortie — au moins « rouvrir avec le moteur d'avant et fermer proprement » pour une base arrêtée brutalement avec un journal ancien au moment de la mise à jour.
