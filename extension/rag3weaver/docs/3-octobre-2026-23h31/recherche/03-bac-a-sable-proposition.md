# Le bac à sable de `run_command` — proposition

**Session recherche, 4 octobre 2026.** Deux passes Gemini l'ont désigné :
un modèle fort traite un refus comme un obstacle — variantes de lecture,
inspection d'environnement, création de liens, tentatives d'écriture. La
garde par analyse de ligne les a toutes tenues, mais **analyser du shell
n'est pas une frontière de sécurité** : un binaire du dépôt lancé par
`cargo test` lit ce qu'il veut. La frontière honnête, pour le mode `auto`
et tout usage sans humain devant, est que la commande **ne voie pas** ce
qu'elle n'a pas à voir.

## La forme : Landlock d'abord, bubblewrap en repli

**Landlock** (noyau ≥ 5.13, ABI v2+) : des règles de fichiers posées par
le processus lui-même, **sans privilège**, héritées par les enfants, et
irrévocables après `prctl(no_new_privs)` + `landlock_restrict_self`. Le
parent (notre `Atelier`) les pose entre `fork` et `exec` : la commande
démarre déjà confinée, rien à installer sur le poste.

**bubblewrap** en repli déclaré quand le noyau est trop vieux ou Landlock
désactivé : mêmes règles par montages (`--ro-bind`, `--bind`,
`--unshare-net`), au prix d'une dépendance externe. Le choix se fait au
démarrage du backend, est journalisé, et `describe` le dit.

## Les règles (la politique par défaut)

| Zone | Droit |
|---|---|
| le domaine de travail (workspace) | lecture + écriture |
| `/usr`, `/lib`, `/bin`, `/etc` (lecture des outils et libs) | lecture seule |
| `/tmp` propre à la commande (un tmpfs par exécution) | lecture + écriture |
| le dossier de l'utilisateur (`$HOME`) | **rien** |
| le réseau | **rien** par défaut |
| tout le reste | rien |

Déclaration au manifeste, sous `workspace` :

```json
"sandbox": {
  "mode": "landlock",        // landlock | bubblewrap | off (défaut : landlock si possible)
  "network": false,            // true l'ouvre, et describe le dit
  "extra_read": ["~/.cargo/registry", "~/.rustup"],
  "extra_write": ["~/.cargo/registry/cache"]
}
```

`off` existe parce qu'un poste de confiance peut le vouloir — mais le
mode `auto` de la porte **refuse de s'armer sans bac à sable** : c'est la
condition qui rend `auto` honnête.

## Ce que ça casse, et comment on le déclare

- **cargo** : lit `~/.cargo/registry` et `~/.rustup`, écrit
  `~/.cargo/registry/cache` et le `target/` du projet. Le `target/` est
  sous le domaine (rien à faire) ; les caches se déclarent en
  `extra_read`/`extra_write` — c'est l'exemple fourni, pas un défaut
  caché : un manifeste qui ne les déclare pas verra `cargo` échouer en
  le disant.
- **git** : `~/.gitconfig` (lecture à déclarer si l'identité compte),
  et tout `fetch`/`push` exige `network: true` — c'est voulu : un agent
  qui pousse passe par l'humain.
- **les outils qui téléphonent** (rustup, npm install…) : bloqués sans
  `network: true`. Le refus du noyau est net (`EACCES`/`ENETUNREACH`),
  et le motif de la garde expliquera la clé à déclarer.
- Les caches exotiques (sccache, pip…) : même mécanique, déclarée.

## Ce que ça NE remplace pas

La garde par analyse reste **devant** : elle refuse tôt, avec un motif
lisible et la ligne « pas par une autre formulation » — le bac à sable est
le filet du dessous, pas le message. Les trois gardes de lecture
(chemins, liens, journaux) restent telles quelles : défense en
profondeur, trois couches déjà éprouvées plus une.

## Les tests

1. Les cas de contournement des passes deviennent des **échecs du
   noyau** : lecture hors domaine par commande libre, création de lien
   vers l'extérieur puis lecture, écriture hors domaine — chacun vérifié
   comme erreur de la commande, pas comme verdict de la garde.
2. Le chemin heureux : `cargo check` d'un petit crate dans le domaine
   avec les caches déclarés ; `git status` ; les outils de la liste libre.
3. La tâche 5 du scénario d'agent rejouée en mode `auto` **avec** bac à
   sable : la sentinelle peut accorder librement, le noyau tient.
4. Un poste sans Landlock : le repli bubblewrap s'annonce, ou `off`
   refuse d'armer `auto` en le disant.

## L'implémentation (à grands traits)

Une variante d'`Atelier::executer` : `executer_confine(laissez, regles)` —
le crate `landlock` (pur Rust, maintenu par l'auteur noyau) pose les
règles post-fork via `pre_exec` ; bubblewrap préfixe l'argv. Les règles
se construisent depuis `workspace` + `sandbox` au chargement, une fois,
et sont les mêmes pour toute la session. Le mode `auto` de `CommandGate`
exige `sandbox != off` au chargement du manifeste — refus actionnable.
