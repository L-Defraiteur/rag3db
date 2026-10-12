# Le mode démon ne tient que sous Linux : Windows ne compile pas, macOS ne reconnaît pas l'hôte vivant

- **État** : corrigé (12 octobre 2026, optimiseur, lot C, branche `portage-c`) : l'état d'un processus par système, l'inconnu qui ne vaut pas « mort », la fiche et le détachement par plateforme, l'arrêt propre par console sous Windows, le jeton par `getrandom` (pas de `/dev/urandom` sous Windows), l'hôte détaché qui n'hérite plus des tuyaux du client. Preuve, dite telle qu'elle est : l'épreuve du cycle du démon (`tools/build-images/epreuve_demon.py`, branche paquet-npm-2, runners lancés par L-Defraiteur) — **macOS vert sur tout le cycle** (run 38236456277) ; **Windows** : fiche, `/sante` au pid, client attaché, `/arret`, fiche retirée, tuyaux du client fermés et hôte détaché qui survit, verts (runs 38236456277 et 38242681341) ; seule la réouverture d'une base existante rougit, un défaut du moteur sous Windows, hors de ce ticket : `2026-10-12-sous-windows-une-base-existante-ne-se-rouvre-pas-en-ecriture.md` (la garde B2 et le fichier de données se verrouillent l'un l'autre par `LockFileEx`). Sous Linux, les e2e du cycle du démon et l'épreuve (18 OK).
- **Gravité** : blocage. Sous Windows, le binaire ne se bâtit plus. Sous macOS, le mode démon
  ne s'attache jamais à un hôte vivant (lecture du code, non joué).
- **Atteignable en service** : oui, sur tout poste Windows ou macOS, dès le bâti pour l'un et
  dès le premier client MCP pour l'autre
- **Touche rag3weaver** : oui, et seulement (`src/daemon_lifecycle.rs`, `src/daemon_host.rs`,
  `src/bin/rag3weaver-backend.rs`)

## Ce que c'est

Le cycle de vie de l'hôte (marche 4 : `b431294fe`, `2b1818d68`, `e9387d545`) emploie des API
Unix sans garde de plateforme, et lit `/proc`, qui n'existe que sous Linux.

## Ce qui casse, plateforme par plateforme

**Windows** : `rag3weaver` (lib) ne compile plus. Run GitHub `38148452598`, job `windows-x64`,
sur `93f406221` de la branche `paquet-npm`, qui porte master jusqu'à `d61455f10` :

```
error[E0433]: cannot find `unix` in `os`      --> src\daemon_lifecycle.rs:105:18
    use std::os::unix::fs::OpenOptionsExt;
error[E0599]: no method named `mode` found    --> src\daemon_lifecycle.rs:113:10
error[E0433]: cannot find `unix` in `os`      --> src\daemon_lifecycle.rs:245:14
    std::os::unix::process::CommandExt::process_group(&mut commande, 0);
```

Après ces trois erreurs, le binaire tombera aussi : le gestionnaire SIGTERM de l'hôte
(`src/bin/rag3weaver-backend.rs`, mode `daemon`) emploie `tokio::signal::unix` sans
`#[cfg(unix)]`. Celui de `mcp`, juste au-dessus, a sa garde.

**macOS** : tout compile, mais `pid_alive` lit `/proc/<pid>/stat` et rend donc toujours
`false`. Or `answers` commence par `pid_alive(file.pid)`. Un client ne reconnaît donc jamais
l'hôte vivant de `daemon.json` : il le déclare périmé, efface le fichier et lance un second
hôte. `process_start` rend toujours `None`, si bien que les sessions des clients morts ne sont
jamais relevées. Ce dernier point est dit dans le code (« un pid sans /proc lisible ne compte
pas »), mais pas le premier. La macOS de l'archive est verte, car son épreuve ne joue pas le
mode démon.

## La recette

- **Windows** : `cargo check --features rag3db-native,code,daemon,openai-llm` sur un runner
  Windows, ou `cargo check --target x86_64-pc-windows-msvc` si la chaîne croisée est là.
- **macOS** : le cas 1 de `tests/e2e_mcp_cycle_de_vie.rs` sur un Mac. Le client devrait
  s'attacher à l'hôte qu'il a lancé, puis un second client au même hôte (cas 2).

## Le témoin

Il manque. Il faudrait les e2e du cycle de vie joués sous macOS, et un `cargo check` Windows
dans la CI du paquet. Le workflow `paquet-npm.yml` bâtit déjà Windows : c'est lui qui a vu le
défaut.

## Ce qu'il faut pour le fermer

- `#[cfg(unix)]` sur `mode(0o600)` et `process_group`. Sous Windows, il faut un équivalent ou
  le refus nommé du mode démon :
  - détacher l'hôte par `creation_flags` (`CREATE_NEW_PROCESS_GROUP`, `DETACHED_PROCESS`) ;
  - le droit du fichier par l'ACL du profil.
- Le gestionnaire SIGTERM du mode `daemon` sous `#[cfg(unix)]`, et sous Windows un équivalent
  (Ctrl-Break) ou rien, dit.
- `pid_alive` et `process_start` portables :
  - macOS : `kill(pid, 0)` et `proc_pidinfo` ;
  - Windows : `OpenProcess` et `GetProcessTimes` ;
  - à défaut, `answers` ne doit pas exiger `pid_alive` là où il ne sait pas répondre. `/sante`
    rend déjà le pid.
- Un témoin par plateforme dans la CI du paquet.
