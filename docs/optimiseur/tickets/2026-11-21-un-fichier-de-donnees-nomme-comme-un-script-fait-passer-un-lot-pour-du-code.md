# Un fichier de données nommé comme un script fait passer un lot pour du code

- **État** : ouvert (21 novembre 2026, écrit par G à la demande de c7)
- **Pour** : G (optimiseur)
- **Famille** : produit (l'outillage des passes : la garde du push)
- **Gravité** : basse. Un lot de scripts ou de données passe pour du Rust : il exige une attestation check_features et
  une passe de vingt minutes pour rien.
- **Atteignable en service** : non, le pousseur et la garde seulement
- **Touche rag3weaver** : non (`tools/poste/push_garde.sh`)

## Ce que c'est

`push_garde.sh --code-apporte` compte comme code tout fichier du crate que « quelque chose appelle ». `appeles_dans`
le décide par le **seul nom de fichier** : si `basename` du fichier se trouve, en texte, dans un fichier des racines
jouées (ou d'un script déjà appelé), le fichier est appelé.

Le 21 novembre 2026, le lot pusher-french-identifiers range deux listes de mots sous
`extension/rag3weaver/data/wordlists/{fr,en}/`, chacune avec sa licence `LICENSE`. Le mot `LICENSE` figure dans les
racines jouées : les deux licences ont été comptées comme du code, et le pousseur a refusé le dépôt (« le lot est rust,
pas scripts »).

## Contournement

Les licences sont nommées `LICENSE.txt` (dit dans le README des listes). Tout fichier de données dont le nom est un mot
courant (`LICENSE`, `README`, `VERSION`, `index.json`…) peut le refaire.

## Ce qu'il faudrait

Chercher un appel par le chemin relatif du fichier, ou par son nom dans une forme d'appel (`bash …/nom`, `python3
…/nom`, `source`, `include_str!`, `"…/nom"`), plutôt que par le nom nu n'importe où dans le texte. Témoin : un
`data/x/LICENSE` ajouté n'apporte pas de code ; un script renommé que run_e2e.sh appelle en apporte toujours.
