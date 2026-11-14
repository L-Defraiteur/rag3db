# Rapport de session c7 — 14 novembre 2026

Suite du [rapport du 13](../13-11-2026/02-rapport-de-session-c7.md), qui court jusqu'à 4 h 35 (la nuit :
1a, la liaison, le flux partagé, l'empreinte, les thèmes, la lib 0c41f9fe6, la perte de Note prouvée).

## 13 h — reprise

- Lucie revient (quota hebdomadaire remis à zéro ; la session s'était arrêtée dans la nuit). Rien n'a bougé
  depuis 4 h 35 : master = 7d7524ff0, courant = 835d5298f aux deux postes, hôte 7970 sous 835d5298f.
  Toutes les sessions dormaient.
- **La pose de 0c41f9fe6** : mon message à G (« Lucie est là », déroulé de la pose) a été **refusé par le
  classifieur du mode auto** (arrêt d'hôtes = action destructrice). Je ne le reformule pas ; Lucie écrit
  le mot à G elle-même, c'est le même mot (G l'a dit cette nuit).
- Lots du matin donnés : cœur → correctif (a) de la perte de Note ; arbre principal → 1b sur copie ;
  lucivy → 4.3.7 (score sous masque) puis la mesure ; archi → thèmes de l'IDE (hôte à deux fils) ;
  recherche → Surveillant (le plafond au mot de Lucie) ; embarquement → la démo sans service, puis le
  vrai jeu après la pose ; ED → relecture de la page 30 finale, puis le plan de 2b-B ; hotfixs → la remise
  orpheline, le relevé du gabarit éphémère. 96 et codeparsers attendent les décisions de Lucie
  (archivage, import_binding) ; tty sans lot, à proposer.
- **archi, cause du rouge des thèmes, plus précise** : `/ide/etat` lit index_states, qui prend le catalogue par `catalog_read::lock_for_read` (attente ≤ READ_WAIT, 1 s) ; la recherche en mode auto (generic_search_nodes.rs:446 et :467) prend le même catalogue par un **try_lock nu** : tenu quelques ms par une lecture d'état, elle conclut « occupé ou illisible » et balaye, et la vue Search ne relance pas. La requête /ide/themes.json ne verrouille rien, elle décale l'ordre des requêtes au chargement — **défaut de master, latent**, rendu déterministe par le témoin des thèmes. Décision : la recherche prend le catalogue par lock_for_read (borné à 1 s), le balayage reste pour une indexation entière et la réponse le dit ; **lot à part devant les thèmes**, ticket + témoin rouge d'abord.
