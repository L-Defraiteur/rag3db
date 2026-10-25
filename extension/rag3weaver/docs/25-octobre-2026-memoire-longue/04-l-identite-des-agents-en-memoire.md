# L'identité des agents en mémoire — un design

Demandé par Lucie le 25 octobre 2026 au soir, cadré par c7, tranché ici ; complément de la
page 03. Rien n'est codé : la page attend son mot.

Le problème vécu : les sessions se renomment (G s'affiche « optimiseur », hotfixs « cœur
C++ / hotfixs / tickets », moi « mémoire » après « rag3db-96 »), la référence du harnais change
à chaque lancement (ma socket est passée de 1249035 à 1712882 dans la journée), et après une
compaction l'agent lui-même ne sait plus bien qui il est ni qui lui écrit.

## 1. Quatre mots, et la mémoire qui n'appartient à personne

Le vocabulaire que Lucie retient, en anglais dans les identifiants :

| mot | ce que c'est | dans le modèle |
|---|---|---|
| `agent` | l'identité : ses objectifs, son état, ses capacités (son manifeste), et **la mémoire qu'il cite** | une fiche d'identité, à clé stable |
| `session` | le contexte d'exécution temporaire de cet agent ; il meurt et renaît à la compaction | les handles du moment, le journal des runs, la fiche de la tâche en cours |
| `runtime` | Claude Code, rag3weaver ou autre ; **jamais l'identité** | une rubrique du L1 `identity`, rien de plus |
| `worktree` | l'espace de travail versionné, la branche de la tâche ; indépendant de l'identité | ce que l'agent *tient*, cité `path:` |

**La mémoire n'appartient à aucun agent** (Lucie : « rien ne dit que la mémoire soit
forcément celle de l'agent h24, les sujets peuvent être partagés »). Elle est commune ; chaque
apport porte `agent:<clé>` et sa date ; une fiche d'identité **cite** la mémoire, elle ne la
contient pas. Deux mémoires qui tiennent le même sujet se le disent (§7).

## 2. La clé existe déjà

c7, 73, G, 96 sont les noms courts que les sessions se donnent entre elles depuis des
semaines, et ils n'ont pas bougé quand les noms affichés ont bougé. **La clé est ce nom
court**, choisi une fois à la création, en `[a-z0-9-]+`, jamais la référence du harnais.

## 3. Une fiche ordinaire, trois genres, aucun champ

La fiche d'identité est une Fiche du gabarit memory, rien d'autre : `name` est **la clé**
(unique, c'est le nom par lequel `fiche:<clé>` et `put_fiche` la retrouvent, donc il ne change
jamais), `about` dit en une ligne ce que l'agent est, `state` vit (`current`, `retired` quand
l'agent est dissous). Elle est à part de la fiche de session : un agent traverse des tâches,
et chaque fiche de tâche cite `agent:<clé>` dans ses apports.

| genre | valeur | posé sur | sens |
|---|---|---|---|
| `agent` | la clé | l'ancre de la fiche d'identité, et les apports qui parlent de l'agent | la même forme que `outil:<nom>` : `cited_by agent:<clé>` rend la fiche en tête, puis tout ce que l'agent a fait, tâche après tâche |
| `alias` | un nom affiché | un apport `alias` de la fiche d'identité | « depuis telle date, je m'affiche ainsi » |
| `handle` | une adresse du harnais | un apport `handle` de la fiche d'identité | où l'agent se joint en ce moment |

- **Un alias est un L1, pas seulement une Ref.** Une Ref est partagée (citer deux fois la même
  valeur ne fait qu'une ligne), donc sa date n'appartient à personne : deux agents nommés
  « optimiseur » à des moments différents partageraient la même Ref. Le L1 de section `alias`
  porte la date, l'auteur et la raison du renommage, cite `alias:<nom>`, et ne se réécrit pas.
  L'alias courant est le dernier L1 `alias`. « Qui est qui » est une vue : `cited_by
  alias:optimiseur` rend les fiches d'identité qui ont porté ce nom, la plus récente d'abord.
- **Un handle est un L1 de la même forme**, et c'est là que vit la *session*. Pour une
  session Claude Code : `session:<uuid>` (l'identifiant de sa transcription, qui survit aux
  relances par `--resume`) et `uds:<socket>` (qui change à chaque lancement) ; pour un
  équipier : `task:<nom>` et sa boîte `mailbox.<task>`. Le dernier L1 `handle` dit où joindre
  l'agent aujourd'hui, les précédents disent par où il est passé.

## 4. Où la clé est écrite sur le disque

- **La fiche est déjà un fichier** (`fiches/<uuid>.md`, page 04 du 18 octobre) : elle survit à
  l'arrêt du poste et revient de son dossier si la base se perd. C'est la vérité.
- **Un équipier** écrit sa clé dans `<state_dir>/identity.json` : `{ "agent": "<clé>",
  "memory": "<backend mémoire>" }`. Relancé, il lit ce fichier, puis `get_fiche <clé>`.
- **Une session Claude Code** n'a pas de `state_dir` à elle, et le dossier de mémoire du projet
  est **partagé par toutes les sessions du même dépôt** sur ce poste : c7, 73 et moi lisons le
  même `MEMORY.md`. La clé n'y a pas sa place comme vérité. Ce qu'une session a d'elle seule,
  c'est l'identifiant de sa transcription, que le harnais lui redonne après chaque compaction :
  elle se retrouve par `cited_by handle:session:<uuid>`, qui rend sa fiche et sa clé. La ligne
  « qui est qui » de `MEMORY.md` reste utile comme **vue recopiée**, jamais comme source.

## 5. On s'adresse par la clé

- **Entre sessions** : avant d'envoyer, `whois { agent }` (un Plan de rendu, §8) rend le
  dernier handle et le dernier alias ; le message part à l'adresse rendue. Un message signé
  commence par la clé (« 96 → c7 »), ce que les sessions font déjà à la main.
- **La boîte de l'équipier** : `from` et le destinataire d'une ligne sont des clés ; l'hôte
  résout une clé vers une tâche par le dernier `handle` `task:` de sa fiche. Un message à une
  clé sans fiche est refusé en le disant, il n'est pas deviné.
- **Les apports qui parlent d'un agent** citent `agent:<clé>` : le bilan du juge pour l'agent
  jugé, le plan de l'équipier pour celui qui l'a écrit, un apport de session pour celui qui a
  donné un mandat. Le champ `author` d'un apport (M7) prend la clé pour un agent ; un humain
  qui écrit en a une aussi (`lucie`), avec sa fiche, de la même forme.

## 6. La compaction : l'apport `identity`

Après chaque compaction, **au premier tour**, l'agent écrit un L1 de section `identity` sur
sa fiche. Il l'écrit aussi quand il se renomme ou reçoit un mandat nouveau. Sa forme est fixe,
six rubriques, pour que le juge compare deux L1 rubrique à rubrique :

```
## identity
who: 96 — « mémoire », la session de la mémoire longue de rag3db
runtime: claude-code ; session:25b7ff4b-… ; uds:…/1712882.sock
can: les outils MCP memoire-longue et code-rag3db ; push sur luciepc par le tour
doing: la migration de la fiche de Lucie (attend c7) ; la page 04
holding: worktrees wt-docs-96, wt-mf-96 ; luciepc rag3db-p1-96, p1r-96 ; branches lot-96-*
mandates: chaque commit poussé dans la minute ; master après le mot de c7 ; jamais de force
peers: c7 « orchestration sessions » ; 73 « arbre principal » ; G « optimiseur »
```

Il cite `agent:<clé>` pour chaque pair, `fiche:<nom>` pour chaque tâche ou phase tenue,
`path:` pour chaque worktree, et le `handle:` courant. Pour un équipier, l'hôte l'écrit
lui-même à chaque frontière entre deux pas (il a le plan, la boîte, le manifeste et l'alias
sous la main) ; la compaction par les fiches (équipier-2) repart de là. Pour une session
Claude Code, c'est la parole de l'agent ; le crochet `PreCompact` de M9 range la
transcription, il n'écrit pas à sa place.

**Le juge le note comme un run.** Il lit le L1 `identity` neuf contre le précédent et écrit un
apport `trend` sur la fiche d'identité : une contradiction (`holding` perd un worktree sans
qu'un apport dise qu'il a été effacé), un lot oublié (une tâche citée avant, absente
maintenant, sans clôture), un alias manquant (un pair nommé sous un nom qu'aucun L1 `alias` ne
porte). Il ne change aucun état, et c'est l'agent qui corrige, ou son superviseur.

## 7. Deux mémoires, un sujet : la proposition de fusion

Lucie, mot pour mot : « tiens je propose de merger mon sujet machin chez toi, il existe déjà
aussi, es-tu ok avec le merge ? — pas l'agent qui dit ça, automatique ». Deux bases (deux
agents, deux postes) tiennent une fiche du même sujet. C'est M4 qui porte la détection, par
**une réaction déclarée**, de la famille de `review_anchored` : elle écoute les changements de
Fiche et compare aux mémoires **voisines** que le manifeste déclare (`peers` : d'autres
backends du même hôte, ou un dossier de fiches arrivé par git, puisque les fiches sont des
fichiers et que M10 les partage ainsi).

- **La clé d'égalité : le nom stable d'abord, les ancres ensuite, les deux dites.** Deux
  fiches du même `name` sont le même sujet jusqu'à preuve du contraire : la proposition part.
  Deux fiches de noms différents qui partagent au moins une ancre d'un genre qui désigne une
  chose (`path`, `symbol`, `outil`, `agent`, `fiche`) sont des candidates : la proposition part
  aussi, et dit sur quoi elle se fonde (le nom, ou les ancres communes). `doc:` et `alias` ne
  comptent pas : trop partagés, trop vagues. Rien ne se fond sans réponse.
- **La proposition est le mécanisme de la page 03 §5**, pas une parole d'agent : un apport de
  section `proposal` sur la fiche cible, `via: merge-detector`, qui cite `same_as:<mémoire>/<nom>`
  (la fiche source, dans sa base) et dit la clé d'égalité. Elle part au **propriétaire de la base
  cible** : la clé que son manifeste déclare (`owner: agent:<clé>` ; à défaut, le superviseur de
  l'hôte), jointe par `whois`.
- **Acceptée** : les apports de la source se copient sur la cible **avec leur provenance** :
  `author` et `created_at` inchangés, `via` qui dit la base d'origine, leurs Refs reposées. Un
  L1 ne se réécrit pas, il voyage. Puis M5 refait le L2 de la cible depuis ses L1, les anciens
  et les arrivés. La source reste ce qu'elle est : c'est à son propriétaire de la retirer ou de
  la lier.
- **Refusée** : les deux fiches restent, liées par une Ref `same_as` **dans les deux sens**
  (chaque fiche ancre `same_as:<autre mémoire>/<autre nom>`), et la recherche les rend ensemble
  sans les mêler : `recall_fiches` groupe par `same_as`, chaque fiche gardant son L2. Une
  proposition refusée ne revient pas pour la même paire tant qu'aucune des deux ne change.

Les Refs `same_as` nomment une fiche d'une autre base : leur harnais rhai met la valeur en
forme, et c'est le Plan de rendu qui dit si la base voisine répond encore, comme pour `fiche:`
(page 03 §6).

## 8. Où ça vit, et ce qui s'ajoute

Dans le gabarit memory. Les identités vivent dans **la mémoire commune du projet** (celle où
le juge range, `bilans.memoire`) : c'est là que les bilans et les plans les citent, et §7 vaut
entre elle et toute autre. Ce qui s'ajoute, aucun changement de schéma, format des fichiers
inchangé :

1. les genres `agent`, `alias`, `handle`, `same_as`, par `create_ref_type` ;
2. la convention des L1 `alias`, `handle`, `identity` et `proposal` ;
3. un Plan de rendu `plans/whois.json` et son outil `whois { agent | alias | handle }` : la
   clé, le dernier alias, le dernier handle, les alias passés avec leurs dates ;
4. pour l'équipier : `identity.json` dans son `state_dir`, la résolution des clés dans la
   boîte, le L1 `identity` écrit à la frontière ;
5. pour le juge : la lecture d'un `identity` contre le précédent (lot du juge) ;
6. pour M4 : la réaction `merge-detector`, `peers` et `owner` dans le manifeste, l'acceptation
   qui fait voyager les L1, le refus qui pose `same_as` ; pour `recall_fiches`, le groupe.

Les sessions Claude Code s'inscrivent **sans code** : la fiche se crée par `put_fiche`, les L1
par `put_apport`, par les outils MCP qu'elles ont déjà.

## 9. Les témoins

- **L'identité**, un équipier `t1`, alias « aide », sur faux modèle : il se renomme « équipier
  doc », `cited_by alias:aide` rend encore `t1`, `whois t1` rend les deux noms datés ; une
  ligne de boîte adressée à `t1` arrive à sa tâche, une ligne adressée à `t9`, sans fiche, est
  refusée en le disant ; l'hôte s'arrête et repart, l'équipier lit `identity.json`, retrouve
  sa fiche, reçoit son dernier `identity` à la première frontière et en écrit un neuf, et le
  juge n'y voit aucune contradiction.
- **La fusion**, deux bases dans un hôte, propriétaires `t1` et `t2` : une fiche « régime GPU »
  dans chacune. La réaction émet la proposition sur la cible, adressée à `t2`, qui dit « même
  nom ». Acceptée : la cible porte les apports des deux, chacun avec son `author` et sa date,
  et son L2 est refait. Une seconde paire, aux ancres communes et aux noms différents : la
  proposition dit les ancres ; refusée, `same_as` est posé des deux côtés et `recall_fiches`
  les rend groupées, deux L2 distincts. Puis la mémoire reconstruite de ses fichiers garde
  identités, alias, handles et `same_as` (`e2e_reconstruction`, déjà générique).

## 10. Trois questions pour Lucie

- Les humains ont-ils une fiche d'identité de la même forme (`lucie`), pour que `author` et
  `agent:` parlent d'une seule voix ? Je propose oui.
- La clé d'un équipier : choisie par le superviseur dans la demande, ou engendrée par l'hôte
  (`teammate-<n>`) ? Je propose la demande d'abord, l'hôte à défaut.
- Une proposition de fusion fondée sur les ancres seules part-elle tout de suite, ou seulement
  quand les deux fiches partagent aussi un mot de leur nom ? Je propose tout de suite, en le
  disant : c'est au propriétaire de refuser, pas à la réaction de se taire.
