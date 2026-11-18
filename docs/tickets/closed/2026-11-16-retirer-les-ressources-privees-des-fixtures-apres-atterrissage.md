# Retirer les ressources privées des fixtures après atterrissage

- **État** : fermé, le 18 novembre 2026, par la session mémoire (96). Les ressources privées du lot sont retirées, et le reste est effacé au oui de Lucie.
- **Porteur** : mémoire.
- **Famille** : produit.
- **Gravité** : ressources privées à retirer après livraison, sans perte des preuves.
- **Atteignable en service** : non.
- **Touche rag3weaver** : non, uniquement les ressources privées du lot qualifié.

## Constat et périmètre

Le lot ETXTBSY et son profil TMP sont intégrés. Les worktrees privés
`cx-memoire-script-fixtures*`, leurs administrations et le target privé
`target-cx-memoire-script-fixtures` restent présents aux deux postes.
Les autres lots mémoire et toutes les ressources étrangères sont exclus.

## Contrat avant chaque retrait

Inventaire exact, arbres propres sans nettoyage/reset, branches distantes poussées,
archives fidèles des sources et administrations détruites répliquées aux deux postes,
preuves/ELF et anciens rouges conservés. La source historique 734 et son archive
physique restent conservées. Quiescence fraîche exe/cwd/FD/cmdline ; garde courante
SHA256 6f3dfceb17c9703029d2f4f26385f9aa21820c84bc79c518e1338b6d0f36188c,
rapport neuf VERT relu immédiatement avant chaque retrait. Aucun -D ni branche
remote supprimée. Refus structurel Git sous-module : nouvelle copie et nouveau VERT
avant un unique --force ; aucun contournement de garde. Tout état sale reste conservé.
Target : propriétaire/provenance/reçus terminés/ELF sauvegardés et scan strict frais,
pas de suppression d’une cible liée ou étrangère, pas de service ni de base touchés.

## Critères de fin

Rapport détaillé des ressources retirées et conservées, empreintes des copies aux
deux postes, rapports frais avant retrait, totaux octets/WT/branches. Le ticket reste
ouvert tant qu’un retrait autorisé reste bloqué ou qu’un contrôle manque.

## Résultat et suite bornée

[Rapport13](../../memoire/16-11-2026/13-retrait-garde-des-fixtures.md) : 16WT retirés après
archives2postes et garde6f3 fraîche verte, 14branches -d, target privé retiré après
provenance/ELFs2postes/scans. Archives finales363entrées identiques2postes, manifeste
b7d35c4e2bbf2dd67c9a6804800f612a0b681256791d15285fef2c0d3dcae3d4.
Quatre WT PC conservés avec target interne ou __pycache__ ignorés ; aucun clean/reset.
La suite exige d'abord une décision sur ces données, sans contourner la garde.
Source734/archives physiques/branches distantes/anciens rouges et ELF conservés.
Incident scp cachecommun garde_worktree.py existant réécrit6f3 sans ancienSHA :
rapport et métadonnées publiés, C7 averti ; aucune restauration spéculative.

## Gel du cache commun après bilan

Incident désormais suivi séparément par le
[ticket dédié](../../memoire/tickets/2026-11-16-une-copie-de-garde-ecrase-un-chemin-preexistant-sans-empreinte-avant.md),
avec appel outil et retour originaux, métadonnées/chemins exacts et recherches RO.
Le cache commun reste gelé ; l'ancien SHA du fichier préexistant est inconnu.
Le présent ticket reste ouvert pour les quatre WT PC ignorés nommés dans le rapport13,
sans clean/reset ni retrait supplémentaire. Aucun autre nettoyage n'est entrepris.


## Clôture, le 18 novembre 2026

- La session mémoire (96) a posé ce ticket sur master depuis la branche
  `cx-memoire-script-fixtures-cleanup-r1` (492745eab5), où Codex l'avait laissé.
- Au ménage de la tablette (Lucie : « oui pour le ménage », par c7), les restes ont été effacés
  après un contrôle qu'aucun processus ne les tenait :
  - l'archive `~/.cache/rag3weaver-build/archives/cx-memoire-script-fixtures-cleanup-16nov-v1`
    (2,4 Go) ;
  - les worktrees `cx-wt-memoire-script-fixtures-r4`, `cx-wt-memoire-script-fixtures-cleanup-r1`
    et ses voisins.
- Gardés :
  - le manifeste de l'archive, `…/archives/cx-memoire-script-fixtures-cleanup-16nov-v1-manifest.json`,
    comme trace de son contenu ;
  - l'archive de l'incident, `…/archives/cx-memoire-cache-copy-incident-16nov-r1`, qui sert le
    ticket [une copie de garde écrase un chemin préexistant](../../memoire/tickets/2026-11-16-une-copie-de-garde-ecrase-un-chemin-preexistant-sans-empreinte-avant.md).
- Toutes les branches distantes restent. Le lot lui-même est sur master par `b0a845bb48`.
