# Relevé, scénario 2 : un projet de zéro, en ne lisant que ce que dit le produit

Session tty, nuit du 20 au 21 novembre 2026, à la demande de Lucie (« des tests comme si tu étais un dev qui ne connaît rien au projet et veut lancer un projet de zéro, dont la création du dépôt git, sans push »).

## Le montage

- **L'hôte d'essai** est le binaire installé (`~/.local/bin/rag3weaver-backend`, la tête 83c667e420), lancé avec un **HOME à lui** (`~/.cache/rag3weaver-build/essais-21-11/home`), comme `scripts/test_readme_quickstart.sh`, sur le port 17981. Le modèle est atteint par le chemin de sa clé et son projet ; la clé n'est pas lue. Pas de service d'embarquement : rien n'est envoyé aux services en place, et l'index reste en plein texte.
- **Le terminal** est bâti depuis la tête de master du moment (`cargo build --release --bin rag3weaver-tty`) : il n'est installé sur aucun poste (ticket `2026-11-21-le-terminal-n-est-installe-sur-aucun-poste`).
- **Le projet** est un dossier neuf, `compteur`, avec `git init -b main`, sans remote, donc sans push possible.

## L'incident de 23 h 43, avant ce montage

Un premier hôte d'essai, lancé avec **mon HOME** et `RAG3WEAVER_CONFIG_DIR` et `RAG3WEAVER_DATA_DIR` à moi, ainsi que le fichier d'environnement du service :
- à 23 h 43 min 16 s, il a repris les sessions de chat de `~/.local/share/rag3weaver` et l'équipier `12389e156742/atelier` de Lucie, et réécrit `taches/registre.json` (23:43:16.828, contenu cohérent selon G) ;
- je l'ai arrêté par son pid à 23 h 43 min 25 s ;
- en s'arrêtant, il a dit « daemon.json retiré » de celui de l'hôte de service. G a vérifié que le fichier était intact (mtime 20:50:54) : l'annonce était fausse.

Ses bases ne se sont pas ouvertes, puisque le service tient leur verrou. Son journal : `~/.cache/rag3weaver-build/essais-21-11/hote-incident-23h43.log`. Ticket produit chez archi, par c7 : un hôte isolé doit l'être pour tout son état (sessions, chats, tâches, `daemon.json`), par une seule variable ou un seul dossier, et le dire à `/sante`.

## Les pas

1. **`git init`** du dossier neuf : rien à signaler.
2. **`rag3weaver-backend code-tree add <dossier> --from templates/backends/code/backend.json --daemon <hôte>`**, depuis la racine du crate, comme le dit le README.
   - Premier essai : refusé, parce que le dépôt n'a pas d'identité git (mon HOME d'essai n'a pas de `~/.gitconfig`). Le message dit exactement quoi faire (`git -C <dépôt> config user.name …`, puis recommencer). **Bon.**
   - Second essai : accepté, sur un dépôt sans aucun commit. Deux libellés sont troubles pour un nouveau venu : « base … — 0 fichier(s) de templates/backends/code/backend.json, copie pleine (pas de reflink ici) » et « templates/backends/code/backend.json n'était pas ouvert dans l'hôte ». Ils nomment le gabarit là où l'on attend le dépôt.
