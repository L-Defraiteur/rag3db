# Un lot C++ rejoue 35 minutes quand seul du Rust a atterri

- **État** : fait (15 novembre 2026, master 72f5e1ffe) : la voie des suites C++ transférables de push_garde.sh ; tools/shell y manque encore (ticket docs/optimiseur/tickets/2026-11-16-la-voie-cpp-transferable-ignore-tools-shell.md) ; fermé à la passation de G le 16 novembre
- **Famille** : produit
- **Gravité** : coût ; un lot C++ vert rejoue ses suites (35 minutes) pour du code qu'elles ne lient pas.
- **Atteignable en service** : non, c'est l'outillage du push.
- **Touche rag3weaver** : non ; `tools/poste/push_garde.sh`.
- **Pour** : optimiseur (G).
- **Ouvert le** : 15 novembre 2026

## Ce qui se passe

Le lot C++ du cœur, testé sur a2f9c9b65, doit atterrir après le Rust d'archi. La garde le refuse. En pile, (b) la tête
d'archi n'est pas un ancêtre du testé. Hors pile, `code_apporte(base, master)` liste le Rust d'archi, et déclarer une autre
base mentirait. Le cœur rejoue donc 35 minutes de suites C++, alors que rien de ce qu'elles lient n'a changé.

## La correction attendue

Une voie de la garde. Si `git diff <base testée>..<master>` est vide sous les chemins du moteur (`src/`, `test/`,
`extension/vector/`, `tools/rust_api/`), et si le lot ne touche que ces chemins (plus des `.md`), les suites C++ jouées
sur la base sont transférables. La garde le dit au journal, et pousse. Le symétrique existe déjà : un lot Rust quand seul
du C++ a atterri passe, par la règle MOTEUR_ANCIEN, puisque `code_apporte` ne regarde pas `src/`. Rouge d'abord dans
`test_push_garde.sh` : un lot C++ seul, du Rust atterri sur master, rien sous les chemins du moteur. Puis deux arrêts :
master a touché `src/`, ou le lot touche aussi du Rust.
