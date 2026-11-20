# Un hôte lancé avec CONFIG_DIR et DATA_DIR n'est pas isolé

- **État** : ouvert, le 21 novembre 2026 ; pour archi
- **Famille** : produit
- **Pour** : archi
- **Gravité** : un second hôte « isolé » a repris pendant neuf secondes les sessions de chat de la
  tablette et l'équipier de l'atelier de Lucie. À son arrêt, il a retiré le `daemon.json` de l'hôte de
  service (G l'a remis).
- **Atteignable en service** : oui, dès qu'un second hôte tourne sur le poste (un essai, un témoin, un
  hôte de campagne mal réglé)
- **Touche rag3weaver** : oui, `src/daemon_lifecycle.rs`, `src/code_tree.rs`, et les lectures de `HOME`
- **Signalé par** : tty, la nuit du 20 au 21 novembre 2026 ; relevé par c7

## La cause

- **La racine de `daemon.json` ignore `RAG3WEAVER_DATA_DIR` et `RAG3WEAVER_CONFIG_DIR`.**
  `racine(Dossier::Donnees)` ne lit que `RAG3WEAVER_DAEMON_DIR`, puis XDG ou `HOME`. Un hôte lancé avec
  les deux autres variables écrit donc son `daemon.json` par-dessus celui de l'hôte de service. À son
  arrêt, `remove_if_mine` le trouve « sien », puisqu'il porte son pid, et le retire. Il ne reste alors
  plus aucun `daemon.json`.
- **L'état d'un hôte est éparpillé.** Il y a 38 lectures de `HOME` dans `src/`, et `CONFIG_DIR` et
  `DATA_DIR` n'en couvrent qu'une partie : `code_tree::Roots`. Les tâches (`RAG3WEAVER_TACHES_DIR`),
  les runs (`RAG3WEAVER_RUNS_DIR`), les chats, l'état (`RAG3WEAVER_STATE_DIR`), le registre des agents
  et les crochets ont chacun leur variable, ou aucune. C'est pour cela que l'hôte de campagne d'archi
  tourne avec un `HOME` à lui (`tools/campagne/lanceur.py`, `Hote`), et non avec ces variables.
- **Un client retire un `daemon.json` « périmé » d'un hôte vivant.** `daemon_lifecycle.rs`, vers la
  ligne 704 : un pid vivant mais muet est jugé périmé, et son fichier est retiré.

## Ce qu'il faut faire

1. **Une seule racine d'état pour un hôte** : `RAG3WEAVER_HOME`, ou un dossier passé au lancement
   (`daemon --home <dossier>`). Elle couvre les sessions, les chats, les tâches, les runs, l'état, le
   registre des agents, les crochets et `daemon.json`. Les variables existantes restent des surcharges
   fines sous elle. Elle est dite au démarrage et à `/sante`.
2. **Un hôte ne retire jamais un `daemon.json` qui n'est pas le sien** : il compare son pid et son
   `procStart`. Un client ne retire jamais celui d'un pid vivant.
3. **Un hôte qui trouve un `daemon.json` vivant d'un autre** refuse de démarrer dans cette racine, en
   nommant l'autre.

## Comment le prouver

Rouge d'abord :
- deux hôtes, deux racines, aucun état partagé (ni chat, ni tâche, ni `daemon.json`) ;
- l'arrêt de l'un ne touche pas l'autre ;
- un hôte lancé dans la racine d'un hôte vivant refuse en le nommant ;
- un client ne retire pas le `daemon.json` d'un pid vivant.

## Taille et blocage

Un jour, à cause des 38 lectures de `HOME` à ramener sous la racine. Le point 2 seul tient en une
heure, et il éviterait déjà le retrait du `daemon.json` de service.
