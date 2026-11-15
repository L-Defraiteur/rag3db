# Mesurer la plus longue transaction d'écriture de rag3weaver sur code-lourd

- **État** : ouvert (15 novembre 2026)
- **Famille** : produit
- **Gravité** : planification. C'est la borne d'attente d'un ALTER sous le verrou de schéma du cœur
- **Atteignable en service** : oui. Un ALTER attend désormais les écrivains ouverts de sa table
  (décision du cœur, nuit du 14 au 15 novembre)
- **Touche rag3weaver** : non. C'est une mesure, `src/code_sync.rs` n'est que lu
- **Pour** : everything declarative (ED), mesure pour le cœur
- **Signalé par** : le cœur, à la demande de c7 (15 novembre 2026, 8 h)

## La demande

Deux durées, mesurées sur une copie migrée de code-lourd, sous `poste mesure` :
1. le paquet le plus long d'une synchronisation en masse sous transaction : un paquet de 2 048
   fichiers (`BATCH_FILES_DEFAULT`), validé à sa fin (`RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION` = 1) ;
2. la transaction du chargement final des relations différées (`finir_les_relations_differees`).

Pour mémoire, la même nuit : une resynchronisation, que ce soit par l'écrivain des générations ou par
paquet, n'ouvre aucune transaction explicite, chaque instruction s'y validant seule.

## Ce qui est déjà connu

Les seuls chiffres en main datent d'octobre, sur rag3db-eb, en masse et sous transaction : 113 à 157 s
pour toute la synchronisation (4 octobre 2026). Ils ne disent ni le paquet le plus long ni le COPY
des relations à part.

## Ce qu'il faut pour le fermer

Les deux durées, avec la lib, la copie et le profil (`RAG3WEAVER_INGEST_PROFILE=1`), données au cœur
pour son témoin, à côté de la borne de son scénario.
