# Rouge nominal baseline2b04 acquis avant implantation complète

GO **C7-ENTRY-PURE-BASELINE-2B04-17nov60**, publié **5e2dfd8bbe**, lu avant
lancement. Source exacte **2b0402076daf726f8a541cc3abe5c0c816a88600**, WT propre
avant entrée ; les quatre pins module/test/stub/interpréteur ont été réempreintés
et conformes avant l'unique commande. RO baseline exactes M/Recherche/TTY et CP
conservées ; la RO CP arrivée ensuite n'a pas changé le GO ni le lancement.

UNE invocation du seul nominal épinglé : **1 FAIL, 0 ERROR, 0 skip, rc1**.
Message exact : `pure entry validation capability is not implemented`.
La trace établit l'import du vrai module partagé execution_entry.py du WT
épinglé et son Refusal(capability-not-implemented), transformé en self.fail par
le seul nominal. Aucune erreur d'import/fixture, aucun faux rouge de backend.
Les46autres fonctions ne sont pas jouées ; aucun retry.

Commande complète, cwd/PYTHONPATH, sources et pins figurent dans les bruts.
Timeout TERM58/KILL2, Python3.14 -B, unittest -v -f et nom exact prévus par le
GO ont été conservés. L'enveloppe monotonic locale va de **886756.505244998** à
**886756.554041811**, soit **0.04879681300371885 s**. Unittest affiche0.000s ;
ce n'est pas la durée de l'enveloppe. Aucune comparaison d'horloges interhôtes.

Racine exclusive hors WT, identique sur LOCAL et PC :
`~/.cache/rag3weaver-build/cx-archi-entry-pure-red-2b04-17nov-r1`.
Neuf fichiers : source/module+test+stub, pins, source-et-commande, stdout,
stderr, résultat et manifeste. Les huit fichiers du manifeste et le manifeste
lui-même ont été réempreintés sur les deux hôtes : **0 écart**.
SHA du manifeste :
`d977e605e15ad599ccf9f1869e06305c171e013f85d615bc7ae60bceb94ebadb`.
Les copies et réhashes sont acquis AVANT toute implantation pure complète.
Les bruts rouges restent figés, sans remplacement ultérieur par du vert.

Ce résultat concerne une API pure nouvelle qui refuse encore la capacité.
Il n'atteste ni image d'entrée physique, ni CP/Common réels, ni SDK, stockage,
processus du protocole, appelants, Cargo ou native. Aucun service/target/cache
n'a été touché ; seuls l'exécution CPU autorisée et l'archivage des bruts sur
les deux hôtes ont eu lieu. Le ticket reste ouvert.

La préparation d'implémentation PURE complète est désormais permise par09726,
mais aucun code de cette implantation n'était écrit au moment de ces preuves.
Nouvelle publication/pins/RO puis GO47 distinct AVANT tout autre test. La revue
source CP558959 momentanément suspendue pour ce GO reste à terminer séparément.
