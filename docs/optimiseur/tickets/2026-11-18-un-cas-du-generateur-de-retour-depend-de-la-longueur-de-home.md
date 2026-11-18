# Un cas du témoin du générateur de retour dépend de la longueur de `HOME`

- **État** : ouvert (18 novembre 2026) — à G
- **Famille** : produit (outillage : `docs/procedures/scripts/test_generated_return.py`)
- **Gravité** : faux rouge. Le cas rougit sans défaut du générateur, dès que `HOME` est un peu long.
- **Atteignable en service** : non
- **Touche rag3weaver** : non
- **Pour** : G (optimiseur)
- **Signalé par** : G, à l'évaluation du travail de Codex, le 18 novembre 2026

## Ce qu'on voit

`GeneratedReturnIntegrity.test_second_live_socket_refuses_before_first_move` lie un socket AF_UNIX sous le dossier du
cas, `~/.cache/generated-return-integrity-*/…` (ligne 150, `listener.bind`). Avec `HOME=/home/lucied` (12 caractères),
le chemin tient sous la limite de 108 octets, et le cas est vert. Avec un `HOME` de 26 caractères
(`/home/lucied/.cache/g12345`), il rend `OSError: AF_UNIX path too long`.

## Ce qu'il faudrait

Le socket se lie dans un dossier court à lui (`tempfile.mkdtemp(dir='/tmp')`, que le tmpfs accepte pour un socket), et
le cas le pointe depuis la base de test, par un lien ou par le chemin attendu par le générateur. Le cas doit rester vert
sous un `HOME` long.
