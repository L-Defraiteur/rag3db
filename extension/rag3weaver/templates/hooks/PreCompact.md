# PreCompact — les consignes du résumé de compaction

Ce que le hook rend ici n'est pas du contexte pour Claude : c'est ce que le compacteur lit pour
faire son résumé (10 000 caractères au plus). Relu à chaque appel. Trou : `{rubriques}`. Le hook
ajoute ensuite le dernier `identity` connu de la session, en graine.

## consignes_resume

Dans le résumé, garde tels quels, sous leurs noms : l'identité de la session ({rubriques}), le plan courant avec ses pas faits et restants, les worktrees et branches tenus (chemins et noms exacts), les mandats reçus (qui, quand, mot pour mot), les pairs par leur clé (c7, 96, 73…), et les hashes de commits cités. Rien de tout cela ne se résume : ce sont des faits à recopier.
