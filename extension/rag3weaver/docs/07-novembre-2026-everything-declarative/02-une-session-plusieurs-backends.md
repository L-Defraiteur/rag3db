# Une session, plusieurs backends

7 novembre 2026, everything declarative. La question de Lucie : « je comprends pas pourquoi une
session doit être limitée à un backend, pourquoi c'est pas l'agent qui les explore ? », et avant :
« après je veux voir ça en mode worktrees : une session prend un lot de worktrees sur plusieurs
repos ». Pas un lot ce soir ; la réponse, et par où commencer.

## 1. Pourquoi aujourd'hui un chat = un backend

Rien de profond : c'est l'endroit où chaque pièce a été rangée, une à une.

- **La session est rangée par backend** : l'hôte tient `chats: id du backend → session`, le fil se
  garde sous `<racine des chats>/<id>`, la route est `/ide/code/<id>/chat`.
- **Les outils sont ceux d'un manifeste** : un tour construit sa boîte depuis `b.prepared()`
  (`OutilsDuChat::de`, `DeclarationTools`, `BackendTools`) — les outils attachés à *ce* backend,
  appelés par `Backend::call` sur *sa* base.
- **Le modèle et la politique se lisent dans ce manifeste** : `models.llm`, `declarations`,
  `teammate`, `bilans`, la porte des commandes du workspace.
- **Une tâche est un worktree du dépôt qui porte ce manifeste** (`ouvrir_une_tache(parent, nom)`),
  avec la copie de sa base ; la boîte de l'équipier est celle de cette tâche.
- **La mémoire est un autre backend**, atteint par côté : le juge y écrit (`bilans.memoire`), et
  chez Claude Code le hook parle au backend « memory » à part. Le chat lui-même ne la lit pas.

Claude Code fait déjà l'autre forme : ses serveurs MCP (`code-rag3db`, `memoire-longue`) sont des
backends distincts, et c'est l'agent qui choisit lequel appeler. Notre chat n'a simplement jamais
eu qu'une boîte d'outils.

## 2. Ce que serait une session à plusieurs backends

- **Un manifeste de session** (déclaratif, comme le reste) : ses backends (`{alias, manifest}` :
  le code de plusieurs dépôts, la mémoire longue…), ses worktrees (un par dépôt, s'il travaille
  dans une tâche), son modèle, sa politique d'équipier. Une session devient une entité à elle, avec
  son id, sa route `/ide/session/<id>`, son fil et sa boîte.
- **Une seule boîte d'outils, union des backends** : les outils d'un backend secondaire préfixés
  par son alias (`memoire.recall_fiches`, `lucivy.search_code`) ; les outils de fichiers résolus
  par le chemin (le worktree qui le contient), si bien que `read_file` reste un seul outil.
  L'étanchéité du lot 1 vaut par backend : un backend qui manque rend ses outils indisponibles, les
  autres continuent.
- **Un seul fil, une seule boîte** pour la session ; les sous-équipiers (lot 5 de la page 01)
  peuvent alors porter chacun un sous-ensemble des worktrees.
- **Les worktrees en lot** : « une session prend un lot de worktrees sur plusieurs repos » =
  la session ouvre une tâche par dépôt (une branche, un worktree, une vue de l'index par dépôt
  quand les vues de la vision du 6 novembre seront là), et les fusions restent par dépôt, à un
  humain.

## 3. Ce que ça coûte, ce que ça casse, par où commencer

- **Le journal des runs et le juge** sont rangés par backend (`juger(backend_id, run)`, le
  journal par backend) : il faut un journal par session, chaque appel d'outil portant le backend
  qu'il a touché ; le juge lit le run de la session.
- **L'IDE ouvre un backend** : la vue du chat, la barre, les runs par rôle se lisent par backend ;
  la vue de session les agrège (le flux porte déjà le `backend` de chaque trame).
- **La relecture d'une tâche** devient une relecture par dépôt dans la même page.
- **Le prompt grossit** (deux backends ≈ trente outils) : les préfixes et les descriptions doivent
  rester sobres ; c'est ce que Gemini et Claude tiennent déjà chez Claude Code.
- **Rien ne casse si on garde l'ancienne forme comme cas particulier** : une session à un seul
  backend est exactement le chat d'aujourd'hui.

**Par où commencer** (ce que fait déjà Claude Code) : un chat qui a **le backend de code et la
mémoire longue à la fois**. Le manifeste de code déclare déjà sa mémoire (`bilans.memoire`) ; la
boîte du tour y ajoute les outils de lecture de la mémoire, préfixés (`memoire.recall_fiches`,
`memoire.get_fiche`), appelés sur ce backend-là, l'étanchéité comprise. Un lot court, qui ne
change ni la route ni le journal, et qui prouve l'union des boîtes et les préfixes avant le
manifeste de session. Puis le manifeste de session et le journal par session, puis les worktrees
en lot.
