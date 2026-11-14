# Les écritures reçues sous une fermeture qui tient se rangent en file

Le reste du ticket de 96 (`docs/tickets/closed/2026-11-14-un-backend-ferme-par-l-hote-se-rouvre-au-premier-appel.md`) :
sous une fermeture qui tient (`1cbf1cf3a`), une écriture est refusée et perdue en le disant. Pendant
la fenêtre d'hotfixs sur code-lourd, c'est un vrai manque pour Lucie.

## La conception, en quatre lignes

1. **Ce qu'est une écriture** : un outil dont le graphe porte un nœud qui écrit la base
   (`InsertRecordNode`, `UpdateRecordNode`, `DeleteRecordNode`, `LinkRecordNode`, l'index), ou
   `policy.write_files`. Le code n'a pas ce marqueur aujourd'hui : il se déclare **dans le schéma du
   nœud** (un drapeau « écrit », posé par sa fabrique), pas dans une liste de noms. Une lecture reste
   refusée comme aujourd'hui.
2. **La file** : sous une fermeture qui tient, une écriture se range dans la file du backend (outil,
   arguments, client, heure), bornée en nombre et en durée (réglables, horizon le `until` de la
   fermeture) ; le refus dit « fermé : <raison>, depuis <heure de fermeture>, jusqu'à <quand> ; ton
   écriture est en attente, rang N ». Au-delà de la borne, le refus nommé d'aujourd'hui.
3. **Le rejeu** : à `POST /backend/open`, la file se rejoue dans l'ordre, sur la base rouverte (celle
   que la passe a rendue) ; chaque rejeu dit son sort au journal de l'hôte, un échec n'arrête pas les
   suivants. `/etat` et `daemon_status` montrent la file (combien, la plus ancienne, les échecs du
   dernier rejeu).
4. **Rouge d'abord** par `e2e_demon_hote` : fermer avec une raison, un `put_fiche` rangé au rang 1,
   un `get_fiche` refusé, puis `/backend/open` : la fiche est en base, une seule fois, et l'état dit
   la file vide. Question ouverte : la file survit-elle à un redémarrage de l'hôte (un fichier à côté
   de son état) ? Je propose oui, sinon une fenêtre longue perd tout à un arrêt.
