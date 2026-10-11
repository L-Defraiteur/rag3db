# L'hôte sur luciepc, parlé d'ici par un tunnel

**11 octobre 2026, session optimiseur.** Lucie a dit oui : l'hôte des backends
(`rag3weaver-backend daemon`) tourne sur luciepc, et ce poste-ci
(lucie-tablette) lui parle par un tunnel ssh. Rien de nouveau n'écoute sur le
réseau : l'hôte est sur la boucle locale de luciepc.

## Ce qui est en place (11 octobre, 16 h 40)

| quoi | où, sur luciepc |
|---|---|
| binaire | `~/.local/bin/rag3weaver-backend`, master a3c1ca98f, features `rag3db-native,code,daemon,openai-llm`, hors de tout target |
| moteur | la lib commune `~/git_workspaces/rag3db-lourd/build/lecteurs-csv/src/librag3db.so`, liée par `RUNPATH` inscrit dans le binaire |
| extension vecteur | `~/git_workspaces/rag3db-lourd/extension/vector/build/libvector.rag3db_extension` — même bâti que la lib, identifiants égaux (`rag3db-build-f024578d…`) |
| lancement | l'unité utilisateur `rag3weaver-daemon` (recommandée), activée |
| environnement de l'unité | `~/.config/rag3weaver/rag3weaver-daemon.env` : `RAG3WEAVER_DAEMON_ADDRESS=127.0.0.1:7970`, `RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,127.0.0.1:7879,127.0.0.1:7880` (adresses complètes), `RAG3DB_ROOT=~/git_workspaces/rag3db-lourd` |
| écoute | `127.0.0.1:7970`, et rien d'autre |
| fiche et jeton | `~/.local/share/rag3weaver/daemon.json`, mode 0600 |
| journal | `~/.local/state/rag3weaver/daemon.log` |

## La recette

**Le binaire**, depuis un worktree de master sur luciepc, contre la lib commune :

```bash
L=$HOME/git_workspaces/rag3db-lourd/build/lecteurs-csv/src
cd <worktree de master>/extension/rag3weaver
RAG3DB_SHARED=1 RAG3DB_LIBRARY_DIR=$L RAG3DB_INCLUDE_DIR=$L \
RUSTFLAGS="-C link-args=-Wl,-rpath,$L" \
CARGO_TARGET_DIR=$HOME/.cache/rag3weaver-build/target-G-hote CARGO_INCREMENTAL=0 \
  ~/.cache/rag3weaver-build/poste lourd cargo build --release --bin rag3weaver-backend \
  --features rag3db-native,code,daemon,openai-llm
install -m 755 $CARGO_TARGET_DIR/release/rag3weaver-backend ~/.local/bin/rag3weaver-backend.nouveau
mv ~/.local/bin/rag3weaver-backend.nouveau ~/.local/bin/rag3weaver-backend
```

Le `RUSTFLAGS` est nécessaire : le `rpath` que pose `tools/rust_api/build.rs`
ne passe pas d'une crate de bibliothèque au binaire final (c'est pourquoi
`run_e2e.sh` pose `LD_LIBRARY_PATH`). Sans lui : `librag3db.so => not found`.

**Lancer par l'unité** (recommandé ; `rag3weaver-unit` vient de la branche
`paquet-npm`, `bindings/nodejs/bin/`) :

```bash
RAG3WEAVER_BACKEND=$HOME/.local/bin/rag3weaver-backend node bin/rag3weaver-unit.js install \
  --env RAG3WEAVER_DAEMON_ADDRESS=127.0.0.1:7970 \
  --env RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,127.0.0.1:7879,127.0.0.1:7880 \
  --env RAG3DB_ROOT=$HOME/git_workspaces/rag3db-lourd
node bin/rag3weaver-unit.js status       # ou uninstall
```

**Ou à la main**, si Lucie préfère sans unité :

```bash
RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,127.0.0.1:7879,127.0.0.1:7880 \
RAG3DB_ROOT=$HOME/git_workspaces/rag3db-lourd \
  nohup ~/.local/bin/rag3weaver-backend daemon >> ~/.local/state/rag3weaver/daemon.log 2>&1 &
# l'arrêter : kill -TERM <pid de daemon.json>   (jamais -9)
```

Les deux ensemble, non : le second refuse de démarrer (un seul hôte) ; l'unité
qui voit un hôte manuel échoue en code 1 sans boucler.

**Après un rebâti de la lib commune** (`rag3db-lourd`) : `systemctl --user
restart rag3weaver-daemon`, pour que l'hôte charge la nouvelle lib ; si le
moteur a changé d'interface (`tools/rust_api`), rebâtir aussi le binaire.

## Redémarrer l'hôte : l'annoncer avant

**Un redémarrage de l'hôte s'annonce avant, avec l'heure, à chaque session qui
l'interroge par MCP** (orchestration, 11 octobre au soir, à la demande du cœur).
Le rebâti de la lib commune s'annonce déjà à codeparsers ; c'est la même règle.
Pour l'hôte de luciepc, ce sont le cœur et hotfixs. La liste à prévenir se lit
par `daemon_status`, qui dit les sessions ouvertes.

Vu le 11 octobre à 20 h 11 : G a rebâti le binaire sur faa00f279, puis a arrêté
l'unité avec le seul accord de codeparsers. L'appel du cœur en cours a reçu
« connection reset by peer » au milieu d'une mesure. L'arrêt lui-même était
propre : inactif en moins de 30 s, l'indexation arrêtée au prochain paquet. La
reconnexion automatique du client est vérifiée par la session mémoire.

L'ordre :
1. `daemon_status`, pour savoir qui a une session ouverte.
2. Un message à chacune, avec l'heure prévue. Codeparsers en plus si une
   indexation tourne.
3. `systemctl --user stop`, surveillé cinq minutes. Un `deactivating` qui
   dure veut dire que l'arrêt pend : le dire, ne rien tuer.
4. `start`, puis `/sante` (même pid que `daemon.json`), le build et la lib
   chargée (`/proc/<pid>/maps`).
5. Dire « relancé » aux mêmes sessions.

## Le tunnel, d'ici

```bash
ssh -f -N -L 7971:127.0.0.1:7970 lucied@luciepc
```

7971 pour ne pas se confondre avec un hôte de ce poste-ci (7970). Pour le
fermer : le processus `ssh` qui porte `-L 7971:…` — **pas** par `pgrep -f`
avec ce motif, qui attrape aussi le shell qui le contient (vu ce jour-là).

## Le jeton, pour le côté client

- Il vit dans `~/.local/share/rag3weaver/daemon.json` **sur luciepc**
  (champs `address`, `pid`, `token`, `build`, `started_at`), mode 0600, et
  **change à chaque démarrage de l'hôte** (un redémarrage par l'unité aussi).
- Toute route sauf `GET /sante` l'exige (`Authorization: Bearer <jeton>`) ;
  sans lui, 403.
- `address` y vaut `127.0.0.1:7970` (l'adresse vue de luciepc), pas l'adresse
  du tunnel : un client d'ici qui reprend le jeton « si l'adresse correspond »
  ne le trouvera pas. Le client doit lire le jeton par un chemin ou une
  option (par exemple `ssh luciepc cat ~/.local/share/rag3weaver/daemon.json`),
  et le relire sur un 403 ou un redémarrage. C'est la part de la session
  mémoire.

## Les témoins (11 octobre)

- **Depuis luciepc**, un client MCP local (`rag3weaver-backend mcp --manifest`
  sur une copie de `proto/boutique`) trouve l'hôte de l'unité et
  `daemon_status` le montre : pid 3744221 (= `MainPID` de l'unité),
  `state: running`, le backend ouvert et la session du client.
- **Depuis ici, par le tunnel** : `curl http://127.0.0.1:7971/sante` rend
  `{"service":"rag3weaver-daemon","pid":3744221,"build":"… ~/.local/bin/rag3weaver-backend@…"}` ;
  `curl …/etat` sans jeton rend 403.
