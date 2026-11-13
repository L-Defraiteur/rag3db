# Qualification isolée de read_scope — préparation

État : préparé, aucun Cargo ni runtime lancé. GO lourd C7 requis après
landing physique ; aucun tour master demandé par cette préparation.
Ticket : [erreurs des worktrees](../tickets/2026-11-13-read-scope-masque-les-erreurs-de-lecture-des-worktrees.md).

## Sources et isolation

Base exacte b0a5177c00bfaf3b37ae21ea0c630c515a19e224 ; patch rebasé
a919ad9d9, équivalent à 674f596c187d0d643d6ffec1d05b38dd00e5a511
(range-diff un patch égal). Fichier atelier_fichiers.rs byte-identique à
674f596c1 ; les trois documents du patch sont également conservés.
Branche et WT neufs cx-ed-read-scope-qualification-r2. Revues statiques
Archi et C7 favorables, tests encore non compilés et non exécutés.

Exécution uniquement sur luciepc, clone/WT correspondant au hash publié,
avec sous-modules épinglés et propres (contrôle récursif). Target neuf dédié
~/.cache/rag3weaver-build/cx-target-ed-read-scope-r3 ; ne jamais employer
cx-target-contenu-derive ni les ressources Arbre. Preuves dédiées
~/.cache/rag3weaver-build/cx-preuves-ed-read-scope-r3. Avant création :
attester absence du target et du dossier de preuves, source propre,
aucun rebase, puis enregistrer hash/tree et état des sous-modules.

## Portée et budget proposés à C7

Un seul superviseur extérieur, budget global 6000 secondes incluant toutes
compilations, contrôles et nettoyage. Appelant dans /var/tmp ; le lanceur y maintient superviseur, driver et finaliseur.
Cargo et chaque ELF ont cwd=crate explicitement. Commande après GO exact :

```sh
cd /var/tmp
python3 "$WT/docs/everything-declarative/13-11-2026/read-scope/lancer.py" "$WT" "$SOURCE_EXACTE"
```

La primitive acquiert un slot lourd Max32GiB et porte reçu/bus/budget.
Ne pas imbriquer un autre poste ou scope. Ce lot fonctionnel n’est pas exclusif ;
C7 autorise le parallélisme avec le lot2a sur un autre slot privé.
RAG3WEAVER_BUILD_MEMORY_HIGH=0 est posé AVANT acquisition du poste. CPU j8, pas de service externe,
pas de NAV/CDP/7879 demandé. SDK29 du magasin épinglé
rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF.
CARGO_BUILD_JOBS=8, CARGO_INCREMENTAL=0, RAG3DB_BUFFER_POOL_SIZE=1073741824,
quatre rôles BURN CPU. TMP/cache/config/state/journaux privés sous les preuves.
Charger scripts/lib_du_magasin.sh et exporter_la_lib une seule fois dans
le driver ; garder_le_target avec la racine crate exacte avant Cargo.

Pilotes [lanceur](read-scope/lancer.py), [driver](read-scope/driver.py) et
[finaliseur](read-scope/finaliser.py) matérialisés ; exceptions fatales, chaque commande
journalisée, timeout borné par le minimum cap de phase/temps global restant,
réserve finale de nettoyage ; premier rouge inattendu STOP sans retry.
Caps proposés : compilation 2400s, chaque témoin 120s, module 600s,
frontières 1800s cumulées, nettoyage 60s, toujours sous budget global.
Si le budget ne permet plus une étape, verdict rouge/incomplet, pas de reset.

## Ordre et juges

1. Construire une seule lib de tests avec --no-default-features --features
   rag3db-native,code,daemon ; extraire l’ELF depuis les messages JSON Cargo,
   archiver commande/source/SHA/features/SDK et inventaire --list.
2. Exécuter séparément, --exact --nocapture --test-threads=1, les quatre
   nouveaux témoins sous daemon_host::atelier_fichiers::tests :
   read_scope_preserves_a_backend_error_instead_of_claiming_absence ;
   read_scope_preserves_a_usages_error_instead_of_claiming_absence ;
   read_scope_refuses_unproven_uniqueness_after_a_real_definition ;
   read_scope_keeps_real_absence_and_ambiguity_after_complete_queries.
   Puis le nominal historique
   the_workshop_reads_a_scope_from_the_index_of_an_attached_worktree.
   Exiger exactement un passé/aucun échec/aucun ignoré pour chaque invocation.
3. Inventorier puis jouer le module complet daemon_host::atelier_fichiers::
   avec le même ELF, --nocapture --test-threads=8. Comparer noms/comptes
   à l’inventaire ; publier séparément les ignorés navigateur déjà déclarés
   dans la source, jamais les compter comme verts. Aucune exemption nouvelle.
4. Frontières : appel direct du script canonique scripts/check_features.sh
   complet, exiger sa conclusion CHECK_FEATURES VERT 8/8 ; target privé
   commun et logs par jeu. Aucun recopiage de matrice dans le driver.
   Cet enfant réexporte lui-même le SDK depuis le LD antérieur au profil
   tests, pour éviter le double préfixe. RAG3DB_LIBS_DIR est fixé explicitement
   à $HOME/.cache/rag3db-libs, indépendamment des XDG privés.
   Compilation et contrôles Cargo sous nice15/ionice3.

Les résultats des témoins ciblés et du module sont deux invocations,
pas des juges indépendants supplémentaires. Les contrôles de features
ne constituent pas une nouvelle globale physique. Le succès éventuel
ne prouve ni la cause du FAILED ancien ni l’exhaustivité des inscriptions.

## Verdict et conservation

Avant verdict composé : superviseur vert/code0/restes[], bus retiré,
quiescence PID/start/exe/cwd du WT et du target, source inchangée, ELF réhashé,
attestation maps SDK29 et événements mémoire avant/après. UNKNOWN n’est pas
une absence. Archiver logs et manifeste SHA sur les deux postes, rouges compris.
Aucun retrait du WT/target avant gardes fraîches et coordination distincte ;
aucun push master avant tour explicite C7 et procédure de passe.


Le ledger hors source est écrit avant les juges (après --list et --ignored) ;
aucun hash de commit auto-référent dans les scripts : le lanceur exige le SHA
exact fourni par C7 et le driver le revérifie. Trois ignorés NAV historiques
sont nommément exigés ; toute variation refuse le lancement des juges.
Le finaliseur ne tue ni ne supprime aucune ressource ; les UNKNOWN sont classés par identité PID/start du registre enfants du confineur,
appartenance au scope et contexte exe/cwd. Une baseline protégée au cgroup
lisible hors scope reste UNKNOWN externe explicitement conservé ; un owned,
un contexte vivant ou un rattachement non excluable bloque le vert. Copie SHA des
preuves vers le second poste à effectuer après finalisation, avant livraison.


### Corrections après revue C7 du pilote6a2

ELF copié par copy2 vers preuves/binaire-lib et SHA comparé AVANT inventaires
et tests. Recherche réelle du hash source court embarqué (offset>=0), ledger
hors target avant juges ; finaliseur rehash copie et original si encore présent.
Une disparition de l’original après check_features doit être documentée dans
elf-apres-features.json ; la copie conservée reste obligatoire et vérifiée.

Lanceur et driver reconstruisent l’environnement par liste blanche (outils,
identité Unix, langue, capacités confineur/bus privé nécessaires). Aucune
variable héritée de service/embeddings/PG/clé API/credentials n’est transmise.
Le jeton de validation n’est pas dupliqué dans le profil archivé.

Pendant chaque phase, surveillance du log actif et des logs features : premier
FAILED, panic, erreur compilateur ou CHECK_FEATURES ROUGE provoque sortie du
driver et arrêt des descendants par le confineur. --nocapture conserve les
messages précédant FAILED ; un arrêt peut néanmoins empêcher une stack complète,
ce qui doit rester explicite. Pas de retry. Les lignes par nom peuvent être
entrelacées sous huit threads ; inventaire et comptes libtest sont comparés.


### Réserves complémentaires Archi sur6a2

Annexe RO f66105e8e reprise section seule dans le ticket, sans ses documents
stales. Maps exigées pendant cible-* ou module uniquement, jamais --list :
PID/starttime recontrôlé après lecture, exe réel et SHA identiques à l’ELF
archivé, cgroup sous reçu et seul chemin librag3db du SDK29 admis. Chaque
observation est immédiatement écrite avec phase et fichier maps ; PermissionError
donne une preuve UNKNOWN et arrête, jamais une observation négative.

Mesure du target par du -sb avant/après chaque phase et toutes les dix secondes
dans les phases actives, taille <=32GiB exigée et journalisée ; contrôle final
indépendant. C’est un contrôle observé périodiquement, pas un quota filesystem
ni une preuve que le pic entre deux relevés est resté sous le plafond. Tout
dépassement observé arrête sans vider le target pour rendre le verdict vert.


### Rouge de lancement7a94 conservé — aucun Cargo

Le lanceur r2 a reçu rc2 avant création du reçu et avant le driver : poste
ne pouvait pas joindre le gestionnaire systemd utilisateur, puis le confineur
a refusé l’absence de plafond effectif32GiB. Aucune compilation ni juge lancé,
aucun target créé ; ce n’est pas un défaut produit ni un test réussi.
Cause de recette : la liste blanche du lanceur avait supprimé l’adresse du
bus utilisateur nécessaire à systemd-run (SSH : unix:path=/run/user/1000/bus).

Correction bornée : avant poste, construire /run/user/UID/bus, vérifier
propriétaire UID/socket et runtime non symlink, transmettre XDG_RUNTIME_DIR
et l’adresse de CE bus au gestionnaire. Aucun endpoint applicatif hérité.
Après acquisition, le confineur remplace DBUS_SESSION_BUS_ADDRESS par son bus
privé ; le driver continue sa liste blanche et ses racines XDG privées.

Les quatre fichiers du rouge r2 sont conservés sur PC et copie locale
cx-preuves-ed-read-scope-r2-rouge-local, SHA tous identiques, manifeste
b00796284c9140c5c3ef5f85bff68074adcdb0ead4f87ddd7cb578e9cfdd1111.
Aucun retrait. Nouvelle source exacte et nouveau GO C7 requis avant lancement
sur WT PC neuf cx-wt-ed-read-scope-r3 et paths target/preuves r3 neufs.
Aucune portée6000 authentifiée n’avait démarré en r2 (aucun reçu créé).

Contrôle complémentaire C7 : bus lui-même non symlink et runtime répertoire
explicites ; sonde manager autorisée en lecture seule (Version), sans changement
de manager/KDE/policy et sans repli autorisé hors plafond.


### R3 : rouge de fixture ; préparation R4 sans reset

Source47181 : compilation verte129s, ledger ELF6f5f5638a606c42fdcf0a7ce93753ac46d3ea8047fbd239b46a317a9bf418f7a
archivé avant runtime. Premier juge rouge : helper sans models.embed/embeddings,
donc load refuse avant le graphe attendu. Panic807 capturée ; aucun autre
juge/module/F8 joué. Superviseur rouge code1/restes[]/bus nettoyé. Registre
900identités PID/start postmortem : aucun owned vivant ; aucune quiescence
universelle déduite. Copie23fichiers SHA identiques deux postes, manifeste
fcbb0596b9fef45ca756f2e409a125a8e70d46b464483c9ff6c51eb02026c9d5, plus
postmortem séparé. Tous rouges et ressources conservés.

Correction TEST seulement, chemin relu statiquement Archi : déclaration
models.embed provider service/granite-278m/dim768, entities:{} inchangé et
workspace sans index. needs_embeddings reste faux : ni service ni modèle
ni index démarré. Assertions de causes non relâchées.

Proposition R4 : WT PC neuf cx-wt-ed-read-scope-r4, target/preuves-r4 neufs.
Lanceur acquiert poste lourd puis apres-poste.py vérifie le poste réel et
calcule floor(908183.114381791-now)-3. Aucun nouveau6000 : deadline R3 conservée
dans driver/finaliseur, refus si <=60s. Scope/bus/reçu authentifiés neufs,
aucun deuxième poste. Compilation/tests cwdcrate et caps min restant.
GO exact C7 renouvelé requis ; aucun R4 exécuté à cette publication.
