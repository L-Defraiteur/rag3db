# ED — accords de commande, 12 novembre 2026

## État

Le [ticket d'accord extérieur](../tickets/2026-11-11-un-accord-de-commande-hors-domaine-redemande-le-meme-accord.md)
reste ouvert tant que la passe globale et l'intégration ne sont pas attestées. Lot autorisé
par Lucie via orchestration : ED possède commande.rs et run_nodes.rs ; archi possède le
témoin navigateur et le petit helper de configuration dans le module de tests atelier.

Candidat regroupé propre publié : `4df34c5b06063afaf33d6157eaaf2c05f89e00d8`, branche
`cx-ed-accord-hors-domaine-2`, base master `8cc7e023b`. Worktree isolé aux deux postes :
`~/git_workspaces/cx-ed-accord-hors-domaine`. Target privé luciepc :
`~/.cache/rag3weaver-build/target-cx-ed-accord`. Aucun build sur le portable.

## Ce que le correctif garantit

Un accord retient la ligne complète exacte et le domaine canonique. Il ne devient ni un acquis
de famille ni un droit sur les sous-commandes isolées. Jugement et laissez-passer composent
la même primitive ; les chemins synchrone et en fond l'utilisent. Le mode standard ne reçoit
pas d'accord différé. Une portée bornée CetteFois est consultable sans consommation au jugement,
puis consommée atomiquement lors de la création des laissez-passer ; deux appels concurrents
ne peuvent obtenir le même accord unique.

L'accord de ligne ne donne aucun droit noyau supplémentaire. Le témoin de reprise déclare
`workspace.sandbox.extra_read` pour le seul script extérieur, avant ouverture, avec contrôle
de la configuration réellement chargée et sans écriture extérieure. Le script reste dehors ;
la variante ajoute un argument, doit attendre un nouvel accord et ne produit aucun effet.

## Preuves ciblées réellement lues

- `6cbbb22a9` : rouge 0/2, reprise extérieure et collision entre argv au même affichage.
- `b4c1d13ef` : rouge 0/2, accord CetteFois permanent et deux autorisations concurrentes.
  Aucun laissez-passer destructeur exécuté.
- `aed5ef115` : commande **50 réussis, 0 échec, 1 helper ignoré** ; run_nodes **18/0/0**.
  Le helper `role_hote_qui_sort` est exercé avec `--ignored` par son test parent.
- `157a2f272` : témoin causal du vrai bac **1/0/0**. Les fichiers sont lisibles côté hôte ;
  Landlock est exigé actif. Sans lecture déclarée, le processus accordé rend une erreur
  Permission denied ; avec le seul script déclaré, il rend sa sortie ; son frère reste illisible.
- Navigateur initial `5611fc98e` sans lecture déclarée : rouge produit **0/1/0 en 1,13 s**
  après clic, deux commandes identiques et nouvelle demande d'accord. Les refus de démarrage
  Chrome antérieurs sont des montages, pas cette preuve produit : TMPDIR trop long pour le socket.
- Navigateur `aed5ef115` sans lecture déclarée : rouge **0/1/0 en 30,73 s** ; accord franchi,
  wait_output atteint, puis Python Errno13. Ce résultat reste conservé ; il n'est pas vert.
- Baseline déclarative `8b9499006` : rouge **0/1/0 en 0,90 s**, malgré extra_read du script.
  Candidat `996af8ee4` : vert **1/0/0 en 1,32 s**, même helper et même témoin byte-identiques.
  Vrai clic, commande identique, sortie différée et cwd réel, puis variante sans effet.

Preuves luciepc : `~/.cache/rag3weaver-build/I-luciepc/accord-20261112/` pour ED ;
`~/.cache/rag3weaver-build/cx-accord-exterieur-<source>-<étape>-20261112/` pour archi.
Les chemins précis et les préconditions sont consignés dans le ticket canonique.

## Passe et suites

Première passe globale sur 4df interrompue avant la lib large : features cinq jeux, lib native minimale et lib large,
navigateur complet, lectures et démon hôte. Lib29 épinglée, -j8, plafond mémoire32G,
target privé de budget32G, délai3600s après acquisition de poste lourd. Une mesure moteur
prioritaire a été respectée ; un ciblé supersédé en file a été annulé avant Cargo, par son pid
propre. Pas de concurrence Cargo dans le target ED.

Premier lancement global arrêté avant tests : `createdb` absent de l'hôte. Log et verdict
conservés ; le lanceur utilise désormais le client du conteneur PostgreSQL existant pour la
seule base privée `cx_ed_accord_20261112`. Santé réelle du service sparse7879 vérifiée.
Aucun nouveau service ni GPU. Premier autre rouge : arrêt et diagnostic, pas de rejeu inchangé.

Le [lot de coordination clos](../../tickets/closed/2026-11-12-paralleliser-les-suites-autorisees-sans-chevauchement.md)
a fixé les frontières ; il ne ferme pas le ticket fonctionnel. M8 est porté par mémoire,
avec sa [revue canonique](../../memoire/12-11-2026/01-relecture-m8-adoption-fusion-et-reprise.md).
Le lot équipier figé et le rouge Gemini restent inchangés ; aucun nouvel essai Gemini,
aucune intervention sur la mémoire réelle ni sur un autre lot.

## Régression de compatibilité corrigée avant la passe finale

La relecture a identifié un refus shell ajouté par le correctif, absent de l'ancien
`accorder_ligne`. La première passe a été arrêtée : features5 et native1594/0/15 restent des
preuves partielles sur4df, pas un verdict global. Groupe arrêté et quiescence target relevés.
Rouge décisionnel `a2db2abc3` : 0/1, aucune commande shell exécutée. Correctif
`9a1a8539c` : seules les décisions Refuse empêchent l'accord littéral ; la portée shell
CetteFois est consommée par la primitive atomique existante. Variantes, élévation et
substitutions non réduites restent refusées. Ciblés : commande51/0/+1helper et run_nodes19/0/0 ;
plafond cgroup effectif lu34359738368. Rebase final attendu sur master3b2eab7ec avant la
nouvelle passe ; aucune preuve partielle ne sera réattribuée à cette nouvelle source.
