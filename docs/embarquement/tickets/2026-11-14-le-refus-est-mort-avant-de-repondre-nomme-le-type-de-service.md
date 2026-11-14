# Le refus « est mort avant de répondre » nomme le type de service, pas le programme lancé

- **État** : ouvert (14 novembre 2026, embarquement, relevé pendant le lot de la reprise après plantage ; hors de ce lot, décision de c7)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : diagnostic trompeur. Un fournisseur qui meurt au démarrage est nommé « rag3weaver-embeddings » quel que soit le programme déclaré, et son journal est cherché sous ce nom.
- **Atteignable en service** : oui, dès qu'une activation déclare un `programme` qui n'est pas `rag3weaver-embeddings` (la fixture de test, un fournisseur tiers, une copie renommée)
- **Touche rag3weaver** : oui (`src/serveur.rs`, `src/daemon/embeddings.rs`)

## Ce que c'est

`DaemonEmbedder::serveur(adresse, programme)` construit `Serveur::new(SERVICE, adresse, programme)`. Le
premier argument est le type de service, toujours `rag3weaver-embeddings`. Il sert ensuite de nom dans
le refus de `serveur.rs` (« « {nom} » est mort avant de répondre (code {}) — le journal est dans … »)
et dans le nom du journal (`journaux/rag3weaver-embeddings.log`).

Vu le 14 novembre, sur la fixture `tests/fixtures/residence/fournisseur.py` :

```
activation à la demande : « rag3weaver-embeddings » est mort avant de répondre (code 4) — le journal est dans /tmp/.tmpBWPucd/journaux/rag3weaver-embeddings.log
```

Le programme qui est mort était `fournisseur.py`.

## Ce qu'il faut pour le fermer

Le refus nomme le programme lancé (son chemin, ou son nom de fichier), à côté du type de service. Un
témoin sur la fixture, rouge d'abord : le refus d'un démarrage mort contient `fournisseur.py`.
