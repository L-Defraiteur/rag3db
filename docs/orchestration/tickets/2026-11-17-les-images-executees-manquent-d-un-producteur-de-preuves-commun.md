# Les images exécutées manquent d’un producteur de preuves commun

- **État** : ouvert, 17 novembre 2026 ; frontière documentaire relue, faisabilité et mécanisme physique non établis.
- **Famille** : produit
- **Gravité** : P2, bloque la qualification globale TTY ; aucun défaut d’ELF effectivement exécuté attribué.
- **Atteignable en service** : non établi ; cette demande concerne les passes de qualification attribuées.
- **Touche rag3weaver** : oui, ses pilotes et preuves de qualification, pas une fonctionnalité utilisateur livrée.
- **Pour** : orchestration ; rag3weaver-archi pour l’étude de faisabilité source seulement ; TTY consommateur, Codeparsers propriétaire du relais de publication.
- **Signalé par** : TTY, Mémoire, Codeparsers et Archi ; arbitrage orchestration après leurs revues documentaires.

## Constat et preuves

Ticket parent TTY : docs/tty/tickets/2026-11-16-les-commandes-et-options-du-terminal-en-anglais.md.
Contrat documentaire961c873bb9d74408aa892d7d6799737047882650, note11 etJSON inactif : trois RO documentaires favorables M/CP/Archi, pas une faisabilité physique. Vingt spécifications P01–P20, NON ÉCRITES/NON JOUÉES ; neuf positions racines proposées, pas neuf processus observés ni couverture exhaustive des descendants.

Validateur hermétique fe01 : quatorze témoins verts sous GO distinct, preuves4a6ccde9e281690a97fd3f151f77749985a2a160/RO M ; ELF_PROVENANCE resteVIDE, Q4 execution_ready=false. Les copies/ACK du relais, PID/start et brackets périodiques229b ne prouvent pas à eux seuls chaque image réellement exécutée. PID/start n’identifie pas une génération d’exec ; tentative échouée, image remplacée, fin d’image et wait du processus sont distincts. SHA de l’image ne qualifie pas ses bibliothèques SDK dynamiques.

Codeparsers conserve seulement relais/ACK/Window. C9constante36verts+3rouges et8rougesWindow sont acquis ; capacité4/Windowenpréparation, aucun producteur physique livré ou attribué par ces résultats. Aucun pilote ne doit fabriquer le reçu manquant depuis le catalogue.

## Prochaine action autorisée : étude source et décision, pas implémentation

Archi retrace les primitives réellement existantes de construction des commandes, confinement/Common, publication et observations runtime. Comparer précisément ce qui se prouve par construction du runner de confiance et ce qui exige observation d’une image au vrai exec. Ne pas affirmer une observation PID/image lorsque la preuve est seulement constructive. Le canon2150 reste inchangé ou HOLD explicite.

Produire une proposition unique de raccord minimal commun, ou une impossibilité/lacune explicite : contenu/identité/occurrence/générationexec, rattachement au plan préautorisé, phase/source/inventaire/RAW, perte d’événement, descendants/couverture, rehash avant/après et copie de la bonne image, clocks propres aux hôtes, annulation/END/cleanup attribué. Les capacités SDK dynamiques restent une attestation distincte au PID réel. Ne pas proposer un observateur ad hoc TTY ni dupliquer la boucle du relais.

L’étude doit annoncer les coûts et effets : conserver un débogueur pendant les tests peut changer leurs délais (ancien cycle rouge sous GDB, causalité non démontrée). Proposer si possible une frontière d’observation avant les tests puis libération ; ne pas tenir la sonde active dans une mesure en la déclarant neutre sans preuve. Aucun chiffre de coût inventé. Pas d’accès hors sous-arbre de processus de qualification explicitement attribué, ni privilège/sandbox modifiés.

Référencer les API/source primaires et la surface minimale conditionnelle, les limites de garantie et les témoins à prépublier au vrai raccord. Si le contrat ne peut être tenu avec canon/confinement/bornes demandés, STOP et arbitrage C7 sur garantie/coût/surface avant tout code. TTY fournit le contrat consommateur ; Codeparsers garde le noyau publication ; rôle producteur et adaptation Common futurs restent conditionnés à cette revue.

## Interdictions de cette préparation

Aucun code ou prototype de producteur, réservation de chemins, test/CPU/processus de qualification, traceur, proc d’autres sessions, signal, build, Cargo, native, DB, provider, service, slot, SDK/cache partagé ou target modifié. Aucun GO3600, globalArchipasse, API physique ou liveness n’en découle. Aucune reprise de C1, aucun ancien GO/END réactivé.

## Fin du ticket

Un raccord commun effectivement qualifié par témoins publiés avant correction, admission distincte du vrai montage, artefacts physiques source/exec/générations/bornes/cleanup relus, et intégration dans le consommateur autorisé ; ou abandon explicite documenté remplacé par une garantie plus limitée acceptée. Une étude documentaire ou un seul validateur virtuel ne ferme pas ce ticket.


## Arbitrage C7 après étude a5281ec4e4 et contrelectures

Rapport45 relu par C7 ; avis documentaires exacts TTY et Mémoire favorables à son bornage, pas à une capacité livrée. Le candidat ptrace événementiel persistant ne démontre pas sa couverture des descendants (`CLONE_UNTRACED`, détachement). Nous retenons pour le prochain lot une **invocation d’artefact déclaré avec confirmation de son image d’entrée**.

La preuve lie le catalogue/source/profil préautorisés, l’objet effectivement entré, une copie de son contenu, les arguments et le contexte, le PID/start de l’invocation, ses sinks RAW et son vrai wait. L’entrée doit être confirmée au vrai raccord : un FD, une intention d’appel ou un EOF de pipe ne constituent pas seuls cette confirmation. La stabilité des bytes jusqu’à cette confirmation reste à établir ; une barrière au premier exec réussi est une piste concrète, avant libération du programme.

Le résultat est celui de **l’invocation du programme lancé depuis cette entrée**. Son wait n’est pas présenté comme le code de retour d’une image A restée inchangée pendant toute sa vie ; les octets des sorties ne sont pas attribués individuellement à un PID écrivain. Les programmes et sources qualifiés sont déclarés et revus ; leurs sous-processus peuvent faire partie du comportement du test. Cette qualification n’est pas une preuve contre un programme arbitraire fabriquant ses propres sorties. Ressources et arrêt du sous-arbre restent portés par le confinement, indépendamment de l’exhaustivité d’une trace d’images.

### Changement explicite de contrat pour TTY

Les vingt spécifications de961 restent une proposition historique plus forte. **Le suivi exhaustif des futurs exec et descendants de P17–P20 ne sera pas une condition implicite de livraison du lot anglais.** Le consommateur doit publier une nouvelle version du contrat indiquant ce bornage. Aucun de ces quatre points n’est déclaré satisfait par la preuve d’entrée ; la couverture ultérieure reste UNKNOWN. L’observation positive d’une image d’entrée étrangère au plan refuse, et un B ne devient pas attendu depuis l’actual pour sauver la passe. Les garanties de P01–P16 applicables aux racines et aux copies restent à porter au vrai raccord, avec une table explicite de portée ; les spécifications non remplies ne sont pas comptées vertes.

L’observation exhaustive de toute génération, si un usage futur la requiert, devra faire l’objet d’un lot et de coûts explicites. Aucune implémentation ptrace permanente, seccomp, privilège, sandbox ou journal de tous les syscall n’est autorisée pour contourner le problème dans ce lot.

### Surface autorisée à préparer

Archi porte le contrat/API et les témoins du producteur partagé ; TTY reste consommateur et CP garde publication/ACK/Window. Préparer un module commun de validation sous scripts/, sans boucle de publication parallèle ni logique anglaise. Le mécanisme concret de confirmation doit venir avec son protocole précis avant implantation ; considérer les barrières de démarrage déjà utilisées (exec/main, libération avant tests, parent naturel et wait réel) comme sources réutilisables, jamais comme réactivation de leurs anciens pilotes, délais ou preuves.

Les quatre sites directs de run_e2e et le raccord Cargo runner sont acceptés comme **surface transitive proposée** du lot, à réserver avec Recherche avant de les modifier. Le SHA2150 conservé ne sera pas annoncé comme preuve d’un parcours inchangé. Chaque phase doit déclarer la propagation d’environnement (notamment LD modifié par Cargo), argv, PID du runner/enfant, sources/features, sorties et signaux. Les faits SDK chargés restent séparés au PID et au point réellement observés. Une racine non couverte n’est pas qualifiée par les autres ; auxiliaires et descendants non observés ne sont pas inventoriés fictivement.

Premier travail autorisé : publier un contrat exécutable/API minimale et des témoins hermétiques préalables au vrai futur raccord, avec tableau de portée et limites. Prévoir substitution de chemin/contenu, succès d’entrée à confirmer, échec/mort avant confirmation, copie/RAW/wait liés, erreur et disparition du producteur, END pendant copie/attente/finalisation, puis limite explicite A→B après libération. Des observations de processus sont des modèles dans cette première étape ; elles ne qualifient pas le noyau. Publier avant toute exécution, RO puis GO CPU distinct. Aucune implantation opérationnelle ou modification des quatre appelants avant cette revue de protocole/surface, aucun prototype OS/native/build/proc/signal/test par cette seule préparation. Un futur petit pilote réel et les passes des appelants auront leurs sources, pins, budget et GO propres.

Le ticket reste ouvert. Cet arbitrage choisit la sémantique d’invocation et ses limites ; il ne fournit encore ni API physique qualifiée ni GO global TTY/Archi, ni autorisation de master ou de retrait.


## Préparer la première implantation pure sans faux rouge d’import

Le contrat5cae3c31731dea3348be2c16fe96ed256fe567ff et ses47témoins ont troisRO exactes DOC/APIpure CP/M/TTY favorables ; aucun module runtime n’existe. Archi est autorisé à publier d’abord un module partagé **purement déclaratif/devalidation**, avec Refusal typée et une version initiale sûre de verify_transcript qui refuse explicitement «capacité non implémentée». Aucun lancement/OS ou appelant n’y est câblé.

Pour voir le nominal échouer comme comportement, le témoin nominal peut convertir SEULEMENT cette Refusal fonctionnelle en self.fail (en gardant toutes ses assertions positives). ImportError, erreurs de fixture et exceptions non contractuelles restent ERROR : jamais comptées rouges. Publier cette base et l’adaptation avant toute exécution ; pin du vrai module/import, RO puis GO du seul nominal distinct. Ce sera le rouge d’une APIpure nouvelle qui refuse encore, pas un faux défaut d’un observateur déjà installé.

Après ce rouge réel et conservé, implantation pure du contrat/conservation premièrecause/typesstricts/bornes/attestationCP séparée/fermetureCommon réelle permise en préparation. Publier et relire avant GO47verts distinct ; aucun kernel, fichier, clock, réseau, processus, signal ou publication réelle dans verify_transcript. Le backend, ses adaptateurs CP/Common, preuveshôte/profilstockage/handoff/SDK et les quatreappelants restent NONLIVRÉS et sans GO. Les bool/int ne doivent pas se confondre dans les champs typés. Aucun47vertsanticipé ni compétencephysique conférée par des faitsmodélisés.

## GO distinct du seul nominal de la baseline pure

**GO C7-ENTRY-PURE-BASELINE-2B04-17nov60** : source2b0402076daf726f8a541cc3abe5c0c816a88600, trois revues exactes Mémoire/Recherche/TTY favorables bornées, lecture C7 et réempreinte physique des quatre pins conformes. Module bb1afdb37652a6f1cce3a798388f8eba2c1892016065e040fe82bca43502ef2c, test3a7fceed6e631d36b6920f8503d4c7761a219225c676ecf1a3ca9741c58f7212, stub902e70d3990ad23118c07be42333a08790bb3049f5426832002356da44c772f4, /usr/bin/python3.14 c81d88523721c462f53a98129d65bba68f67791932f0664372e1778e74fd9517. Pins fichier baseline-nominal-pins.json de ce commit, cwd/import exacts sans homonyme et WT propre à revérifier avant entrée.

UNE invocation de la commande épinglée : PYTHONPATH limité au scripts/ de ce WT, timeout TERM58s/KILL2s, python3.14 -B -m unittest -v -f test_execution_entry.EntryContract.test_confirmed_entry_result_belongs_to_invocation depuis son cwd déclaré. Attendu un seul test,1FAIL sur «pure entry validation capability is not implemented»,0ERROR/skip,rc1. Ce sera un refus fonctionnel réel observé de la baseline, pas une preuve physique. Tout autre résultat, pin/import/montage inattendu ou dépassement : STOP sans retry ni correction improvisée.

Les RAW complets stdout/stderr, source/pins, commande, rc et temps réels vont dans une racine de preuve exclusive hors WT ; les préserver et réempreinter sur les deux hôtes AVANT toute implémentation pure. Les46autres fonctions restent non jouées. Les outils de lancement/archivage ne donnent aucun fait OS du protocole simulé. Aucun Common natif, SSH par le témoin, processus/signal/proc/net/DB/SDK/provider/Cargo/corpus/target/service/master/nettoyage autorisé. Après le rouge attendu conservé, seule la préparation pure déjà décrite ci-dessus peut avancer ; nouveaux source/pins/RO puis GO47 distinct avant tout autre test.

GO consommé : preuve f74e7f2cb580567d0ce22c20d8ea1c9d8748e52c,1FAIL attendu/0ERRORskip/rc1 en0,048796813s. C7 rehash personnel9/9 LOCAL+PC sans extra/manifeste d977e605e15ad599ccf9f1869e06305c171e013f85d615bc7ae60bceb94ebadb identique, trace du module/refus/self.fail et sources pins relues ; Mémoire rehash indépendant2hôtes favorable. La condition rouge/conservation avant préparation pure47 est satisfaite. Aucun GO47 ni capacité physique acquis ; publier nouveau source/pins puis RO avant futurs tests, sans modifier les archives rouges.

## Réserve statique C7 sur les identités initiales, avant GO47

Implantation pure cb2906f48c01e7cce39907c7aaf4547e546f492f relue, aucun test exécuté. La branche spawned exclut runner_pid mais pas les identités Common admises initialement. Contre-exemple par lecture : conserver plan(common_owner90/9,common_root95/10,runner100/11), remplacer dans tous les événements de l'enfant pid/start par90/9, garder ppid/parent_pid100 et les faits de clôture d'origine. Les contrôles écrits n'interdisent pas cette identité ; le même processus devient enfant naturel du runner et son propre ancêtre Common déclaré extérieur à la portée. Remplacer par95/10 produit également deux parentages incompatibles pour la racine Common. Pas de verdict d'exécution prêté à ces mutants, mais une incohérence sémantique du transcript doit être refusée avant de revendiquer le contrat pur.

HOLD GO47. Grouper avec les autres revues en cours ; préciser les alias permis (common_root peut être runner si PID ET start concordent) et interdire enfant=owner/racine ainsi que même PID initial avec starts contradictoires. Prépublier les témoins causaux sur le module cb290 intact avant correction, pins/RO puis GO ciblé distinct si confirmé. Pas de correctif ni d'exécution de ces nouveaux mutants par ce constat. Cette réserve concerne le modèle pur, sans nouvel accès OS ou accusation du backend inexistant.

## GO des trois contre-exemples de topologie

**GO C7-ENTRY-TOPOLOGY-RED-BD0D-17nov60** : témoins prépubliés80c68e53de31976e5d846e9d8130b779b8a0d915, pins/source bd0d71dde978f44f57cbf5615c2b3440746d86c7. Module8b6de03703a1b2a2de8f30c90a93a01d247d0b3be436e57215b1603981cb0697 INCHANGÉ de cb290, test35b93db8d3c4198a59f9316a6fd9f9c6e0ee53f7334e0dbb64a0048060e52627, stub902e70d3990ad23118c07be42333a08790bb3049f5426832002356da44c772f4, Python3.14c81d88523721c462f53a98129d65bba68f67791932f0664372e1778e74fd9517. Deux revues ciblées Mémoire/Recherche favorables aux témoins/pins ; C7 lit les quatre ajouts et réempreinte les quatre pins physiques, WT propre.

UNE invocation de topology-red-pins.json : timeout TERM58/KILL2, Python3.14-B/unittest-v SANS failfast, trois noms exacts test_child_cannot_be_the_common_owner, test_child_cannot_be_the_common_root, test_common_root_and_runner_same_pid_need_same_start. cwd/PYTHONPATH/import identiques au paquet relu ; attendu3FAIL «Refusal not raised»/0ERROR/skip/rc1. Le cas positif supplémentaire et les47précédents (48au total) restent non sélectionnés. Tout autre motif/compte/import/pin/temps refuse sans retry. RAW/source/argv/rc/temps/manifeste en racine exclusive hors WT, copies/rehash deux hôtes avant correction. Aucun fait physique/OS/backend/SDK/appelant acquis. Après ces rouges exacts conservés, préparation du seul correctif pur de topologie autorisée ; publication/pins/revues puis GO51 distinct avant tout vert. GO47 demeure retiré/non donné, aucune exécution automatique enchaînée.

## GO51 du modèle pur corrigé

Les trois rouges sont acquis en2a715fd7b971a009fcf666c27ac50c5d87f344cf, preuves M/R/TTY favorables dans leurs portées ; Mémoire réempreinte personnellement les deux hôtes, R/TTY local seulement. Source corrigée68eab84d95c8346c6dfe51ec4f3a7f0d79a093c3 : seules deux gardes ajoutées, enfant distinct runner/owner/racine et cohérence start si racine=runner. Le cas exact racine=runner reste permis. C7 relit le delta complet et les pins, réempreinte module/test/stub/interpréteur et WT propre ; deux RO totales R/M favorables source/pins, réserve topologie levée statiquement. Aucun vert51 encore déduit de ces lectures.

**GO C7-ENTRY-PURE-GREEN-68EA-17nov60** : UNE invocation de pure-51-pins.json du commit68eab, depuis son cwd avec son PYTHONPATHscripts exact. Module565b7f39494911b65ef121f19565450c70803c93b9ef6e62599d570707dbf218, test35b93db8d3c4198a59f9316a6fd9f9c6e0ee53f7334e0dbb64a0048060e52627, stub902e70d3990ad23118c07be42333a08790bb3049f5426832002356da44c772f4, Python3.14c81d88523721c462f53a98129d65bba68f67791932f0664372e1778e74fd9517. Commande timeout TERM58/KILL2 python3.14-B-munittest-v-f test_execution_entry ; attendu51PASS/0FAIL/ERROR/skip/rc0, noms exacts de la liste scellée. Premier rouge/inattendu/pin/import/temps : STOP sans retry, ticket avant correction. RAW/source/pins/argv/rc/temps et manifeste hors WT, rehash deux hôtes, anciens rouges intacts. La borne60 concerne cette seule exécution CPU ; aucun OS réel du protocole ni SDK/CP/Common/appelant/target/native/provider/service/master/nettoyage n'est autorisé ou qualifié. Source/pins modifiés invalident ce coupon, pas de GO47 réactivé.

## Suffisance pure et protocole concret à préparer

GO51 consommé : preuves892bc4271fec9b387142954358c4110a58ff2bdd,51PASS/0ERROR/skip/rc0,0,079794829s. C7 lit les résultats/rapport53 et réempreinte personnellement9/9 LOCAL+PC, sans extra, manifeste003ca193ffc17c0d54eb6366aee549ce54b5a07f5e5ddc5d7d9fb1d015a671b3. Trois RO preuves favorables M/R/TTY, Mémoire deux hôtes/R et TTY local. Les51noms sont exactement ceux des pins, pas55verts : trois commandes totalisent55invocations dont la dernière seule est verte. Suffisance C7 acquise pour ce validateur PUR et ces51témoins ; rouge nominalf74 et topologie2a restent conservés.

Prochaine préparation autorisée, sans exécution : **protocole opérationnel concret/API minimale et recette d'un petit producteur d'entrée** avant implantation OS. Source/rôles/ordre temporel à publier : plan attendu admis indépendamment ; entrée EXEC réellement confirmée avant corps, objet/fichier et stabilité du profil local déclarés ; copie/empreintes puis publication/ACK du vrai CP avant libération ; parent naturel et wait, RAW et fermeture Common fournie par son propriétaire extérieur distinct. Décrire le mécanisme exact et les refus, domaines d'horloge, bornes incluant kill/reap/publication, disparition et premièrecause, modes fork/Cargo/shell. Les séquences exec/main/detach déjà qualifiées servent de sources, jamais d'anciens GO. Si GDB est retenu, distinguer Popen/debugger/inferior/parent naturel et détacher avant tests, sans prendre rcGDB pour rcprogramme.

CP reste le seul propriétaire publication/ACK/Window ; pas de boucle parallèle. Ses gardes encore rouges interdisent de présumer son adaptateur qualifié. Un éventuel éditeur de fixture doit être nommé FIXTURE_ONLY, sans publier une admission CP/global. Aucun appelant/Cargo runner modifié avant protocole/surface relus, aucun backend OS codé/exécuté par cette préparation. Prépublier témoins pertinents et recette native minimale/pins/coûts proposés ; RO puis arbitrage source/GO distinct avant prototypage. Ne pas coder un traceur exhaustif : image d'entrée/invocation seulement, images futures UNKNOWN et SDK séparé. Aucun service/modèle/DB/target partagé, privilège ou sandbox modifié.


Protocole d81 puis69db en revue après réserves regroupées CP/R/M/TTY : clock/origine avantwatchdog/PIDstart identité seule, payloadsCP explicites/ACK, renameavecFDA positif vs entréeB négative, exclusion écriture causale mêmefixture privée, OwnedLaunch et cleanup préHeldEntry. Aucun backend/helpers exécutables. C7 attribue Archi comme porteur de la préparation et de la future implantation du nouveau producteur après RO du protocole ; Recherche revoit et garde les raccords e2e futurs, pas une prise en charge implicite du backend. Source/pins/recette/contre-exemples avant code/exécution selon leur étage ; aucun privilège ou sandbox modifié, pas de publication parallèleCP ni traceur exhaustif.


## Implantation SOURCE du petit producteur en mode fixture

Le protocole corrigé69db61c102a0987fad43b86fa3082e03cca071be reçoit quatre RO exactes DOC/API M/R/TTY/CP favorables dans leurs portées. C7 lit README et API ; les réserves d81 sont traitées au contrat. Vigilance TTY maintenue : soft_end/kill_end issus de START avant Popen sont des cutoffs logiques conservateurs, pas une preuve que le timeout démarré ensuite a déjà tué/récolté à START+125. La future recette doit observer durée/overrun/résidus et refuser tout positif tardif ; aucune garantie physique universelle inventée.

Archi est autorisé à préparer le CODE SOURCE du module neuf execution_entry_runner.py et de ses contrôleurs/fixtures, mode FIXTURE_ONLY exclusivement pour cette première implantation. Prépublier les témoins exécutables pertinents avant les correctifs et respecter rouge comportemental avant fix, jamais import ERROR comme rouge. Sources/rôles/pins/recette exacte seront relus avant tout import/test/OS/native ; aucun coupon d'exécution fourni ici. Le module pur565b et ses preuves restent gelés. Les capacités opaques ne naissent pas d'un bool/PID libre ; OwnedLaunch précède fork, toutes exceptions préHeldEntry conservent premièrecause/cleanup attribué et END original.

LIVE/adaptateurCP/finCommon/publicationréelle restent explicitement unsupported tant que leurs ports ne sont pas qualifiés ; pas de fallback fixture pouvant produire un grant. Une sonde de profil doit rester une qualification expérimentale privée, sans se déclarer déjà profil connu ; ELF/FD/rename et ETXTBSY causal à prouver selon N04/N06/N19. Pas de traceur exhaustif, de privilège ou sandbox modifié, d'appelant/Cargo/canon existant changé, de target/service/DB/provider. Les quatre sites e2e et Cargo runner demeurent sous réservation/revue Recherche pour une phase ultérieure. Origines et durées entre hôtes restent séparées, stdin/argv/env publics stricts, jetonCommon jamais journalisé.
