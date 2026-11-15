# Un push de docs annule le run de code en attente : la CI de master ne joue que dans un creux

- **État** : fermé (15 novembre 2026, 18 h 31, hotfixs) — corrigé par 6c59d5c78. La preuve est en fin de page.
- **Famille** : moteur
- **Gravité** : la CI ne dit rien. Un commit de code reste sans verdict tant qu'un push de docs le suit dans les vingt
  minutes.
- **Atteignable en service** : non (contrôle de CI).
- **Touche rag3weaver** : oui, par son workflow, que ses docs déclenchaient de même.
- **Pour** : hotfixs

## Ce qu'on voit

GitHub ne garde qu'un run en attente par groupe de concurrence, le plus récent, et sur master le run qui tourne va au
bout (`cancel-in-progress` faux). Chaque push annule donc le run qui attendait. Les docs se poussent toutes les
quelques minutes (tickets, pages, orchestration), et un run du « CI Workflow » dure une vingtaine de minutes : le run
d'un commit de code est presque toujours remplacé avant de jouer. Le 15 novembre, les runs sur 9ea2e2408 (le format)
et sur a8b3b25bb sont « cancelled » sans job.

## Le correctif

- « CI Workflow » : `paths-ignore` sur le déclencheur `push`, avec `docs/**`, `extension/rag3weaver/docs/**`,
  `extension/rag3weaver/visions/**` et les `*.md` de la racine.
- Workflow `rag3weaver` : ses `paths` gagnent `!extension/rag3weaver/docs/**` et `!extension/rag3weaver/visions/**`.
- **Pas de `**/*.md`** : des `.md` de `extension/rag3weaver/templates/` sont compilés dans le binaire
  (`include_str!`, `claude_code_hook.rs`, `render_nodes.rs`). Un changement de gabarit doit garder son run.

## Pour fermer

Un run de master sur un commit de code qui joue ses jobs jusqu'au bout, alors que des pushes de docs seules l'ont suivi
pendant qu'il tournait ou attendait.

## La fermeture (hotfixs, 15 novembre 2026, 18 h 31)

- **Un push de docs seules ne lance plus rien.** 3b4ba3cb7 (c7, 17 h 51) n'a lancé ni CI Workflow ni rag3weaver.
  Après f85977d09, dix commits de docs seules sont venus sans créer de run.
- **Un run de code joue jusqu'au bout malgré des docs derrière lui.** Le run 43739124130 sur f85977d09 (le lot
  service-503 d'embarquement) est vert : sanity checks, python lint check, code format et shell test.
- **Le run du lot lui-même**, 43737760344 sur 6c59d5c78, a été remplacé, mais par un push de code (f85977d09). C'est
  le comportement voulu : le code le plus récent passe devant.
- **Les deux runs créés ensuite** (43740369684 sur e9e47391a, 43741129343 sur 677d361ad) viennent de pushes qui
  portaient du code de tty et de l'hôte, sous un dernier commit de docs. GitHub juge le filtre sur les fichiers de tout
  le push, pas sur le dernier commit.
