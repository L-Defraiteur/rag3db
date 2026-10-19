# Ce qu'il faut savoir pour reprendre (G, 19 octobre 2026, 2 h)

## Le témoin navigateur

- `src/daemon_host/navigateur.rs` : `Navigateur::lancer`, `aller(url, condition)` (attend un
  nouveau document où la condition JS est vraie, 20 s), `attendre`, `evaluer`, `commande` (le
  protocole DevTools brut, 30 s). Les erreurs relevées : exceptions, `console.error` / `warn` /
  `assert`, journal en erreur. Une attente dépassée dit la vue, les appels en vol
  (`fetch` enveloppé dès la création du document) et les erreurs.
- Le navigateur : `RAG3WEAVER_NAVIGATEUR`, sinon chromium / google-chrome dans le PATH, sinon le
  cache Playwright (luciepc : `chrome-headless-shell` 1217). Sans navigateur : une ligne `SAUTÉ
  <nom>` ; `RAG3WEAVER_EXIGER_NAVIGATEUR=1` (posé dans la CI) en fait un rouge.
- Lancé par `--remote-debugging-pipe` : fd 3 en entrée, fd 4 en sortie, un message JSON par fin
  `\0`. Les tubes passent au-dessus de 10 avant `dup2` vers 3 et 4.
- Le test de l'aperçu à chaud du proto (`daemon_host/apercu_a_chaud.rs`) exige
  `RAG3WEAVER_PAQUETS_REACT` quand le navigateur est exigé : `scripts/paquets_react.sh <dossier>`
  (npm pack de react, react-dom, scheduler, react-refresh à versions fixées). À luciepc :
  `~/.cache/rag3weaver-build/paquets-react`.
- Jouer le témoin contre un hôte en service sans compiler ici : bâtir le binaire des tests à luciepc
  (`cargo test --lib --no-run --message-format=json`, la lib du magasin au même chemin des deux
  côtés), le rapatrier, et le lancer avec `LD_LIBRARY_PATH` sur la lib du magasin, `--ignored
  --exact daemon_host::navigateur::l_ide_d_un_vrai_hote_se_rend_en_lecture_seule`.

## La mémoire de l'hôte

- `crate::allocateur::rendre_au_systeme(raison)` : `malloc_trim(0)` sous Linux glibc, une ligne
  `[mémoire] <raison> : VmRSS a → b Mio` sur la sortie d'erreur. Appelé en fin d'index
  (`index_nodes.rs`, ligne au journal de l'index aussi), à la fermeture d'un backend
  (`DaemonHost::close_slot`) et au repos (le fil du relevé, 60 s sans requête ni travail, une fois
  par repos).
- Mesure : un hôte isolé (`RAG3WEAVER_DAEMON_DIR` à part, un port à soi, jamais 7970 ni 7979-7981 à
  luciepc, qui sont les services d'embarquement), piloté par `mesure-hote.py` (session,
  `/backend/open`, `/call index`, journal suivi jusqu'à « indexation terminée »,
  `/backend/close`, `/arret`), relevé par `sonde-hote.sh` du banc.
- **luciepc envoie l'anonyme inactif en échange zram dès T0** (`swappiness` 150, 93,6 Gio de zram) :
  toute mesure de mémoire là-bas compte RssAnon + VmSwap.

## L'hôte de Lucie

- Recette du binaire : `bati-lucie-2.sh <id de lib>` (scratchpad) sous `poste lourd`, dans le
  worktree `rag3db-hote-socket` détaché sur master ; refuse si `courant` n'est pas la lib annoncée.
- Bascule quand il dort : `.pret` puis `mv`, `RAG3DB_LIBRARY_DIR` de l'env, `installer-l-unite-de-l-hote.sh
  --filet-seul`, `daemon-reload`, réveil par `/sante`. **Un appel à `/etat` réveille aussi l'hôte**
  (la socket le lance) : vérifier `ActiveState` après tout appel, avant de remplacer le binaire.
- Vérifications : MainPID = pid de `/sante`, la lib dans `/proc/<pid>/maps`, `LISTEN_FDS=1`,
  l'environnement du processus, le md5 de `daemon.token`, `recall` par le client MCP de Lucie (le
  canal `vector` dans la réponse), puis le témoin en lecture seule.
- Ses copies de gabarits : `~/.config/rag3weaver/code-rag3db/templates` et
  `~/.config/rag3weaver/memoire-longue`. Le relevé (`derive-gabarits.py`, scratchpad) classe chaque
  fichier identique / ancien / modifié contre les gabarits livrés d'un commit, et ne rafraîchit que
  les anciens. Le lot des gabarits désignés par leur nom doit rendre ce relevé inutile.

## La CI

- Sur master, GitHub ne garde qu'un run en attente par groupe de concurrence : un push remplace celui
  qui attend (« cancelled », sans job), jamais celui qui tourne. Les deux workflows ont
  `cancel-in-progress: ${{ github.ref != 'refs/heads/master' }}`.
