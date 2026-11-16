# Générateur des retours posé aux deux postes

Source master e51b0d0c5bda791a62f39910a8872f43f7912468, sur basebef0.
La recette générée conserve les voisins présents AU RETOUR, y compris un WAL
apparu après le snapshot. Son préflight global vérifie tous les snapshots,
inventaires, ensembles vivants/types et intersections avant tout déplacement.
Le helper V2 reste e1fd, repris bitidentique depuis bf0aba.

C7 a autorisé landing et pose ; ED a levé P1 sur deux vrais rouges conservés
(second vivant absent et socket spécial). Quinze tests CPU passent aux deux
postes, puis quinze contre CHAQUE COPIE POSÉE, zéro échec/saut. Aucun rejeu Rust
induit : les arbres produit sont identiques au master de base, seuls huit
chemins docs/outillage changent. Vrai postePush, garde réelle12égaux, fetch et
contrôles propre/sansrebase/HEAD=origin-master lus aux deux postes.

Bundle installé : `~/.cache/rag3weaver-notes-A/return-generator-b8d7579797/`.
Entrée `~/.cache/rag3weaver-notes-A/instantanes.sh` publiée par lien atomique,
ancienne f4ab conservée par cp-p dans
`instantanes.sh.avant-return-generator-b8-20261116`.

| Fichier régulier755 | SHA256 |
| --- | --- |
| instantanes.sh | 0cecea650a9db3dbe9dbe57159f63372339dd99cb35ef2b0d5909cb99eec9437 |
| generate_snapshot_return.py | ba27110ea947697aa432f84ed5c8e9be9d06878717d9c5dd2738cab10c9e8cd4 |
| restore_snapshot.py | e1fd96b25c1cc61b70507c59f7f203acb21936e5b676f68c74b88d216f923854 |

Reçus/journaux sur chaque poste :
`~/.cache/optimiseur-notes/return-generator-16/install-b8/` ; landing local
`landing-e51/{push,pile}.log`. Les fixtures générateur et les anciens rouges
sont conservés. Le [ticket clôturé](../../tickets/closed/2026-11-16-le-retour-d-un-snapshot-oublie-les-voisins-crees-apres.md)
porte les SHA des reçus et toutes les étapes.

Refus de PRÉFLIGHT : aucun déplacement ; succès : ensemble ancien exact vérifié.
Une erreur après début peut être partielle, avec état après et preuves conservés.
Le retour opérateur reste hors Common ; ne pas injecter le bus utilisateur dans
une validation privée. Ce lot n'a restauré aucune base originale ni modifié
ancien RET, snapshot/attestation72h, API, lib ou GPU.
