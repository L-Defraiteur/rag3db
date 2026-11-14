# La navigation dans les symboles : la couverture des membres

Le ticket `docs/rag3weaver-archi/tickets/2026-11-12-exposer-la-navigation-dans-les-symboles.md`
garde ouverte la couverture étendue. Sur master, les écarts de la revue du 12 sont traités (le
budget commun aux sorties d'ambiguïté, le parent indéterminable rendu en compact, les faits de
membres invalides dits). Reste le mode `members` : il ne lit que `declarations` et `field_types` du
scope demandé, si bien qu'une `struct` Rust ne montre aucune méthode (elles vivent dans `impl T`, un
autre scope) et qu'une classe Python dont les déclarations sont vides n'en montre pas davantage.

## La conception, en quatre lignes

1. Les membres d'un type = ses faits indexés (`declarations`, `field_types`) **plus** les scopes
   qu'il contient (`range_containment`) **plus** ceux des scopes compagnons de même nom dans le même
   fichier (Rust `impl T`, `impl Trait for T`), chacun avec sa provenance.
2. Hors du fichier (méthodes Go à récepteur, définitions C++ hors classe) : par l'appartenance que
   l'index porte déjà quand il la porte, avec sa provenance ; sinon la couverture nomme la limite pour
   ce langage, au lieu d'une liste qui se croit complète.
3. Rien d'inventé : aucun UUID fabriqué ; une méthode à la fois déclarée et contenue n'apparaît
   qu'une fois (même nom, même ligne), avec ses deux provenances.
4. Rouges purs d'abord, sur les lignes du Plan, sans lourd : une `struct` et son `impl` séparé (les
   méthodes de l'`impl` dans les membres de la `struct`) ; une classe Python aux déclarations vides
   (ses méthodes contenues) ; un type Go et une méthode à récepteur (la limite nommée, ou le membre
   par l'appartenance indexée).
