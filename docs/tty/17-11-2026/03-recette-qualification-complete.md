# Qualification complète du lot anglais : recette pour revue

Ticket porteur toujours ouvert :
`docs/tty/tickets/2026-11-16-les-commandes-et-options-du-terminal-en-anglais.md`.
Préparation autorisée par C7 après revue des preuves 4bc72f ; aucun runtime ici.
[Plan et empreintes physiques](qualification-plan.json).

## Couple et audit

Candidat produit utile : correctif36b121e6c0, témoin vert source1944b44dfd,
preuves4bc72fdeb1677a7da65f82a6cc26361b46e13c29. Même worktree/branche7,
propre avant préparation. Fetch origin/master relu :99fef92ce6ee9a0007eaf99c7c3a6a3789061c57.
Cette base est déjà ancêtre : aucune union, aucun nouvel arbre nécessaire.
Les anciens WIP restent intacts. Diff produit exactement sept fichiers tty
(README, trois sources, trois fichiers de mesure), aucun Cargo.lock modifié.
Les quatre entrants Vertex déjà audités sont présents dans la base99 ; la future
lib entière devra exercer leurs témoins, pas porter le vieux F8source6d.

`required_suites.sh origin/master HEAD` relu : sortie vide/rc0. Les chemins tty
ne déclenchent aucun glob de required_suites.txt ; e2e_code_sync n'est pas
imposé par cette table. Cela ne dispense pas du canon proposé ci-dessous.
Diff59→master99 sous src/extension/vector/tools/rust_api/CMakeLists/cmake vide
.
SDK59 compatible au périmètre moteur ; aucun nouveau moteur à bâtir.

SDK physique rehashé, magasin explicite `~/.cache/rag3db-libs/`, identifiant
rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF :
so b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101,
vector1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561.
Pas de bascule de courant ni de changement SDK requis.

## Passage canonique proposé

Copie cache `passe_epinglee.sh` == source tools/poste, SHA256
2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d.
L'appeler directement, sans réécrire ses journaux ni produire un résumé à sa place.
Les argv/env exacts figurent dans le plan JSON : témoin `e2e_tty` avec filtre
vide, puis SUITES=e2e_lectures,e2e_code. run_e2e inclut les ignorés : les quatre
tests tty doivent réellement être joués, 0ignoré ; la suite tty n'est pas
rejouée par une deuxième entrée. Puis F8, preuve de revérification, lib entière
jeu large et navigateur distinct, comme le script canonique les exécute.
Le navigateur ajoute sa feature ; ses ignorés attendus dans la lib sans cette
feature ne valent pas un succès navigateur. Tout autre saut doit être expliqué
et ne peut rendre vert une capacité absente.

Le canon impose CARGO_BUILD_JOBS=8. Ce n'est donc pas une recette j4 globale :
32 cœurs disponibles relevés, huit jobs laissent plus de deux cœurs libres.
Ses consommateurs déclarés LOCAL sont les tunnels embarquement7979/7980 et
le PostgreSQL de test5433 du lanceur ; pas de requête préparatoire ici. Le GO
devra autoriser explicitement ces capacités. Absence de service/capacité :
refus conservé, aucun démarrage ni recours à un provider réel implicite.
Le canon ne pose PG que sur PC ; le pilote LOCAL devra épingler explicitement
l'URL de la base de test attendue avant la lib entière. Aucun secret lu.
Les paquets React locaux sont présents ; leur fermeture et le navigateur
exact devront être épinglés dans le Q final avant lancement.

Ensuite, même target : lib tty entière vue par défaut via cargo_garde -j4,
build binaire tty neuf -j4, puis harnais réel complet (18phases attendues,
accord exact/reprise/kitty/retoursligne/commande!/5thèmes). Composer le pilote
harnais déjà relu : archivage brut par phase, chemin état privé unique,
arrêt au premier vrai ROUGE par pidfd de l'autorité Common vérifiée. Aucun
ancien ELFcb24 ou ancien harnais934 ne remplace le binaire nouvellement bâti.

## Provenance, bornes et coût

TargetLOCAL unique conservé, propriétaire réel nouveauW/tty :2294Mio relevés,
plafond32Gio. Reprise vers le crate racine puis retour tty uniquement par les
vraies gardes. Avant toute mutation : archive/ledger stable, copie du testELF
vert8cee et historiquecb24, reçus terminés, scan frais sans exemptions larges.
Le ledger post1944 et les RAWrouge/vert restent préservés ; pas de purge.
États XDG/fixture/TMP sur disque privés, HOME inchangé, aucun arbre Lucie touché.

Proposition :3600secondes TOTAL, file poste comprise, mémoire32G/HIGH0.
Watchdog OS extérieur exec sans travail après, réserve TERM/KILL et marge
incluse dans3600 ; après poste, Common reçoit MIN(reste frais -15s), pas un
nouveau budget. Codes/receipts/quiet/hashes/publication fsync avant même END.
Le premier refus termine la tentative, sans réessai ni second coupon caché.

Coûts réellement connus : précédent F8+build+harnais arrêté en451,706s
(source historique, ce n'est pas la durée d'une qualification complète),
harnais934 seul146,660s ; coupon ciblé1944 10,609s. Le coût de la lib entière
et du navigateur sur union99 n'a pas été mesuré dans cette session.3600s est
une borne proposée, pas un résultat ni une garantie d'achèvement.

## Livraison du pilote et conditions avant GO

Cette recette est soumise d'abord à RO de périmètre/capacités. Elle n'est pas
un lanceur autorisé : aucun Q complet ni nouveau runtime n'a été créé.
Après accord sur cette recette, publier le pilote composé exact et son Q
horsW avec toutes empreintes, chemin journaux canonique NEUF, journal stages
séparé, état target, environnement whitelist et capacités nécessaires. Revue
TOTALE source+Q puis GO distinct, jamais réemploi des fenêtres600échues.
Le pilote relira aussi les résultats effectifs de chaque étape canonique :
rc0 seul ne prouve ni suite jouée ni preuve de compilation ni navigateur rendu.
Le résumé canonique reste écrit exclusivement par le script canonique.

Le ticket reste ouvert jusqu'à qualification réelle et landing explicitement
ordonné par C7. Pas de push master ni retrait compris dans cette préparation.
