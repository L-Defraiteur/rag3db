# Retrait gardé des ressources privées des fixtures

Master a reçu **b0a845bb4877328eada37951c174e3d208986630** après ÀTOI exact C7,
vrai poste push et trois contrôles relus aux deux postes : arbre propre, aucun
rebase, HEAD=origin/master=FETCH_HEAD. Aucun rejeu ni déploiement. Le
[ticket de nettoyage](../tickets/2026-11-16-retirer-les-ressources-privees-des-fixtures-apres-atterrissage.md)
a été publié avant tout retrait ; il reste ouvert pour les quatre arbres conservés.

## Retraits effectivement réalisés

- Portable : **14 worktrees privés** `cx-wt-memoire-script-fixtures*` retirés,
  **14 branches locales** supprimées avec `git branch -d` seulement, après
  vérification exacte de leurs têtes sur les branches distantes poussées.
- PC : **2 worktrees propres** détachés, intégration b0 et pilote383, retirés.
- **Aucun --force**, aucun -D, aucune suppression distante, aucun prune/deinit,
  aucun clean/reset et aucun changement dans les ressources d'autres lots.
- Target privé PC `~/.cache/rag3weaver-build/target-cx-memoire-script-fixtures`
  retiré : **5 787 059 841 octets réguliers logiques**, 22 730 entrées avant retrait.
  Ce total ne prétend pas mesurer les blocs libérés ni le gain net après archives.

Avant chaque worktree : copie complète des sources en tar PAX gzip, copies
fidèles des administrations détruites (index/refs/reflogs/config/objets compris),
réplication et réempreinte aux deux postes, contrôle source propre et tête exacte,
scan exe/cwd/FD/cmdline, **rapport neuf VERT lu de la garde courante**
SHA256 **6f3dfceb17c9703029d2f4f26385f9aa21820c84bc79c518e1338b6d0f36188c**,
puis second scan immédiatement avant `git worktree remove` normal. Aucun refus
partage/sous-module contourné. Les UNKNOWN globaux sont conservés dans les scans,
pas assimilés à des propriétaires vivants ni à une preuve d'absence globale.

Pour le target : racine canonique/non liée/UID propre, dev/inode/mode et contenu
exact des deux marqueurs attestés, propriétaire source PCr4 identique, composé
union GREEN/autorité terminée/bus absent et canonique SHA intact relus. L'ELF courant
c397 et l'ancien6f12 ont été réempreintés dans leurs copies PC **et** portable.
Inventaire complet du target et empreinte, répliqués avant retrait ; inventaire et
marqueurs identiques au dernier contrôle, deux scans frais sans référence au target.
Aucun autre cache ni modèle, service ou base n'a été touché par ces retraits.

## Archives et limites conservées

Racine identique aux deux postes :
`~/.cache/rag3weaver-build/archives/cx-memoire-script-fixtures-cleanup-16nov-v1/`.
Sous-dossiers `lucie-tablette/` et `LuciePC/` : **16 archives source**, total gzip
**2 536 625 118 octets**, administrations et inventaires. Rapports individuels dans
`reports-lucie-tablette-v2/` et `reports-LuciePC-v2/` ; premiers rouges PC dans
`reports-LuciePC/`. Scripts opératoires privés, scans, métadonnées target et bilan
répliqués sans les introduire dans le lot produit.

Manifeste final extérieur à cette racine :
`~/.cache/rag3weaver-build/archives/cx-memoire-script-fixtures-cleanup-16nov-v1-manifest.json`.
**363 entrées types/modes/UID/GID/mtime_ns/SHA256 identiques aux deux postes**,
SHA256 **b7d35c4e2bbf2dd67c9a6804800f612a0b681256791d15285fef2c0d3dcae3d4**.
Les anciens manifests intermédiaires sont historiques ; le manifeste final garde
les deux générations de snapshots et les rapports rouges sans les repeindre.

Conservés : source734 locale propre, archive physique734 PC et son administration,
toutes les branches distantes, preuves historiques et rouges, copies ELF6f12/c397,
worktree documentaire de ce nettoyage, et **quatre worktrees PC** où la garde
refuserait encore des fichiers ignorés :

- `cx-memoire-script-fixtures-r4` : `extension/rag3weaver/target/` ignoré ;
- `cx-memoire-script-fixtures-resume-pilot-r1` : `script-fixtures-resume/__pycache__/` ;
- `cx-memoire-script-fixtures-second-only-pilot-r1` : `script-fixtures-second-only/__pycache__/` ;
- `cx-memoire-script-fixtures-union-pilot-r1` : `script-fixtures-union/__pycache__/`.

Ils sont préservés tels quels, sans extraction ou effacement du contenu ignoré
pour obtenir artificiellement un arbre propre. Leur traitement reste au ticket.

## Incidents du lanceur, sans masquage

Le premier contrôle PC a refusé deux retraits : après les snapshots, un `git status`
sans GIT_OPTIONAL_LOCKS=0 a changé **uniquement mtime du répertoire administratif**
(diff exact `.` ; contenu index inchangé). Aucun retrait sur ces rouges. Le lanceur
utilise désormais GIT_OPTIONAL_LOCKS=0 ; snapshots v2 distincts, réplication et
réempreinte refaites, nouvelles gardes vertes avant les deux retraits. Les premières
copies, rapports rouges et le diagnostic restent conservés.

Une première copie scp d'utilitaires a visé par erreur le cache commun PC.
`prepare.py` était neuf et a été déplacé dans l'archive privée. Le fichier
`~/.cache/rag3weaver-build/garde_worktree.py` existait depuis le15nov ; il a été
réécrit avec la version courante6f3 sans relever son SHA précédent. Aucune identité
avec son ancienne version n'est donc affirmée. Aucun outil de ce nettoyage n'a été
exécuté depuis cette copie commune ; elle n'a pas été supprimée ni restaurée avec
une version supposée. Incident communiqué immédiatement à C7, métadonnées et SHA
actuel dans `common-cache-copy-incident.json`. Les autres fichiers du cache commun
ne sont pas inclus dans le périmètre de retrait.
