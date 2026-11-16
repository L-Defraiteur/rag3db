# La maintenance du plein texte ne peut pas ouvrir un index existant

- **État** : ouvert, 16 novembre 2026, demande C7.
- **Pour** : Recherche ; coordination G/Arbre pour copies privées de migration.
- **Famille** : produit
- **Gravité** : maintenance bloquée ; une ouverture ordinaire peut modifier la preuve.

## Constat et liens

Sonde RO rapportée par C7 : `/backend/fts-live-ids`, File, hôte927, renvoie422 avec aucun handle ouvert. Cela ne prouve pas une absence sur disque.
`ensure_fts_handle` et `open_fts_files` suivent un chemin autorisant suppression de dossier sur génération divergente et création après échec d’ouverture : ils ne conviennent pas à cette maintenance.
Suite du [ticket des comptes](2026-11-15-l-etat-d-index-ne-voit-pas-les-orphelins-du-plein-texte.md), prioritaire avant sa qualification. Le lien exact du ticket Arbre de bascule des trois bases est demandé à son porteur ; aucune base de Lucie n’est ouverte ici.

## Plan avant code

Réutiliser le format snapshot Lucivy (`export_snapshot_sharded` puis `ShardedHandle::open_snapshot`) déjà employé par `fts_handle::open_blob_snapshot`, en adaptant la lecture des fichiers existants sans writer. Auditer la version réellement épinglée avant de retenir cette voie. Ne pas appeler les constructeurs de stockage qui font `create_dir_all`.
L’identité demandée est la table physique dans la cellule exacte du catalogue : aucune résolution/redirection d’une entité publique. Vérifier la génération de base et celle du dossier strictement, distinguer absence, corruption, divergence et erreur IO ; aucun fallback create, rebuild, retrait, warmup ou modification de jeton. Un handle déjà ouvert ne dispense pas du contrat de provenance à définir explicitement.

Publier les témoins avant le correctif : index absent (aucun dossier créé), config/shard corrompu (octets conservés), génération divergente ou illisible (aucun retrait), cellule étrangère et alias public refusés, index existant valide (vivants et jeton cohérents, manifeste avant/après identique). Les erreurs doivent porter des variantes typées, pas une chaîne interprétée. Fixtures privées seulement.

## Limites et qualification

Aucun Cargo/rustc/test Rust lancé ; témoins à écrire puis publier, donc aucun rouge observé à ce stade. Attendre pin G exact avant toute compilation. Inventaire RO d’Arbre distinct de l’implémentation Recherche ; un seul éditeur Catalog. Aucun service/base réelle/purge. Qualification à coordonner sous poste, premier rouge conservé ; aucun ancien GO réactivé.
