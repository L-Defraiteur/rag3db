# Passation ED — 13 novembre 2026

## Arrêt demandé et état de reprise

Passation demandée par Lucie via C7 pour limite d'usage. Aucun nouveau GO, calcul, compilation, test ou nettoyage. Les anciens délais monotones sont historiques : **ils ne se réactivent pas à la reprise**. Toute exécution future exige une nouvelle décision C7, un couple exact et un créneau ; aucune autorisation de master ou release n'est implicite.

ED est en veille de relecture. Aucun lot produit ED actif, aucun processus de qualification ED ni poste/verrou/service détenu à ma connaissance. Les derniers travaux sont des relectures de sources et de preuves appartenant à Arbre, Recherche et Lucivy. Ne pas exécuter leurs pilotes à leur place.

## Seul worktree ED encore présent

- Portable : `/home/lucied/.cache/rag3weaver-build/cx-wt-ed-ci-writer-diagnostic`, branche `cx-ed-ci-writer-diagnostic-r1`, remote `origin/cx-ed-ci-writer-diagnostic-r1`.
- Avant ajout du présent rapport : HEAD propre et publié `8714fa2635fe2473eec121ca67533f733dc5d2bc`, base documentaire `549ab1e3d2cf05d5ba8e9cb9dc07eda4d10a03bc`. Le présent commit est documentaire seulement ; son SHA est celui de la branche publiée contenant ce fichier.
- Document de diagnostic : `10-diagnostic-ci-writer-physique.md`, même dossier. Aucun Rust ni pilote modifié par ED dans ce lot, aucun target créé.
- luciepc : la lecture de `git worktree list --porcelain` ne trouve aucun worktree ED de diagnostic ou read_scope. Aucun jumeau de ce worktree n'y a été créé. Ne pas confondre avec les worktrees des propriétaires des autres lots.
- Aucun WIP ED produit non poussé. Ce rapport est poussé sur la branche ci-dessus, jamais sur master.

Arbre original de Lucie `/home/lucied/git_workspaces/rag3db` : préserver strictement `M extension/rag3weaver/codeparsers` et `?? extension/rag3weaver/docs/lucie-tmp/`. Ne pas y checkout/éditer/committer. Origin/master observé à la passation : `3a5080d93bdaeba05fd333b25bdd0565b49ad18b`, sans qualification de ce nouvel état par ED.

## Livré ED et ressources déjà retirées

### read_scope

Produit livré master `4e2e2bfbb7c8ce670a49a06a36580d481d9abbc0`, bilan documentaire `940ad65bf57461b714ff2671266bf84cb74f6966`. Ticket fermé : `docs/tickets/closed/2026-11-13-read-scope-masque-les-erreurs-de-lecture-des-worktrees.md`. Le changement remonte les erreurs backend/usages au lieu de prétendre absence ou unicité après lecture incomplète.

Source qualifiée R4 `461e8b868f40907bb0b2f731ff2fcaebc78923d2`, preuves `07f346ec5566d764c7862e01f3e87c0e071e9c9e`. F8 8/8 ; cinq exacts 1/0/0 ; module 20/0/3 NAV ignorés attendus, inventaire23. Pas de globale physique déduite. ELF `033c31d00bb7d63b1034218b5b3bbfec257ad9f9f7bed94621dd77114800375a`. Manifest306 fichiers `0d33d59f9caf97446700557b3d7f546230abd28c32f4bea68a60cddb7d4b835b`.

Rouges R2 admission avant compilation et R3 fixture privée sans modèle conservés ; seul le témoin R4 corrigé a été qualifié. Tous les anciens worktrees ED read_scope, branches locales correspondantes et deux targets PC ont été retirés sous gardes autorisées. **Ne pas rouvrir ce lot.** Remotes historiques conservés : `cx-ed-read-scope-erreurs`, `cx-ed-read-scope-qualification-r2`, `cx-ed-read-scope-integration-r3`, `cx-ed-read-scope-fin`, `cx-ed-ticket-read-scope`.

Preuves conservées sous `~/.cache/rag3weaver-build/` : PC `cx-preuves-ed-read-scope-r2`, `-r3`, `-r4` ; portable `cx-preuves-ed-read-scope-r2-rouge-local`, `-r3-rouge-local`, `-r4-local`. Retraits et archives d'admins fidèles deux postes dans `cx-read-scope-nettoyage/{portable,pc}`, notamment `fin-ressources.json`. Ancien guard de retrait qualifié SHA `6f3dfceb17c9703029d2f4f26385f9aa21820c84bc79c518e1338b6d0f36188c` ; il ne dispense jamais d'une nouvelle garde ni d'une autorisation actuelle.

### Frontières boîte

Ancien lot livré `b876881887d174f9ce4e889d64aebf671a8a4f61`, bilan `f8cbd6a48abac5ad563883689132d0f45621916f`. Ressources retirées auparavant ; aucune reprise nécessaire. Remotes `cx-ed-frontieres-boite*` conservés.

## CI writer et Lucivy : en cours, porteur Lucivy/C7

Ticket commun OUVERT : `docs/orchestration/tickets/2026-11-13-la-ci-refuse-la-reouverture-du-temoin-physique.md`.
CI D17 `d17a230b14f60fbac59aa1e4ac10510ba5f3c7e2`, run43407787602/job131826283645 : un échec `native_legacy_foreign_schema_and_values_refuse_owned_open_without_writes`, réouverture writer après Drop. CI liée au moteur statique du checkout, pas au SDK29. **Le détenteur exact du writer historique n'est pas attribué. Aucun rerun CI effectué par ED.**

Diagnostic ED publié8714 : l'acquittement Shutdown en lucivy/luciole4.3.3 précède la destruction de l'acteur ; sa chaîne shard/Index/BlobDirectory/store peut conserver la connexion/Database. Le permis physique valide chemin exact et origine ; ne pas le remplacer par le polling Weak/REOPEN_WAIT ordinaire, qui n'est pas une barrière de destruction.

Dans le dépôt Lucivy, branche `cx-lucivy-shutdown-destruction` : production FBD `fbd3f0a4179917e3674fa52a56ffde027831a354` ferme l'admission, attend les sends admis et draine les messages avant completion. Garde handler RAII worker/coopératif/pinned ; wrapper avant on_start ; panique de Drop capturée puis reprise après nettoyage. Anciennes réserves07086b6 (file flume non vidée, self-join pinned),4c41543 (unwind) corrigées. Dix exacts CPU qualifiés, preuves rag3db `5cba3779263bfc2a92fdab17e65e1e7f231645cc`.

Source avec tests supplémentaires `3b1661446c573ee794b918ec016c0cbff748e4ce`, production FBD inchangée. Passe propriétaire sous900s/8G/HIGHmax/j4 : lib luciole179/0/0 + six exacts core1/0/0 chacun ; preuves rag3db `613d9362995740bb50f45cccb5861c9e1b3fc85e`, `docs/lucivy/13-11-2026/preuves/plein-3b16614/`. ED a vérifié32 hashes et pilote9b2be2fc exact. **Rectification conservée : trois lignes de noms sont entrelacées sous nocapture, donc pas de preuve stricte “0 nom absent” par extraction de lignes ; résumé179/0/0 reste acquis.** Pas lib core entière, SDK ou CI physique qualifiés.

Mesure A663/BFBD publiée `a568adb29`, `03-mesure-cout-envois.md` et `preuves/mesure-envois-fffe/` : ED a vérifié210 hashes,96 cas et recalculé les huit ratios. Harnais et locks A/B identiques ; affinité20–23,8G,600s, ABBA. Débit baisse sept cas sur huit ; worker −17,5 à −40,1 %, un cas quatre producteurs/cap1024 +80 %. Pinned −9,3 à −20,9 %, biais de diagnostic stderr vers devnull explicitement conservé. Delta global, pas coût du seul mutex ni débit d'ingestion. C7 a mis la fusion FBD en HOLD à cause de ce coût ; aucun seuil inventé, aucune release.

Alternative C publiée, **NON qualifiée dans le dernier état relu ED** : Lucivy branche neuve `cx-lucivy-barriere-admission-atomique`, SHA `fa1d0273c468b997163f3cf9d5f56bae052f522a`. RO mécanisme favorable : bit closing+compteur dans le même AtomicUsize/CAS AcqRel, garde empruntée sans cloneArc, fin des sends avec mutex/notify seulement si closing ; réveil à CHAQUE fin requis quand capacité < nombre d'admis. Test compteur et3admis/cap1 pertinents. Qualification correctness C puis mesure A/C restent à autoriser/relire sur sources exactes ; ne pas transférer les verts FBD ni rejouer B identique.

Dépôt Lucivy local observé via `/home/lucied/git_workspaces/cx-lucivy-shutdown-destruction` pour lecture seulement. Ses autres W/T et homologues PC appartiennent à Lucivy : consulter sa passation, ne rien retirer. Rapports/pilotes/preuves rag3db dans `origin/cx-lucivy-shutdown-preuves-4`, dernier HEAD observé à la passation3a5080d93bdaeba05fd333b25bdd0565b49ad18b ; les nouvelles entrées après les SHA relus ci-dessus ne sont pas déclarées relues.

## Garde target Recherche : livré par son propriétaire, ne pas relancer

Source qualifiée `9ec421069320b2f0cca4099fa27f83059e944de9`, pilote `65ad0810ca3f4f1496539d5e2945e5c959b94a07`, preuves `cdcc032dfde186f9b33fb8d9804ca50fcf2c1a4b` : F8 8/8, lib2308/0/66 incluant le juge CI en succès SDK29, NAV40/0/0, deux corps HTTP issus d'UN juge,223 gabarits/ELF. ED réhash558 fichiers manifest `cece3588719044bf461eb6f7f322eb8f9b0db524e2b63e979eb07f782522cb3d`, recalcul223 blobs Git et présence octets dans chacun des deux ELF. Local `cx-preuves-garde-target-union-v2-final/{client,pilote,superviseur}` sous cache commun. Verts SDK29 ne réparent pas CI statique.

Candidat relu `8eefcd65687d8ae999620ed745385aa705432cdb` :6783 entrées horsdocs strictement9ec,8008 entrées master protégées, seul ticket garde fermé. Landing propriétaire ensuite présent dans base9feb d'Arbre. Ne pas faire de push/retrait/rejeu à partir de cet ancien avis ; état courant et ressources à lire chez Recherche.

## 2b-A Arbre : dernier avis, pas de nouvelle exécution ED

Ticket OUVERT : `docs/arbre-principal/tickets/2026-11-13-appliquer-la-visibilite-des-couches-avant-les-requetes-des-vues.md`. A = admission CPU/fixtures et raccords communs ; B chargeur physique durable/liaisons qualifiées/snapshot composé et C outils accessibles restent absents et ouverts.

Source A initiale `affa6d03b5a57dd99810b6853783dc1a267f76b9`, pilote `c75ad3ae4b437495fb4544045894a986ac23bebb`, preuves `33a64da41` :15 exacts puis libCPU1662/0/6 ; publication25 incluse ;1662 distincts/1677 invocations. ED réhash315 fichiers manifest `59bf3e14da0adfc99f1788ff431a6ae4d707602e32f860bb51b654019682d6ba`, ledger `22e3ccb9af4c9711db3491d545486a41b231bff0cbb96ae23b78dfda909b1be2`, profil main et deux états frères. Copie `cx-preuves-couches-2b-a-v1-local`. Cette preuve ne qualifie pas native/B/C.

Union193 `193d77cd71fa4cc1608e961ea4c8a8d3d445412e` puis rouge F8 dead_code horscfgtest (preuves e6e6b8677,48 fichiers conservés). Source corrigée, figée, dernier état produit relu : **`d9aa4be86c6f21665cf85c22e47134c87ab6aab8`**, branche/WT `cx-arbre-couches-2b-union-cfg-r1` / `cx-wt-couches-2b-union-cfg`. Huit cfg(test) ciblés, corps/juges et politique runtime inchangés, RO favorable. Ancien pilote9625 retiré sans run après assertAPP193 résiduel ; corrigé d7d00dcc avec témoin AST.

Rouge corrective v2 publié `237614f8343a196f384d2b01361a8f83e7a0b490`, rapport39 : reprise193→d9 verte, F8 jeux1/2/3 verts puis observation Cargo jeu4 sept champsNone. Rawlen absent, rc jeu4 non acquis, cause exec/environ NON ATTRIBUÉE ; pas de nouvelle lecture pour innocenter après rouge. Aucun ELF ni juge union qualifié. ED réhash59 fichiers manifest `fd4e8bb02f79559458341a178e4c7ae740bc992d24a7e95abf773465cc572aab`, copie `cx-preuves-couches-2b-union-v2-local`. T porte désormais le WTcfgd9 ; pas193 ni affa.

Dernier couple relu, favorable statique seulement : **PILOTE `4fca0dd38378d5bb31269dab76713d6bc0060ebc` / APPd9aa**. Branche/WT `cx-arbre-couches-2b-union-observe-pilote-r1` / `cx-wt-couches-2b-union-observe-pilote`, scripts `couches-2b-union-observe/`, rapport40.

- Capture Cargo possédé par pidfd, identité/CG/UID avantSTOP, refus déjàT/tracé, tous threadsT stables, snapshot sélectionné immutable avant assertion puisCONTfinally mêmefd. Pas de lecture profil au cleanup.
- Réserve776 corrigée : refus module converti RuntimeError NON OSError avec cause ; les handlers de polling ne peuvent plus absorber PermissionError/disparition. Juge AST vraiobserve/run couvre ces trois erreurs et aucun Popen suivant.
- Présence exigée selon profil de phase explicite : SDK sept champs présents ; metadata/CPU six absents et LD rustup présent. Valeurs, rawlen et diff stricts, pas d'exemption None arbitraire.
- Cache SAMEWTd9 : garde commune silencieuse, fingerprints identiques attendus, provenance/reçu rouge terminé, ancien ELF affa uniquement comme preuve historique, scans frais postposte/rôles stricts. Pas de réinvalidation prétendue.
- Séquence proposée F8/3ELF/ledger préalable/liblarge détachée+vraiwait/CPU15+publication25/NAV40/223Git/final. **NON JOUÉE par ED** ;24 fonctions hermétiques rapportées par propriétaire, pas de signal/PID/Cargo réel lors de ces fixtures.

Ancienne fenêtre Arbre INITIAL918138.385510088 / END922938.385510088 : historique, non réactivable ce week-end. Dernier message propriétaire dit aucune Auth/PQSv3 consommée ; cela n'est pas une observation fraîche de fin de passe. Lire sa passation et C7 avant toute action, sans déduire ici qu'une exécution ultérieure a eu lieu ou non. Les deux postes W/T/PQS appartiennent à Arbre ; aucune suppression autorisée.

## Contraintes et trois prochains pas

1. Lire la passation C7 et celle des propriétaires ; relever les HEAD/remotes et l'état réel des passes arrêtées. Commandes RO : `git fetch origin`, `git status --short`, `git worktree list --porcelain`, `git log -5 --oneline origin/cx-lucivy-shutdown-preuves-4`. Ne pas toucher l'arbre sale de Lucie. Relire ce rapport avec les tickets ; aucun délai/GO historique ne repart.
2. Reprendre les RO sur les nouvelles preuves ou sources exactes : priorité sûreté observer Arbre si delta nouveau, puis correctness C Lucivy et coût A/C si autorisés séparément. Commandes RO : `git show 4fca0dd38378d5bb31269dab76713d6bc0060ebc:docs/arbre-principal/13-11-2026/couches-2b-union-observe/cargo-stop.py` ; dans le dépôt Lucivy `git show fa1d0273c468b997163f3cf9d5f56bae052f522a:luciole/src/mailbox.rs`. Aucun Cargo/rustc/runtime de diagnostic automatique.
3. Transmettre les avis exacts à C7 et aux propriétaires, puis attendre son arbitrage courant avant intégration ou calcul. Ne pas fermer le ticket CI sur une preuve SDK/CPU ni livrer2b-B/C par extrapolation. Conserver tous les rouges/W/T/preuves ; aucun push master ni retrait sans décision actuelle explicite.

Coordination : C7 `01a1cb5c-62b9-7ea0-8b7e-cf38398ece09`, Arbre `01a1cb5d-2191-7112-82be-648e153fe4cd`, Recherche `01a1cb5c-d8c7-7182-9712-a78a9c43d24d`, Lucivy `01a1cb5e-7665-7a23-9980-ac3f98b0d97b`. Session ED `01a1cb5d-09c6-70f1-831a-f0c62627359a`.

SDK29 historique épinglé : `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`, SO SHA `b014febe51417ca535218d1c0b18308e7d301157abf45027a5cbba70dfc54929`. Ne pas rebâtir/patcher le SDK ni le registre de dépendances. Aucun secret .env/.vault à lire ou publier, aucun signal à un PID non attribué, aucun gc/prune/repair/deinit. Tout calcul autorisé à l'avenir passe par vrai poste, confineur/bus/reçu authentiques, mémoire et budget exacts, premier rouge STOP sans retry aveugle. Pour cette passation : **zéro nouveau calcul**.
