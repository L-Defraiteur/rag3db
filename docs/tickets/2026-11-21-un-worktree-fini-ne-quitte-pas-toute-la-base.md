# Un worktree fini ne quitte pas toute la base

- **État** : ouvert (21 novembre 2026, 18 h 3x), arbre principal.
- **Famille** : produit
- **Gravité** : la base grossit de ce qui est fini. Lucie veut qu'un worktree fini disparaisse aussi de la base, pas
  seulement du disque. Aujourd'hui, une tâche fusionnée ou rejetée laisse toutes ses lignes ; une vue fermée à la main
  laisse les siennes ; et même une vue purgée laisse la fiche de sa couche et les contenus devenus orphelins.
- **Atteignable en service** : oui, à chaque tâche fermée et à chaque vue fermée.
- **Touche rag3weaver** : oui (`src/suivies.rs` : `end_layer`, `purge_layer`, `fermer_une_tache` ;
  `src/daemon_host/taches.rs` : `fermer_une_tache` ; `src/daemon_host.rs` : `fermer`).
- **Pour** : arbre principal ; au mot de c7 du 21 novembre, 18 h 3x. Embarquement s'y branche pour les textes orphelins.
- **Signalé par** : Lucie, relayée par c7.

## Ce que chaque fermeture retire aujourd'hui (relevé dans le code)

- **`close_view` à la main** : `suivies::end_layer`. La fiche de la couche reçoit `fin` ; ses lignes et ses générations
  restent, lisibles par leur id.
- **`close_view` avec `purge`, et la fermeture tirée d'une vue atterrie** (`fermer(…, true)`) : `suivies::purge_layer`.
  Les lignes de la couche quittent les tables à générations, ses générations quittent `_generation`, puis elle finit.
  La couche `fourche:<commit>` suit, quand aucune autre vue ouverte ne la partage. **Restent** : les clés de méta
  `fiche_couche:<dépôt>:<id>` et `couche_nom:<dépôt>:<nom>`, et les contenus que plus aucune couche ne porte.
- **La fusion ou le rejet d'une tâche** (`fermer_une_tache`) : la couche de sa branche sort des suivies et se marque
  fermée ; **rien n'est effacé** (le code le dit : « la fermeture de ses générations est un autre lot »).

## Le correctif attendu

1. **Une tâche fermée purge sa couche**, comme une vue atterrie : fusionnée, ses lignes sont dans le principal ;
   rejetée, elles ne servent plus.
2. **Une couche purgée retire aussi sa fiche** (`fiche_couche:…`, `couche_nom:…`). Une vue par son ancien nom se
   refuse alors « couche inconnue », au lieu de « couche finie ».
3. **Les contenus orphelins** (les textes qu'aucune couche ne porte plus) partent dans la même écriture : la forme B
   d'embarquement s'y branche.
4. **Une vue fermée à la main** : décider si elle purge par défaut (Lucie : « disparaître de la base ») ou si elle garde
   `end_layer` avec un `purge` explicite. Question pour Lucie.

## Témoins, rouges d'abord

- Une tâche ouverte, une écriture, puis rejetée : ses lignes et ses générations ne sont plus en base.
- Une vue purgée : sa fiche ni son nom ne sont plus dans la méta.
- Un contenu porté par la seule couche purgée n'est plus en base ; un contenu partagé reste.
