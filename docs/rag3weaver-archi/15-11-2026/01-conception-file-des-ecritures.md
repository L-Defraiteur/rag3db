# Les écritures reçues sous une fermeture qui tient se rangent en file

Le reste du ticket de 96 (`docs/tickets/closed/2026-11-14-un-backend-ferme-par-l-hote-se-rouvre-au-premier-appel.md`) :
sous une fermeture qui tient (`1cbf1cf3a`), une écriture est refusée et perdue en le disant. Pendant
la fenêtre d'hotfixs sur code-lourd, c'est un vrai manque pour Lucie.

## La conception, en quatre lignes

1. **Ce qu'est une écriture** : un appel que `PreparedBackend::reads_only` ne prouve pas en lecture —
   son graphe instancié (liaisons comprises) et son crochet après sont `snapshot_safe` (L6), sa
   politique n'écrit ni ne lance rien, il n'a pas de harnais. *Changé au code (c7, 15 novembre,
   00 h 5x)* : le drapeau « écrit » à part que je proposais est retiré ; `snapshot_safe` est déjà posé
   par chaque fabrique et prouvé par son test (graph_tool.rs), un seul rédacteur d'un fait déjà prouvé
   vaut mieux qu'un second marqueur qui pourrait le contredire. Le défaut est prudent : un nœud non
   marqué, un outil inconnu ou un graphe qui ne s'instancie pas est rangé, jamais servi comme une
   lecture. Une lecture reste refusée comme aujourd'hui.
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
