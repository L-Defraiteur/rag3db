# Le terminal n'est installé sur aucun poste

- **État** : fermé le 21 novembre 2026 par la session tty. G pose désormais `rag3weaver-tty` avec chaque hôte, bâti de la même tête, dans `~/.local/bin`, aux deux postes. Vérifié à 4 h 05 : `~/.local/bin/rag3weaver-tty` à la tablette et à luciepc, même empreinte (md5 7e70a282). L'installeur côté produit, qui posera le terminal avec l'hôte, est en ticket chez G. Ouvert le 20 novembre 2026 à 23 h 4x.
- **Famille** : produit
- **Gravité** : bloquant pour l'essai au terminal : sans bâtir le crate elle-même, Lucie ne peut pas lancer le terminal.
- **Atteignable en service** : oui, à chaque poste.
- **Touche rag3weaver** : oui (la pose de l'hôte, le crate `extension/rag3weaver/tty`).

## Ce que c'est

L'hôte est posé dans `~/.local/bin/rag3weaver-backend` aux deux postes, avec ses sauvegardes, mais `rag3weaver-tty` n'est ni dans `~/.local/bin` ni dans le `PATH`. Ni la tablette ni luciepc ne l'ont. Le README du crate dit seulement `cargo build --release --bin rag3weaver-tty`.

## Le pas qui bloque

Le tout premier : `rag3weaver-tty --repo <dépôt> --task <tâche>` répond « commande introuvable ».

## Contournement

Bâtir le terminal depuis la tête de l'hôte posé, dans le crate :

    cd extension/rag3weaver/tty && cargo build --release --bin rag3weaver-tty

Puis lancer `target/release/rag3weaver-tty`, ou le copier dans `~/.local/bin`.

## Ce qu'il faut

Que la pose d'un hôte pose aussi `rag3weaver-tty`, bâti de la même tête, dans `~/.local/bin`, aux deux postes. c7 l'a demandé à G le 20 novembre au soir (un lot ou un ticket chez G).
