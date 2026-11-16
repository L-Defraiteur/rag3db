# RO du CLI 6f3 et archives des deux journaux ignorés

Garde exacte SHA256 **6f3dfceb17c9703029d2f4f26385f9aa21820c84bc79c518e1338b6d0f36188c**, comparéePC/portable. CLI réellement appelé, aucunforce :

```bash
python3 /home/lucied/git_workspaces/rag3db-cx-ed-stable-tie-integration-r1/extension/rag3weaver/scripts/garde_worktree.py \
 --depot /home/lucied/git_workspaces/rag3db \
 --worktree /home/lucied/git_workspaces/rag3db-cx-ed-stable-tie-identity-r2 \
 --rapport /home/lucied/.cache/rag3weaver-build/cx-ed-ties-cleanup-reports-20261116/source-r2.json \
 --archive /home/lucied/git_workspaces/rag3db/.git/worktrees/rag3db-cx-ed-stable-tie-identity-r2 \
 /home/lucied/.cache/rag3weaver-build/cx-ed-ties-cleanup-archives-20261116/source-r2-administration
```

RapportROUGEcode2 conservé. Administrationfidèle admise ; seulstatusignoré des sources refuse. [Inventaire détaillé](ties-ignored-source-inventory.json) : index_entries vide, aucun fichier indexé manquant, aucune autre sortie ignorée signalée.

| Fichier sous extension/rag3weaver/target | UID:GID | Mode | Octets | Règle |
|---|---|---|---|---|
| charge-last.tsv |1000:1000|0644|575|.gitignore:89 extension/rag3weaver/target/|
| e2e-last.log |1000:1000|0644|46629|même règle|

La source ciblée est le WT propre qualifiébd76, pas l'arbreLucie. Copie fidèle supplémentaire souscachePC cx-ed-ties-source-logs-archive-20261116 : UID/GID/mode/taille/mtime_ns/SHA de CHAQUE fichier comparés aux originaux avant/après, originaux inchangés. ArchiveTARpax **cx-ed-ties-source-logs-archive-20261116.tar**, SHA **c9909c48bc190aa4914dfdc55924eb036ccee44be9983c1a4c48731b5df3cb52** identiquePC/portable ; copieportable réhachée. Ces archives sont séparées du TAR ELF+administration38d8daec...8bb7 déjà conservé ;29preuvesv3 demeurent intactes.

## Pourquoi l'archive ne peut pas suffire au VERT de 6f3

Lectureexacte garde_worktree.py : lignes303–306 refus inconditionnel dès que status --porcelain=v1 --untracked-files=all --ignored=matching rend quelque chose. Lignes325–354 : --archive ne compare que racinesADMINISTRATIVES détruites, puis toute source d'archive nonadministrative est refusée « preuve archive non attribuable ». Il n'existe pas d'option du CLI pour attester un journal source ignoré préservé. Ajouter --archive pour le dossier target ne supprime pas le refusstatus et ajoute un secondrefus. Aucun nouvel essai inchangé lancé : ce résultat découle de la relecture du codeexact, pas d'un VERT supposé.

Plan déjà réalisé : inventorier les2fichiers/index, copierfidèlement horsW, archiverPC+portable/réhash, préserver rapportinitial. Suite suspendue à arbitrageC7 : sous les contraintes pasmodifier6f3 et pasretireroriginauxavantVERTréel, cette garde ne sait pas accepter les journaux préservés. Il faudrait une capacité de garde explicitement revue, ou une séquence de préservation/retrait des journaux distinctement autorisée ; ED ne modifie pas la garde ni les sources pour fabriquer un vert. Aucun mv/reset/cleanforce/unlink/rm fait, aucunW/T/branche retiré. Le ticket reste ouvert et la qualification produit reste livrée surd395.
