# Le journal d'une transaction reste sans borne pour les insertions ordinaires

- **État** : ouvert — suite de la borne du journal d'un `COPY` (`71cffbc4b`), hors de ce lot (décision de l'orchestration, 5 octobre 2026)
- **Gravité** : refus (« The buffer pool is full ») d'une très grosse transaction d'insertions ordinaires ; pas de perte
- **Atteignable en service** : oui, par une transaction qui insère ou met à jour assez pour remplir le tampon de son seul journal
- **Touche rag3weaver** : le mode où le plein texte vit dans la base (voir plus bas) ; il s'en garde en demandant le point de reprise forcé pour ses paquets
- **Ouvert le** : 5 octobre 2026, session cœur C++
- **Pour** : cœur C++

## Ce que c'est

Le journal d'une transaction vit en mémoire jusqu'à sa validation, en pages que le tampon ne peut pas évincer. Depuis `71cffbc4b`, un `COPY` journalisé qui le porte au-delà de `copy_journal_threshold` replie la transaction sur le point de reprise forcé. Le seuil n'est regardé **qu'aux écritures d'un `COPY`**. Une transaction qui n'écrit que par `CREATE`, `MERGE` ou `SET` — ou qui écrit ainsi après son dernier `COPY` — n'a aucune borne : ses lignes locales sont d'ailleurs écrites au journal **à la validation**, en plus de leur stockage local.

## Le fait mesuré qui le motive

Première indexation du dépôt (7 014 fichiers, 80 001 scopes), `COPY` journalisé, sonde `RAG3DB_PROFILE_JOURNAL`, 5 octobre 2026 (session des embarquements) :

- quand le plein texte vit dans la base, la table `_index_blobs` écrit 849 Mio de journal par insertion (12 025 lignes, 74 Ko par ligne), plus 30 Mio de mises à jour ; les transactions de paquet pèsent 441, 305, 220 et 208 Mio ; la passe perd 30 à 40 s, les blobs étant écrits deux fois (journal, puis pages au point de reprise) ;
- quand il vit en fichiers, la plus grosse transaction n'est pas un paquet mais le chargement final des relations : 158 Mio pour 1 111 300 lignes de relations. Sur un dépôt deux fois plus gros elle franchit le seuil par défaut (256 Mio) et se replie : c'est le comportement voulu.

## Ce qui est décidé pour l'instant

Le défaut du seuil est confirmé : un huitième du tampon, plafonné à 256 Mio. Le mode à blobs demande lui-même le point de reprise forcé pour ses paquets (`force_checkpoint_on_copy=true`), côté rag3weaver : une transaction forcée n'écrit rien au journal, insertions ordinaires comprises.

## Pour le fermer

Regarder le seuil à toute écriture au journal local, pas seulement à celles d'un `COPY` — au moins pour une transaction qui porte un `COPY`, peut-être pour toutes (une transaction d'insertions ordinaires trop grosse se replierait au lieu d'être refusée). À peser : le repli rend la transaction forcée, qui attend le départ des autres à sa validation.
