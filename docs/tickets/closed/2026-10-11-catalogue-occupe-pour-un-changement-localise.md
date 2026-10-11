# « Catalogue occupé » pour un changement localisé

- **état** : corrigé (le commit des trois défauts de la montre, 11 octobre 2026)
- **gravité** : blocage (une lecture refusée)
- **atteignable en service** : oui — une lecture pendant la réindexation d'un fichier
- **touche rag3weaver** : oui (et seulement)

## Ce que c'est

Vu par Lucie sur l'hôte de luciepc : « pas normal, ça aurait dû être rapide, un seul
changement localisé ». La réaction de la montre tient le catalogue le temps de réindexer
un fichier ; `read_catalog` faisait un seul `try_lock` et refusait au premier échec, avec
« une indexation le tient ».

## Le témoin

`backend::tests::a_short_hold_of_the_catalog_does_not_refuse_a_read` : le catalogue tenu
300 ms, `usages` répond.

## La correction

Une lecture attend un catalogue tenu jusqu'à `READ_WAIT` (1 s) : la réindexation d'un
fichier passe, seule une tenue longue (une indexation entière) rend le refus. La réponse
porte le retard dit (W3).
