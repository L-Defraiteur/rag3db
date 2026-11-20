# Un hôte lancé avec CONFIG_DIR et DATA_DIR n'est pas isolé

- **État** : ouvert, le 21 novembre 2026 ; pour archi
- **Famille** : produit
- **Pour** : archi
- **Gravité** : un second hôte « isolé » a repris pendant neuf secondes les sessions de chat de la
  tablette et l'équipier de l'atelier de Lucie. Le retrait du `daemon.json` de service, annoncé d'abord,
  est démenti par G : le fichier est intact.
- **Atteignable en service** : oui, dès qu'un second hôte tourne sur le poste (un essai, un témoin, un
  hôte de campagne mal réglé)
- **Touche rag3weaver** : oui, `src/daemon_lifecycle.rs`, `src/code_tree.rs`, et les lectures de `HOME`
- **Signalé par** : tty, la nuit du 20 au 21 novembre 2026 ; relevé par c7

## Les faits, puis le code

**Les faits** (G, la nuit du 20 au 21) : le `daemon.json` de la tablette est intact (mtime 20 h 50 min
54 s, pid 2800109 du service, jeton présent, les clients MCP le trouvent). Le retrait annoncé d'abord
ne s'est donc pas fait sur ce fichier. Une première version de ce ticket disait « écrit par-dessus puis
retiré » : c'était une lecture du code, pas un fait, et le fait la contredit. Ce qui a été réellement
écrit et retiré reste à lire dans les journaux de tty (l'environnement exact de son hôte, et son
journal de démarrage et d'arrêt).

**Ce que le code montre**, et qui suffit à expliquer la reprise des sessions et de l'équipier :
- **Les tâches et les chats sont partagés.** Sans `RAG3WEAVER_TACHES_DIR` ni racine des chats propre,
  un second hôte lit `~/.local/share/rag3weaver/taches` et les chats sous la même racine, quel que soit
  `RAG3WEAVER_DATA_DIR` (qui ne couvre que `code_tree::Roots`). À son démarrage,
  `reprendre_les_equipiers` relance les équipiers dont la demande n'a pas de fin : ceux de l'hôte de
  service, dont celui de l'atelier de Lucie.
- **La racine de `daemon.json` ignore `RAG3WEAVER_DATA_DIR` et `CONFIG_DIR`.** `racine(Dossier::Donnees)`
  ne lit que `RAG3WEAVER_DAEMON_DIR`, puis `HOME`. Le second hôte visait donc le même `daemon.json`.
  Que le fichier soit resté intact dit que le verrou ou `remove_if_mine` (qui compare le pid) l'ont
  protégé. À vérifier par le journal de tty.
- **L'état est éparpillé** : 38 lectures de `HOME` dans `src/`. Pour un hôte isolé aujourd'hui : un
  `HOME` à lui plus `RAG3WEAVER_DAEMON_DIR`, comme fait `Hote` dans `tools/campagne/lanceur.py`.
- **Un client retire un `daemon.json` jugé périmé**, vers la ligne 704 de `daemon_lifecycle.rs`, y
  compris pour un pid vivant mais muet.

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
