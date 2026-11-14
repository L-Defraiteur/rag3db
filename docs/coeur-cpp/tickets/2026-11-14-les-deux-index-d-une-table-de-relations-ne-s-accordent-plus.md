# Les deux index d'une table de relations ne s'accordent plus

- **État** : ouvert (14 novembre 2026, hotfixs)
- **Famille** : moteur
- **Gravité** : donnée fausse. Une même relation rend d'autres bouts, ou une autre `line`, selon qu'on la lit
  depuis sa source ou depuis sa destination. Des cases à zéro (relation 0, nœud 0) se lisent comme de vraies
  relations.
- **Atteignable en service** : oui. code-lourd et code-principal à luciepc, dans l'état laissé par 0c41f9fe6.
- **Touche rag3weaver** : par le moteur. `usages` et `impact` lisent l'un ou l'autre sens, selon le plan.
- **Signalé par** : G (le contrôle des deux sens de la bascule vers 373b5477b), lu par hotfixs avec c7
- **Pour** : cœur c++. La réparation des deux bases est chez hotfixs (ticket de G
  `docs/optimiseur/tickets/2026-11-14-code-lourd-et-code-principal-divergent-entre-leurs-deux-rangements.md`).

## Ce qu'on voit

Une relation est rangée deux fois, dans le groupe CSR de sa source (sens avant) et dans celui de sa destination
(sens arrière). Sur les instantanés `avant-373b5477b` de code-lourd et de code-principal, trois tables par base
ne s'accordent plus. Le détail des comptes est dans le ticket de G. Ce que hotfixs a lu, relation par relation,
en comparant les deux sens par leur **contenu** (bouts en `_uuid`, propriétés) et non par l'identifiant :

- **Des cases à zéro.** Des lectures portent la relation 0 et partent du nœud 0, ou y arrivent. Le Scope
  d'offset 0 est un fichier `.md` qui ne porte aucune de ces relations : dans code-lourd `df461a4b…`
  (`09-highlighting-architecture.md`), dans code-principal `8e37fb11…`. Le sens arrière les lit comme
  source. Dans code-principal, le sens avant les lit aussi comme destination, sur IMPLEMENTS.
- **Des identifiants consécutifs décalés.** `29:357` à `29:360` (INHERITS_FROM, code-lourd) vont de `11:5359`,
  `11:5358`, `11:5357`, `11:5356` vers `11:5354` dans un sens, avec d'autres bouts dans l'autre. Dans LOCKS
  de code-lourd, la même source et la même `line` (`1689599c…`, ligne 56) vont vers `923f0281…` dans un sens et
  vers `005a5807…` dans l'autre. `005a5807…` est aussi la destination de la ligne 71 d'une autre source.
  Une colonne d'un des deux index est décalée d'un cran par rapport à ses voisines.
- **Le sens avant est le juste, à ses cases à zéro près.** On juge une lecture plausible si ses deux bouts
  sont hors de l'offset 0, si sa ligne tombe dans les lignes de son Scope source, et si le nom de sa cible
  paraît dans le texte de la source. Les lectures plausibles s'ajoutent alors aux relations en accord, à
  une arête par paire. On retrouve **exactement** le nombre de contenus distincts du sens avant, dans les six
  tables : code-lourd 650, 20 093 et 544 ; code-principal 20 210, 328 et 425. Les lectures du sens arrière en
  désaccord sont presque toutes fausses (LOCKS de code-principal : 36 sur 36).

## La piste (cœur, relayée par c7)

Sous 0c41f9fe6, l'annulation d'un COMMIT refusé tronquait le groupe CSR sens par sens
(`InMemoryVersionRecordHandler::rollbackInsert`). Quand un autre écrivain avait versé dans un sens et pas dans
l'autre, les deux index perdaient des lignes différentes. Cela donne exactement des cases à zéro et des
sources décalées d'un cran. Le correctif (a) de 373b5477b vise cette annulation.

Les deux bases ont été écrites en dernier le 14 novembre à 14 h 25, sous 0c41f9fe6.

**Ce qui reste à départager** : depuis quand l'écart existe-t-il ? Le contrôle d'un instantané plus ancien
(`avant-0c41f9fe6`, 13 nov. 20 h 36) le dira. Et le journal de l'hôte de luciepc a-t-il connu des refus du
pool ou des COMMIT refusés sous plusieurs écrivains ? `journalctl --user -u rag3weaver-daemon` depuis le
1ᵉʳ novembre ne rend rien sur « buffer pool », « pool is full » ou « rollback ».

## Ce qu'il faut pour le fermer

1. Un témoin rouge sous 0c41f9fe6 : deux écrivains de relations sur une même table, un COMMIT refusé qui
   s'annule, puis le contrôle des deux sens (`tools/check_rel_directions`) rend un écart.
2. Le même témoin, vert sous 373b5477b.
3. Les deux bases réparées dans la fenêtre d'hotfixs, puis le contrôle des deux sens égal sur les 22 tables.
