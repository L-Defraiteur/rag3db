# `code-tree add` et le terminal ne disent pas le même nom de dépôt

- **État** : ouvert le 21 novembre 2026 par la session tty, dans les essais de la nuit (scénario 2, un projet de zéro). Pour arbre (c7) : un seul nom partout.
- **Famille** : produit
- **Gravité** : gêne au premier pas ; le message du terminal permet de s'en sortir.
- **Atteignable en service** : oui, à chaque dépôt ajouté par `code-tree add`.
- **Touche rag3weaver** : oui (`code-tree add`, `rag3weaver-tty --repo`).

## Ce que c'est

`rag3weaver-backend code-tree add <dossier>/compteur …` répond « code-compteur : manifeste … », puis « code-compteur enregistré à l'hôte ». Un nouveau venu reprend ce nom : `rag3weaver-tty --repo code-compteur --task debut` répond « aucun dépôt attaché ne s'appelle « code-compteur » ; attachés : compteur ».

## Le pas qui bloque

Le premier lancement du terminal, avec le nom que vient de donner `code-tree add`.

## Contournement

Prendre le nom que liste le message du terminal (`--repo compteur`), ou le chemin du dépôt.

## Ce qu'il faut

Un seul nom pour un dépôt, que disent `code-tree add`, le terminal et l'IDE. À défaut, `--repo` accepte aussi le nom que donne `code-tree add`.
