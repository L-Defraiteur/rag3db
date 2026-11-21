# Le temps dans les graphes : `postpone`, le planificateur, et les pièces de la porte

21 novembre 2026, session everything declarative. Une page de conception, avant tout code, accordée avec archi.

Le contexte, côté archi, est la porte du mode auto en graphe déclaré : `docs/rag3weaver-archi/21-11-2026/03`, le
mode auto contextuel, et `04`, le survivant. Sa page 03 renvoie à celle-ci pour tout ce qui touche au temps.

Lucie, à 10 h 5x : « ça ne nous donne pas une justification pour des nœuds de cron ? genre postpone → othernode ».

## Pourquoi un nœud qui reporte, et pas un nœud qui attend

Le moteur de graphes ne sait pas tenir une attente longue :
- **l'échéance d'un appel n'est que coopérative** : seul un nœud qui la lit s'arrête ;
- **un niveau attend tous ses nœuds**, dans une portée bornée (`std::thread::scope`) ;
- **un graphe qui attend 600 s** tient un fil et le backend pendant tout ce temps.

La porte devient donc une machine à évènements, où **aucun graphe n'attend** :
- `gate/decide.mmd` est court et se joue à chaque demande. Il rend `pass`, `refuse` ou `card{stage}`.
- `gate/on_event.mmd` se joue à chaque évènement d'une carte ouverte. Il rend l'étape suivante, ou la fermeture.
- **La carte** (archi, en Rust, côté hôte) tient l'état et **une seule fermeture** : le premier évènement qui ferme
  gagne, et les suivants sont journalisés sans effet.
- **Le temps** (30 s, 600 s) est porté par des réveils, posés dans le graphe par `postpone` et tenus par un
  planificateur.

Le même planificateur porte le **cron** du socle de l'équipier : un graphe chaque nuit, un agent qui dit « je repasse
dans une heure ». Ce n'est pas dans ce lot, mais la forme le permet dès maintenant.

## Le nœud `postpone`, déclaré

```
postpone["Postpone(after_s=30, resume='gate/on_event.mmd', key='card:$card:user_wait', event='user_wait_elapsed')"]
```

- **Quand** : une seule de ces trois formes, sinon le gabarit est refusé au chargement :
  - `after_s` : dans N secondes ;
  - `at` : à une heure dite, en RFC 3339 ;
  - `cron` : selon une expression cron à cinq champs, en heure locale du poste.
- **Où reprendre** (`resume`) : un graphe du même gabarit, par son chemin, avec l'entrée qu'il recevra au réveil.
  Pour la porte, cette entrée est `{card, event, at}`.
- **Sous quelle clé** (`key`) : l'unicité se juge par la clé. Reposer la même clé **remplace** le réveil, et
  l'annulation se fait par clé ou par préfixe.
- **Sa sortie** : `{postponed: {key, when, resume}}`. Le graphe s'arrête là, et c'est l'hôte qui le réveillera.

`postpone` ne tient aucun état métier. L'état de la carte reste à la carte ; un graphe repris le relit par elle.

## Le planificateur, une primitive Rust de l'hôte

1. **Sur disque** : chaque réveil est un fichier,
   `<état de rag3weaver>/scheduler/<clé échappée>.json`, qui contient `{key, when, resume, input, cron?, generation}`.
   Il est écrit à part puis renommé, pour qu'un lecteur ne voie jamais un fichier à moitié écrit. Le dossier d'état est
   celui de `builtin_templates::state_root()`, hors de tout worktree d'agent.
2. **Il survit au redémarrage** : au démarrage, l'hôte relit le dossier. Un réveil passé se joue tout de suite, une
   seule fois.
3. **Il est idempotent** : un réveil tiré passe d'abord à `fired` (sa génération, renommée), puis le graphe est joué.
   Tiré deux fois, à cause d'une course ou d'un redémarrage entre les deux pas, il ne joue le graphe qu'une fois. Un
   `cron` se réarme à l'échéance suivante, avec une génération neuve.
4. **Il s'annule** :
   - `cancel(key)` retire le réveil de cette clé ;
   - `cancel_prefix(prefix)` retire tous ceux dont la clé commence par le préfixe. C'est ce qu'appelle la carte à sa
     fermeture (`cancel_prefix("card:L12:")`), sans tenir la liste de ses clés ;
   - les deux sont appelables depuis le Rust, hors d'un graphe, et sans effet sur un réveil déjà tiré : c'est la
     carte qui juge un évènement arrivé après sa fermeture.
5. **Il se laisse lire** : la liste des réveils (clé, quand, graphe repris), pour l'IDE et pour `/etat`.
6. **Un seul fil** de l'hôte dort jusqu'au prochain réveil et se réveille à toute pose ou annulation. Les graphes
   repris partent en fond (`dataflow::background`), jamais sur ce fil.

## Les trois pièces de la porte

1. **Le délai d'un appel de modèle** (`timeout_s`, sur `LlmNode` et `Decide`). Il sert au juge et au survivant, des
   appels bornés, sans rapport avec `postpone`.
   - Le nœud qui le porte tourne **hors de la portée du niveau**, dans un fil détaché : un fil Rust ne se tue pas.
   - À l'échéance, le niveau prend la sortie déclarée `timeout` et continue.
   - Le fil abandonné finit seul, et sa réponse tardive est jetée. Pour qu'il meure à peu près en même temps, le client
     HTTP de l'appel prend le même délai.
2. **Une politique de types fermée** (`NodeTypePolicy::only`) pour `gate/decide.mmd` et `gate/on_event.mmd` : les
   nœuds de politique (la table, `Decide`, `LlmNode`, `Postpone`, les nœuds scriptés de la table) et les primitives de
   garde (`RuleClass`, `RootGrants`, `ResolveEvidence`, `Card`). **Aucun nœud d'écriture, de commande ou de réseau.**
   Un autre type est refusé au chargement, en le nommant.
3. **Le chargement depuis `~/.config/rag3weaver/gate/` seulement.**
   - Le chemin résolu (`canonicalize`) doit rester sous ce dossier ; un lien symbolique qui en sort, vers un worktree par
     exemple, est refusé en le nommant.
   - Le dossier est hors des racines et des worktrees de tout agent, et **le bac ne le monte jamais en écriture**.
   - L'empreinte des deux graphes et de leurs fichiers cités est dite à l'agent au démarrage de sa tâche, et elle est
     notée dans `/etat`.

## Les témoins, chacun rouge d'abord

- **`postpone`** : un graphe qui pose un réveil à 1 s est repris une fois, avec son entrée. Reposer la même clé
  remplace le réveil. `cancel` et `cancel_prefix` l'empêchent de partir.
- **Redémarrage** : un réveil posé, l'hôte arrêté, l'échéance passée ; au redémarrage, le graphe est repris une fois.
  Tiré deux fois (deux hôtes ou deux fils), il n'est joué qu'une fois.
- **`timeout_s`** : un faux modèle qui répond en 5 s, sous `timeout_s=1`. Le niveau rend `timeout` en environ 1 s, et
  la réponse tardive ne change rien.
- **La politique fermée** : un `RunCommandNode` dans `gate/decide.mmd` est refusé au chargement, en le nommant.
- **Le chargement** : un `gate/` qui est un lien vers un worktree est refusé ; l'empreinte dite change quand un graphe
  change.
- **Côté archi**, dans `teammate_tests/card.rs`, sur la tête de ce lot : la réponse de l'utilisateur avant 30 s
  annule le réveil, et `on_event` n'est jamais joué pour `user_wait_elapsed` ; un réveil arrivé après la fermeture ne
  ferme rien une seconde fois.

## Ce qui reste à décider

- **La forme du cron** : cinq champs en heure locale, ou une bibliothèque qui accepte plus (secondes, fuseaux) ? Je
  propose cinq champs, la forme qu'on lit partout.
- **Qui pose un réveil** : un graphe de la porte, oui ; un agent, pour le cron du socle, seulement par un outil borné
  (le nombre de réveils et leur fréquence), jamais en écrivant le dossier du planificateur.
