# Relogement réversible du target tty — contrat proposé

Ticket : `docs/tty/tickets/2026-11-16-les-commandes-et-options-du-terminal-en-anglais.md`.
Porteur tty. Source seule :
`~/.cache/rag3weaver-build/target-cx-tty-english-aliases-2` ; destination seule :
`~/.cache/rag3weaver-build/archives/cx-tty-aliases-red-20261116/retired-target`.
Le worktree propriétaire `~/git_workspaces/cx-tty-english-aliases` reste en place.

## Frontière et préconditions

Renommage entier sur le même système de fichiers, aucune suppression,
invalidation d'empreintes, réécriture de marqueur ou import d'artefact. Les
inodes anciens restent ceux de l'archive, y compris pour un FD encore ouvert.
Ce contrat ne prouve pas une absence universelle de writer.

Avant action, relever source/destination par lstat : UID propre, racine source
répertoire sans lien, destination absente et parent canonique sans surprise,
même device. Conserver dev/inode/mode/uid/gid/mtime et SHA des fichiers avant
puis après. Comparaison stable immédiatement avant rename ; tout changement
arrête. Ne pas remplacer une destination concurrente : contexte privé, contrôle
frais d'absence et création d'archive exclusivement par cette session.

Les preuves du rouge identifient la création à partir d'un target absent,
source4cc, ELF SHA b2b6101a8face2ebda84400a31ad653444e576167e9198410dc6a64ad83d3015,
Common g4dqpr6q terminé/restes[]/nettoye=true. Archive tar intégrale aux deux
postes comparée SHA56aca2b2488b99a302f0e806724bf58a2677805a9413f8ca9152ac2caf9d09f3.
Relire ces preuves avant action ; aucune ancienne fenêtre de calcul réactivée.

Scan frais exe/cwd/FD : conserver les liens trouvés et les accès opaques.
Un writer OWNED de cette validation ou un inconnu attribué à T/W arrête.
Les PID opaques de baseline non attribués sont consignés UNKNOWN ; ils ne
constituent pas une preuve de writer de ce lot et ne sont ni interrogés par
permission supplémentaire, ni arrêtés, ni qualifiés par leur nom.
La distinction repose sur provenance de création et autorité Common terminée,
pas sur une affirmation que tous les FD du système ont été lus.

## Après relogement

Relire racine et contenus : inode/device/modes/SHA conservés, ancien chemin
absent ; rapport hors T avec UNKNOWN non attribués explicites. Aucun Cargo.
Le futur pilote GO2 créera exclusivement le même chemin T vide et appellera
la vraie garde initiale depuis le propriétaire avant Cargo. Aucun marqueur
écrit à la main, aucun artefact repris de l'archive, un seul target actif.
Le relogement n'autorise pas le futur pilote600 ni un retrait du worktree.
La garde worktree reste obligatoire pour son retrait futur ; son précédent
refus sur T seul reste conservé sans transformation en verdict vert.

## Témoin de frontière

`python3 -B docs/tty/16-11-2026/test_target_relocation.py` : filesystem factice,
FD ouvert conservant le même inode/contenu/mode après rename, nouveau chemin
vide d'inode distinct, refus writer owned ou destination préexistante.
Ce témoin ne valide ni la quiescence du target réel ni les accès UNKNOWN.
Aucune action réelle de relogement n'a été effectuée à la publication.
