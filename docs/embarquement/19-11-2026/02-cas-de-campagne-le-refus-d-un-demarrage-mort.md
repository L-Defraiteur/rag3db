# Cas de campagne : le refus d'un démarrage mort ne nomme pas le programme

```json
{"lanceur": {"parent": "c4b36feffa9f76654d1537e21c2bc8b1877cfc70", "tree": "extension/rag3weaver", "needs_lib": false, "witness": "serveur::tests::a_dead_start_names_the_program", "test": "cargo test --lib -- serveur::tests::a_dead_start_names_the_program", "suites": [], "hints": ["extension/rag3weaver/src/serveur.rs", "extension/rag3weaver/tests/fixtures/residence/fournisseur.py"], "ours": "0bda7c020f6f5f48cf37429af46497a2b8b396c2"}}
```

Session embarquement, 19 novembre 2026, pour la campagne des correctifs passés (chantier de Lucie ; lanceur d'archi,
juge du banc). C'est un cas court, en Rust, sans la lib rag3db et sans service d'embarquement. La fixture est un
fournisseur Python factice, en CPU, sans modèle ; il faut `python3`.

## Le point de départ

- **Commit parent** : `c4b36feffa` (18 novembre 2026, sur master). Le témoin y est, rouge ; le correctif n'y est pas.
- **Arbre** : `extension/rag3weaver` suffit, sans la lib rag3db (aucune feature).

## Le signalement, tel qu'il était

> `DaemonEmbedder::serveur(adresse, programme)` construit `Serveur::new(SERVICE, adresse, programme)`. Le premier
> argument est le type de service, toujours `rag3weaver-embeddings`. Il sert ensuite de nom dans le refus de
> `serveur.rs` (« « {nom} » est mort avant de répondre (code {}) — le journal est dans … ») et dans le nom du journal
> (`journaux/rag3weaver-embeddings.log`).
>
> Vu le 14 novembre, sur la fixture `tests/fixtures/residence/fournisseur.py` :
>
> ```
> activation à la demande : « rag3weaver-embeddings » est mort avant de répondre (code 4) — le journal est dans /tmp/.tmpBWPucd/journaux/rag3weaver-embeddings.log
> ```
>
> Le programme qui est mort était `fournisseur.py`.
>
> Le refus nomme le programme lancé (son chemin, ou son nom de fichier), à côté du type de service. Un témoin sur la
> fixture, rouge d'abord : le refus d'un démarrage mort contient `fournisseur.py`.

## Le témoin

- **Filtre** : `serveur::tests::a_dead_start_names_the_program`, dans `extension/rag3weaver/src/serveur.rs`. Le témoin
  est identique sur master.
- **Commande**, depuis `extension/rag3weaver` : `cargo test --lib -- serveur::tests::a_dead_start_names_the_program`.
- **Rouge au parent**, vérifié le 19 novembre à luciepc, sans la lib : le refus dit seulement « « rag3weaver-embeddings »
  est mort avant de répondre (code 70) », sans `fournisseur.py`. **Vert sur master** (19 novembre, même commande).
- **Suites exigées par les chemins** : aucune (`tools/poste/required_suites.txt` ne nomme pas `serveur.rs`).
- **Ce qui ne doit pas changer** : le nom du journal (`<type de service>.log`), que `cause_d_arret`
  (`src/model_source/activation.rs`) lit.

## Notre correctif (caché à Algareth, pour le juge)

`0bda7c020f`, « fix(embarquement): le refus d'un démarrage mort ou muet nomme le programme lancé » :
`extension/rag3weaver/src/serveur.rs`, +20 −9. `ServeurError::Mort` et `::Muet` portent le programme, et leur message
dit « « rag3weaver-embeddings » (fournisseur.py) est mort avant de répondre (code 70) ».
