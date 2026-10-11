# Les e2e du démon lancent le binaire sur un port « libre » qu'un autre peut prendre

- **État** : corrigé (11 octobre 2026, optimiseur, branche `demon-e2e-port-zero`)
- **Gravité** : test instable (un rouge sans défaut du produit)
- **Atteignable en service** : non (en service, l'adresse est fixée par l'appelant)
- **Touche rag3weaver** : oui (`src/serveur.rs`, `src/daemon/`, `src/bin/rag3daemon.rs`,
  `src/bin/rag3weaver-embeddings.rs`, `tests/e2e_rag3daemon.rs`, `tests/e2e_demon_embeddings.rs`)

## Ce que c'est

Le même défaut que le ticket fermé du démon de test (`closed/2026-10-10-le-demon-de-test-n-a-pas-repondu-sous-charge.md`),
mais entre processus : `e2e_rag3daemon` et `e2e_demon_embeddings` choisissaient un port par
`port_libre()` (lier le port 0, lire le numéro, **le relâcher**), puis lançaient le binaire de
service sur ce numéro ; entre les deux, un autre test ou un autre processus pouvait le prendre.

## Recette

Pas de Cypher : la course. Dix tours de huit processus du binaire e2e, chacun les trois tests
ignorés de `e2e_rag3daemon` en parallèle (`--test-threads=3`), sous `poste lourd` :

```
assurer : 127.0.0.1:35235 est déjà pris, mais ce n'est pas « rag3daemon » : aucune réponse lisible
```

## Témoin

`~/.cache/rag3weaver-build/demon/temoin-e2e.sh <binaire e2e> 10 8`.

| | tours rouges sur 10 | processus rouges sur 80 |
|---|---|---|
| avant (origin/master 058f023ff) | 2 | 2 |
| après | **0** | **0** |

## Le correctif

- **Le binaire de service sur un port nul** (`rag3daemon`, `rag3weaver-embeddings`) : appelé avec
  `--adresse 127.0.0.1:0`, il se lie lui-même (`daemon::ecouter_si_port_nul`), écrit sur sa
  sortie standard la ligne `serveur::LIGNE_ADRESSE` (`adresse liée : 127.0.0.1:45123`) avant de
  servir l'écouteur (`servir_lie`), et `/sante` dit l'adresse réellement liée (champ `adresse`).
  Sur un port non nul, rien ne change : `servir(adresse)` comme avant.
- **Le lanceur** (`Serveur::assurer`) : sur un port nul, il lance toujours un nouveau serveur, lit
  la ligne d'adresse sur la sortie de **son** enfant (par un tube ; un fil recopie le reste dans
  le journal, que plusieurs lancements partagent), sonde cette adresse-là et la rend dans
  l'attache. Pour se rattacher ensuite au même serveur, on le décrit par cette adresse.
- **Les clients** (`DaemonConnection`, `DaemonEmbedder`) disent l'adresse où ils joignent le démon
  (`adresse()`).
- **Les deux e2e** lancent sur `127.0.0.1:0` et lisent l'adresse dans le client ; plus de
  `port_libre()` nulle part.

## Ce qui n'est pas joué

`e2e_demon_embeddings` charge 2,2 Go de poids sur la carte locale : il est compilé, pas joué de
jour (règle « ménager l'écran ») ; il emprunte le même lanceur et le même binaire que
`e2e_rag3daemon`, éprouvé ci-dessus.
