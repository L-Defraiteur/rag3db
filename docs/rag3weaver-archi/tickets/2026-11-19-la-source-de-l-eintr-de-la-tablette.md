# La source de l'EINTR de la tablette

- **État** : ouvert, le 19 novembre 2026 ; pour archi
- **Famille** : outillage
- **Pour** : archi
- **Gravité** : faible : le correctif `reprise-eintr` fait reprendre l'attente d'une réponse, et le témoin `a_backend_closes_between_two_tool_calls_of_a_step` ne rougit plus pour cette raison. Ce qui reste inconnu, c'est d'où vient le signal.
- **Atteignable en service** : peut-être : un client du produit qui attend une réponse sur une socket à délai, sur le portable, peut recevoir le même EINTR. `agent_with_timeout` le reprend désormais.
- **Touche rag3weaver** : non, c'est un relevé
- **Signalé par** : tty, le 19 novembre 2026 (deux rouges au portable, aucun à luciepc)

## Ce qui est su

- L'erreur : `Io(Os { code: 4, kind: Interrupted, message: "Interrupted system call" })` sur le `send` d'un agent de ureq à délais (`agent_de_temoin`), le 19 vers 12 h, puis l'après-midi (journal de tty : `~/.cache/rag3weaver-build/cx-garde-rouge/test.log`, tablette).
- **Ce qui est écarté :**
  - **Un gestionnaire installé par un test voisin** : une cale `LD_PRELOAD` qui journalise chaque `sigaction` et `signal` montre que le binaire de tests n'installe que SIGSEGV et SIGBUS, la garde de pile de Rust. C'était sur une passe valide de `daemon_host::teammate_tests::a_` à luciepc, 31 tests.
  - **Le moteur** : `librag3db.so` n'importe ni `sigaction` ni `signal`.
  - **Une veille ou un gel d'unité** : le journal du système de la tablette ne montre qu'une sortie de veille, à 9 h 09.
  - **Les scripts de profil du banc** (`eu-stack -p`) : ils n'attachent que leur propre enfant.
- **Ce qui reste plausible :** un arrêt et une reprise du processus de tests sans gestionnaire (ptrace d'un outil d'échantillonnage, SIGSTOP et SIGCONT). D'après `man 7 signal`, ils rendent EINTR aux appels sur socket à délai.

## Ce qu'il faut faire

À la prochaine occurrence sur le portable, relever en même temps `/proc/<pid>/status` (`SigCgt`, `TracerPid`) du binaire de tests et le journal du système autour de l'instant. Sans nouvelle occurrence, fermer le 3 décembre 2026.

## Taille et blocage

Une heure, à la prochaine occurrence. Ne bloque rien.
