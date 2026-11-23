# Une montre écrit après la fin d'un témoin d'e2e_tty

- **État** : ouvert le 23 novembre 2026, vu une fois sur deux passes de `host-state-cache` (le même code), à luciepc.
- **Famille** : témoin
- **Gravité** : une passe d'`e2e_tty` peut sortir en rouge alors que ses 5 tests passent. `run_e2e.sh` trouve un résidu dans le `TMPDIR` de la suite, et le dépôt est retardé d'un rejeu.
- **Atteignable en service** : non, ce sont les témoins ; mais la cause (une montre qui écrit dans une base fermée) mérite d'être regardée.
- **Touche rag3weaver** : oui (la montre des fichiers, `ReindexFileNode`, et l'arrêt des backends des témoins d'`e2e_tty`).

## Ce qui s'est passé

Passe de `host-state-cache` (bd18b9a404), 23 novembre, 19:47 : les 5 tests d'`e2e_tty` passent, puis `run_e2e.sh` relève « RÉSIDUS e2e_tty : 1 entrée(s) dans son TMPDIR (8,0K) ». Dans le journal :

> [montre] ReindexFileNode : le lot de 1 fichier(s) a échoué (db error: Library_Chunk: must reopen: … IO exception: Cannot open file /tmp/rag3weaver-e2e-…/e2e_tty/.tmpvUO7nP/data/code-petit.rag3d…

> [montre] src/lib.rs en erreur : … la base doit être rouverte …

La montre de fichiers d'un dépôt de témoin (`code-petit`, créé par `depot_inscrit`) réindexe encore `src/lib.rs` après la fin du témoin. Son dossier temporaire est déjà retiré, et l'écriture recrée une entrée dans le `TMPDIR`. Rejouée telle quelle, la même tête passe sans résidu.

## Ce qu'il faut

- Les témoins d'`e2e_tty` qui hébergent un dépôt arrêtent leur hôte, et donc les montres de ses backends, avant que leur `tempdir` soit retiré ; ou bien la montre s'arrête d'elle-même quand sa base a disparu, sans rien recréer.
- Le témoin est rouge d'abord : un hôte de témoin dont le dossier est retiré pendant qu'une réindexation est en vol ne laisse aucun résidu.
