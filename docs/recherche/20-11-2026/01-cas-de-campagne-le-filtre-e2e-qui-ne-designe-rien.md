```json
{
  "parent": "0f6cff8130f3426c11be0e9e3f154cd29207b31c",
  "tree": "extension/rag3weaver",
  "paths": [
    "extension/rag3weaver/scripts",
    "extension/rag3weaver/run_e2e.sh"
  ],
  "needs_lib": false,
  "witness": "scripts/test_played_tests.sh",
  "test": "bash scripts/test_played_tests.sh",
  "suites": [],
  "hints": [
    "extension/rag3weaver/scripts/played_tests.sh",
    "extension/rag3weaver/run_e2e.sh"
  ],
  "ours": "f6eca5f135b7bfb759066d077b7f333cda9cbe6f"
}
```

# Cas de campagne : un filtre e2e qui ne désigne aucun test passe pour vert

Session recherche, 20 novembre 2026, pour la campagne des correctifs passés (chantier de Lucie ; lanceur d'archi,
juge du banc). C'est un cas court, en shell, **sans la lib rag3db ni cargo** : deux scripts bash.

## Le point de départ

- **Commit parent** : `0f6cff8130` (le témoin rouge, sur master depuis le 20 novembre). Le témoin y est, rouge ; le
  correctif n'y est pas.
- **Arbre** : `extension/rag3weaver`, dont seuls `scripts/` et `run_e2e.sh` comptent. Bash suffit : ni cargo, ni
  lib, ni service.

## Le signalement, tel qu'il était

> `run_e2e.sh --test <suite> <filtre>` joue une suite e2e sous un filtre de noms de tests. Un filtre qui ne désigne
> aucun test de la suite, par une faute de frappe ou un test renommé, rend pourtant un code de sortie vert : la
> passe se croit prouvée alors qu'elle n'a rien joué. Le refus existe depuis le 3 octobre (« aucun test joué »), mais
> il ne mord pas. Le binaire e2e unique joue d'abord ses propres filets, deux tests à chaque passe, et le compte des
> tests joués les inclut. Un filtre qui ne désigne rien doit être refusé, quels que soient les filets ; sans filtre,
> une suite qui n'a rien joué doit l'être aussi.

## Le témoin

- **Fichier** : `extension/rag3weaver/scripts/test_played_tests.sh`. Il fabrique des journaux synthétiques de
  `run_e2e.sh` (les filets, puis la suite) et dit le verdict que chacun doit rendre : `VERT` (code 0) ou `ROUGE`
  (code 1). Il est identique au parent et sur master (vérifié le 20 nov. sur f6eca5f135).
- **Rouge au parent** : le cas du ticket (« a filter that names no test of the suite is refused, whatever the nets
  played ») sort 0 au lieu de 1. Le témoin dit `ROUGE`, code 1.
- **Vert au correctif** : `VERT`, code 0. Les deux sont joués le 20 novembre sur des copies des deux scripts.
- **Suites exigées par les chemins** : aucune (`scripts/` et `run_e2e.sh` ne sont pas dans
  `tools/poste/required_suites.txt`).

## Notre correctif (caché à Algareth, pour le juge)

`f6eca5f135`, « fix(e2e): un filtre qui ne désigne aucun test de la suite est refusé — les filets du binaire ne
comptent plus parmi les tests joués » :
- `extension/rag3weaver/scripts/played_tests.sh` (+5 −2) : le compte saute le bloc des filets (`e2e_filets`) ;
- `extension/rag3weaver/run_e2e.sh` (+3 −15) : il appelle le script au lieu de son propre compte.

Le ticket est fermé dans le même commit, mais `docs/` est hors de l'arbre archivé.
