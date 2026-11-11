# Cap produit confirmé le 11 novembre

Lucie confirme deux chantiers prioritaires à faire avancer en parallèle :

1. **Les vues par worktree sans cloner l’index**, avec une **pyramide de couches**.
2. **Le chat de l’IDE doit disposer de tout pour coder comme un agent standard.**

Les lots déjà engagés de fiabilité et leurs passes se terminent ; les prochains lots produit
se rattachent à ces deux axes. Les raffinements de la jauge passent après le parcours de code.

## Un index, des vues et des couches

Arbre principal tient le chantier ; ED relit le modèle déclaratif et recherche le filtrage.
Point de départ : `extension/rag3weaver/docs/08-novembre-2026-arbre/01-conception-des-vues-marche-1.md`.

Le résultat attendu est un index ancre par dépôt et des couches de différences par branche/worktree,
avec héritage des branches empilées et masquage des éléments supprimés. Une recherche dans une vue
rend la bonne version sans mélanger les couches ; un contenu réutilisable garde ses vecteurs.
Le garde-fou des scopes déplacés ne constitue pas à lui seul la livraison de ce modèle.

Terminer les relectures ED/recherche, découper les lots et leurs témoins, puis implémenter selon
ces contrats. Les mesures M3–M5 demandent un créneau convenu avec optimiseur, sans banc concurrent
qui rendrait leurs durées ininterprétables. Aucun nouvel accord humain implicite n’est ajouté.

## Un parcours de code complet dans le chat

Archi tient le parcours navigateur, le flux et les cartes ; ED tient les contrats d’outils,
les gabarits et les règles de l’agent. Ils vérifient le chat de l’atelier et celui d’un dépôt ou
d’une tâche, avec leurs périmètres respectifs.

La preuve attendue est une demande réelle depuis l’IDE qui mène à un résultat vérifiable :
explorer et chercher, lire les fichiers et les relations, modifier, utiliser les commandes et
les outils git/worktree pertinents, exécuter les tests, montrer les sorties et le diff.
Le parcours doit aussi reprendre après un accord, rendre une erreur lisible, permettre l’arrêt
et ne pas annoncer « fait » sans le livrable demandé. La présence d’un outil dans le code
ne prouve pas qu’il est accessible et utilisable depuis ce chat.

Commencer par l’inventaire effectif et le premier blocage démontré, puis corriger par lots avec
rouge/vert et validation du parcours. Les protections des espaces de Lucie restent applicables.

## Convention des branches et worktrees

Lucie demande le préfixe **`cx-`** pour identifier les branches et worktrees des sessions reprises.

- Toute nouvelle branche et tout nouveau worktree portent ce préfixe, y compris lors de la reprise
  d’une branche existante : par exemple `cx-archi-ide-par-le-flux` et `cx-wt-flux`.
- Pour un lot dont une passe tourne, conserver ses chemins jusqu’au prochain point sûr ; ne pas
  déplacer un worktree ou modifier ses scripts sous une compilation en cours.
- À la reprise suivante, créer le nouveau nom préfixé depuis le commit voulu, pousser la branche
  et mettre à jour scripts et rapport. Ne pas réécrire les commits ni forcer un push pour renommer.
- Les anciennes branches distantes restent : aucune suppression sans le mot de Lucie.
- Les règles de worktree isolé, de branche neuve après rebase et de passe/push restent inchangées.

L’orchestration tient la file des pushes de code. Ce cap et cette convention ne donnent aucun tour.
