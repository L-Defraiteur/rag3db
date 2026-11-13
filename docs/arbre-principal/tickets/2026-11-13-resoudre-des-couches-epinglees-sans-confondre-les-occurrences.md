# Résoudre des couches épinglées sans confondre les occurrences

- **État** : ouvert ; résolveur pur2a et modulepublication qualifiés CPU sur69dc/piloteabaf, adaptateur natif compilé seulement ; revue/intégration en attente.
- **Famille** : produit.
- **Gravité** : capacité manquante ; pas encore de résolution de couches épinglées.
- **Atteignable en service** : non pour la nouvelle primitive ; les outils lisent la vue L6 existante sans pile de couches.
- **Touche rag3weaver** : oui, résolution pure de publications/couches ; aucun stockage ou chemin de recherche modifié dans ce premier diff.
- **Pour** : arbre-principal ; revues C7/ED/Recherche/Mémoire.
- **Signalé par** : suite cohérente pyramide demandée C7 après landing b0a5177c0.

Parent [ancres](2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md).
Contrat [lots2a/2b](../11-11-2026/03-contrats-et-lots-des-vues.md).

Avant diff produit : déclarer une pile immuable de révisions de couches, chaque
révision liée à origine/publicationP existantes et parent exact. Résolution
pure de valeurs d’occurrences opaques, identité occurrence distincte du contenuH
qu’elles portent. Tombstone occurrence/fichier masque les ancêtres ; override
local prévaut sans fusionner les occurrences partageantH.

Réutiliser PublishedDeclaration(P) et QualifiedPhysicalState(neuf) pour
construire les références, aucune nouvelle génération parallèle ni création
d’index/base. Refuser doublons/parents absents/cycles/origines différentes,
vue inconnue, et sélection par nom ambigu entre plusieursP. Résultat lié à
un ensemble immuable et provenance par occurrence. Pas de certificat physique
forgé depuis la déclaration, pas d’accès index autorisé par cette primitive.

Témoins CPU à écrire avant implémentation, NON joués : f parente/g fille/h
ancre, suppression héritée occurrence/fichier, réintroduction descendante,
ancre avancée sans effet implicite, mêmeH/deuxoccurrences, inconnue/ambiguë,
parent manquant/cycle/origine différente, champs/positions/provenance propres.
Sous-lot2b reste distinct : masques avant top-k dense/creux/FTS et arêtes aux
deux bouts visibles, intégration L6/outils/stockage et durabilité non qualifiées
par un résolveur pur. Aucun clone de base, legacy, modèle, service ou M3-M5.

Fin de ce diff : API et contre-exemples publiés pour revue, recette CPU bornée
à proposer ensuite ; aucune exécution Cargo/DB/service sans GO exact.


### Précisions après revue statique5ec8

Tombstone « scope » signifie ici occurrence de l’entité Scope par UUID, PAS
cellule(org,project). OccurrenceKey doit être unique dans l’origine choisie.
FileKey est une identité logique fournie : l’appelant inclut domaine/cellule
si nécessaire ; deux cellules nommant a.rs par la même clé seraient masquées
ensemble. Le résolveur ne peut reconstruire un domaine absent de ses entrées.
Des racines indépendantes d’origines différentes sont admises ; seule une
arête parent entre origines différentes est refusée.

Le juge existant ajoute remplacement fichier tombstone+valeurs locales DANS
la même révision et deux FileKeys qualifiées partageant le chemin affiché,
ainsi que racines indépendantes multi-origines. Compte inchangé4fonctions,
toujours NON JOUÉES. Validation cycles O(n²) dans une chaîne, aucun coût mesuré.
Deux propositions de fenêtre ont été reçues(900sRecherche/3000sED) ; ni durée
ni compilation autorisées avant décision/recette/GOexact C7 et disponibilité.


## Pilote concret préparé après arbitrage3000 C7

Application69dc figée, contrôle en WT/branche séparés ; [recette11](../13-11-2026/11-recette-qualification-couches-2a.md).
Build1800→inventaires4exact0ignored/ELFJSONcopiéledgerpublié→4exact60
chacun→publication120→checknatif900 dans même échéance3000 après poste
réel. Attestation maps/profil déterministe au main du premier exact via gdb,
puis continuation de ce même test, aucun test ou modèle supplémentaire.
Sup/driver/final horsW, environnement liste blanche, cgroup32/HIGHmax/CPUj8
etSDK29, target privé neuf absent avant GO. Sources et pilote propres figés.
Compilation adaptateur ne vaut pas runtime natif. Revue+GO du couple SHA
encore requis ; zéro Cargo/rustc/target/DB/service créé pendant préparation.


Précision C7 intégrée au pilote : bus manager utilisateur initialisé dans
l’environnement du lanceur AVANT poste puis sanitize du driver ; aucune
création/redémarrage de service. CPU features[] sans exportSDK ni exigence
mapsnative. SDK29/export/profil réel/pool1G limités au CHECK natif,
compilation≠exécutionSDK/DB. ED occupe son propre slot lourd/target, jamais
partagé avec cette qualification2a. Aucun run avant GO du pilote révisé.


### Réserve mémoire b38 : finaliseur et dernière borne

Revue RO identifie un intervalle après la vérification d’échéance : scan
/proc puis deux hashes ELF coûteux pouvaient terminer après END et encore
publier VERT. Aucun run n’a exposé ce cas. Correction du pilote uniquement :
relire monotonic après ces contrôles et immédiatement avant la publication
composée ; expiration impose ROUGE. Aucun reset ni élargissement de durée.
Nouvelle revue exacte requise avant GO, sourceapp69 inchangée.


### Réserves ED pilote5a0a avant GO

Le détecteur lisait seulement « test ... FAILED » ; sous nocapture un panic
peut être suivi de FAILED seul, avant rc final. Ajouter détection en activité
du panic/FAILED autonome pendant les juges, aucun test should_panic/catch
attendu dans ces deux modules. Conserver log et stop immédiat du périmètre.
Le pont manager doit aussi refuser runtime/socket liés ou propriétaire socket
étranger ; vérifier lstat et stabilité autour du Ping. Aucun service redémarré.
Réserves de recette, aucune exécution ni défaut2a établi ; nouveau couple à
faire relire avant GO.


## Qualification CPU bornée obtenue

GO C7 exact APP69dc/piloteabaf, scope/finaliseurVERT214.675462s/3000.
4exacts1/0/0 puis modulepublication10/0/0(4déjàjugés) :10fonctionsdistinctes,
14invocationsbrutes ; check adaptateurnatif rc0, COMPILATION seulement.
67fichiers SHA identiques deuxpostes. [Rapport12](../13-11-2026/12-preuves-cpu-couches-2a.md)
avec ledger/ELF/caps/memory/maps et limites. Pas de persistance/L6/top-k/outils
qualifiés, aucune DB/service/modèle ouvert. Ticket reste OUVERT pour revue
et décision d’intégration/qualification complémentaire, pasGOmaster/retrait.


## Préparation union après ED — aucun run autorisé

C7 juge preuves72db suffisantes dans portée2a, attend landingED avant notre
intégration. [Plan13](../13-11-2026/13-plan-union-couches-apres-ed.md) :
branche de rebase NEUVE depuis72db après fetchED, audit range-diff+identité
fichierslayers69 et préservation master. Source entrante Rust justifie
qualification union ciblée4exacts/publication10/checkadaptateur avec notre
cache privé conservé, pas globalephysique. Recette sourceunion/controlSHA
à publier et GOcréneau exact nécessaires avant runtime. Proposition1200s
à arbitrer C7, aucun reset de l’ancienne portée. Sources69/abaf/preuves/
target conservés intacts, tickets ouverts jusqu’à intégration.


Décision C7 complémentaire : l’arrivée Rust ED impose F8 CANONIQUE et lib
entière jeu large avant juges2a/modulepublication sur l’union ; ciblé seul
initialement proposé ne vaut PAS dispense. Plan13 remplacé en conséquence,
proposition4800s à arbitrer sur coûts/cache ;2ELFs(large/CPU) et ledger avant
runtime. Checkadaptateurminimal déjà inclus au jeu8 F8 --checktests, pas
commande redondante sans preuve distincte. NAV/lectures/PG/globalesphysiques
non rejoués explicitement selon C7, sans résultat neuf inventé. LandingED
confirmé4e2e2bfbb7c8ce670a49a06a36580d481d9abbc0, prépare rebase neuf,
aucun run avant recette/SHAcouple/GOcréneau.


Rebase union NEUF sur ED4e2e effectué sans conflit,9range-diff égaux,3fichiers
2a identiques à69 et entrant RustED préservé ; overlayproduitexact sansparasite,
7930apportsmasterhorschevauchement+suppressionspréservés. Rapport14/audit
conservés. Cette préparation n’étend pas les preuves72db à l’union ; nouvelle
recette F8/liblarge/CPU2a à publier et GO exact requis avant run.


BilanED940adMDseul intégré AVANT gel application par rebasebrancheWTneufs
cx-arbre-couches-union-r2 depuis2009 :10patcheségaux, produitidentique2009,
masterpréservé. R1nonjouée supersédée. Budget4800/caps du plan13 approuvés
pour préparer le pilote seulement ; aucunGO exécution ni preuve rétroactive.
