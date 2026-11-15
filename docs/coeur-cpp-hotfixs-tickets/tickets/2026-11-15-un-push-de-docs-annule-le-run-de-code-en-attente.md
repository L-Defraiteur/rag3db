# Un push de docs annule le run de code en attente : la CI de master ne joue que dans un creux

- **État** : ouvert (15 novembre 2026, hotfixs, relevé par c7). Corrigé par le lot `ci-docs-sans-run` ; se ferme
  au premier run de code de master qui joue malgré des pushes de docs derrière lui.
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
