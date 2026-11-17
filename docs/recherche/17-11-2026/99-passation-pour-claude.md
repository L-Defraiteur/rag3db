# Passation Recherche — arrêt prioritaire du 17 novembre 2026

Le 17 novembre PRIME les historiques du 13/16. STOP Lucie relayé par C7 :
aucun calcul, test, GO, health, provider, master, tag ou nettoyage nouveau.
Tous coupons non consommés révoqués ; aucun ancien END réactivable. Les commandes
ci-dessous sont des lectures futures INACTIVES, pas une autorisation de reprise.
Ne pas reprendre sur un rappel automatique. Claude doit lire la synthèse C7 et
obtenir les nouvelles autorisations nécessaires avant toute exécution.

## Sauvegarde et ressources

Branche de cette passation : cx-recherche-passation-claude-20261117-r1.
WT local : /home/lucied/.cache/rag3weaver-build/cx-wt-recherche-passation-claude-20261117-r1.
Base master locale historique 927941a67a5805450d49f1e04b39fdd542a6059b,
sans atterrissage ni modification de l'arbre de Lucie. Le SHA final de la
passation est donné par C7 et par HEAD de cette branche, à vérifier contre origin.
Identité Git Lucie Defraiteur / luciedefraiteur@gmail.com.

[Inventaire local](99-inventaire-git-local.json) : 32 WT Recherche, tous sans diff
au relevé AVANT création des présents fichiers. Les 31 anciens HEAD sont publiés
(égaux ou ancêtres de leur branche origin contrôlée par ls-remote).
[Inventaire PC](99-inventaire-git-pc.json) : 19 WT, tous sans diff ; plusieurs
HEAD diffèrent de l'origine publiée. Ce sont des copies historiques : ne pas FF,
rebaser, supprimer ou nettoyer. Une inégalité n'établit pas un WIP non poussé ;
l'ascendance des objets PC absents localement n'a pas été certifiée.
Aucun WIP produit local créé dans ces revues ; cette passation docs est le seul
nouveau lot à pousser. Les inventaires donnent HEAD complets, branches, origin,
chemins et status ; ils ne prétendent pas un inventaire des autres sessions.

[Relevé ressources](99-ressources-metadonnees.json) : noms/lstat de premier niveau
uniquement sur les deux postes ; aucun parcours récursif, du, suppression ou
qualification de target. Tous W/T, logs, anciens rouges et copies sont préservés.
Aucune exécution détenue par Recherche n'était active à réception du STOP :
aucun PID/scope/lock à arrêter par cette session. Aucun poste lourd réservé,
aucun service/provider/PG ouvert ou modifié par Recherche dans ces revues.
Ce constat de propriété n'est PAS un scan global de quiescence ; l'état des
services d'autrui n'a pas été interrogé. Aucun nettoyage autorisé.

## Périmètre et état livré

Recherche a effectué des RO source et preuves, pas les essais des propriétaires.
Lectures Git, métadonnées et hash/JSON/raw par scripts stdlib propres seulement :
aucun import/parse/test du candidat, Cargo/rustc/ELF/GDB, /proc actif, signal,
DB, FTS, SQL, health ou provider exécuté pour ces avis. Les réplications PC des
preuves restent des claims owner sauf vérification PC explicitement citée.

Les SHA complets des références ci-dessous sont dans [99-references.json](99-references.json).
Les objets d'un autre dépôt (Lucivy) y sont identifiés comme tels ; ne pas chercher
leur source Rust dans rag3db. Aucune fermeture de ticket par cette passation.

### Arbre : lecteur B, réserves et contre-exemples en attente

Source 7a78310d6d (runtime 4e3fc8402e), rapport Arbre 17nov/15,
paquet Q1d1441034dc2f811239bfd9688a63ebc3244cf65173890080b04171205f5737d :
RO R RESERVÉE. R1 : SHA courant du Reader utilisé comme attendu au lieu du pin
indépendant add010. R2 : deadline vérifiée avant hash/open mais pas juste avant
exec ; calcul deux now et pré-Popen à borner. Ancien paquet RETIRÉ du GO.

Dernière demande 3240c0a469 : cinq témoins CPU publiés AVANT FIX dans
runtime-99/test_reader_boundaries.py + red-boundary-pins.json : promotion SHA,
END pendant hash, END pendant open, deux now/END-15, END avant Popen. Vrais
first_query/reader_exec inchangés ; proc/clock/Popen/exec fictifs. RO de cette
source NON TERMINÉE par R ; aucun de ces cinq rouges acquis ici. GO révoqué.
Les 12 autres fonctions non sélectionnées ; aucun B natif qualifié par A.

A CLI99 : preuves 90aa9d053d, APP99fef92ce6, RO favorable bornée BUILD/STATIC :
34 fichiers locaux /117387802 octets, manifeste fab645b8020848f4a3119fb45ebddfca59189a1a74f83f8c060901c20515d0ee.
Deux bins backend687de/checkerc587 ; Reader add010 auto-test typed seulement,
aucune DB/provider ni attestation maps Reader. Suffisance A C7 9a771 historique.
Q9/E9 preuves 6e97b2ff8a : cycle writer historique, pas preuve maps Reader.
B9 callback b9e5 historique : réserve communicate sans borne déjà supersédée
par transport E9 edf45f73 ; C7 a demandé de ne plus y consacrer du travail.

### Lucivy : correctif groupé FS encore à relire par R

Dépôt /home/lucied/git_workspaces/lucivy. Source exacte
13298f7db3443956ea01e088b0c014fd8c60c2ff, branche cx-lucivy-fs-output-fix-17nov,
parent bdf83ad ; docs rag3db2107c70a13, rapport Lucivy29/source-pins.json.
Le gros diff demandé par R a été tronqué : ne PAS prétendre sa lecture totale.
Banc a fourni RO favorable PREPA c7e2e5d381, pas une RO indépendante R.
À vérifier : InvocationPaths root/reçu/reports uniques/hors input/feuilles absentes,
slot create_new sans mkdir AVANT builder, None→Missing/bounds, flags invalides
avant toute metadata/output selon C7 30561. Limite namespace observé explicite,
pas ancrage FD ancêtres/garantie hostile rename. 16 témoins verts NON OBSERVÉS.

Rouges 5094f2b29e déjà relus R : 57 entrées PC+6 portable rehash Git ; 18 copies
LOCAL/17108104 octets, cinq FAIL101 causaux+un PASS0, pas retry. collision
reçu/rapport b'index-report documents=1\n', input/alias ADMITTED, None→Ok0,
flags ancien REFUSED/Manifest, positif intact. Common186154/start128760537
TERMINE/VERT0 nettoyé, 11UNKNOWN non attribués, pas absence GPU universelle.
Archive locale R/cx-lucivy-fs-output-red-bdf83ad-17nov-v1 ; les originaux/anciens
5ELF avant compile ont leurs copies conservées. Ne pas confondre rouges
sémantiques acquis et gate FS vert. Nouveau paquet GREEN/RO/GO distinct requis.
Ticket docs/lucivy/tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md.

### Archi : modèle runner, réserve rc acquise source mais rouge encore attendu

Ticket docs/orchestration/tickets/2026-11-17-les-images-executees-manquent-d-un-producteur-de-preuves-commun.md.
Source 5877852b4f : RO totale R favorable PREPA MODELED ; copies privées des
observations avant callback/clock, quatre handles opaques conservés, request
profonde et END posthash maintenus. Runner cf3e7a41fc30eabd0ef5ff1b377efbf102ac14e3443d9796e3b3f19099cb4a0e.
30 tests fc300 byte inchangés, pur565b gelé ; quatre pins conformes R.
Pas de qualification OS/CP/CommonCLI/LIVE ni borne interruptible des callbacks.

Dernière source 2df73d1dc5, rapport66/runner-wait-rc-red-pins.json : RO témoin
favorable PREPA1. RAW bytearray invalide fait refuser observation-types AVANT
mémorisation du rc7 ; attendu 1FAIL None!=7, NON OBSERVÉ ici. 31 fonctions,
une sélectionnée, 30 autres NONJOUÉES ; test d9d61264000941cb312bdc9f72bcd944bb364bc7b6c97cde888f10eb7880adf9.
La réserve rc reste ouverte : décoder/valider le statut avant copie RAW sans
callback/clock, sans accepter RAW invalide, seulement après rouge conservé.
Preuve b5113587e7 des deux précédents rouges observations reçue mais réhash
indépendant R NON FAIT : neuf bruts, manifeste c13bb055e8ceaa1507f466e5fdf9ee701e745874223d6e49eb4d8ccd8aa43e12.
Anciennes preuves purs51/model8 et4 restent historiques, pas un GO30/31.

### G : psql, 23 témoins sémantiques verts, native HOLD

Ticket commun docs/recherche/tickets/2026-11-16-la-garde-des-comptes-en-blobs-reste-a-302-documents-pour-301-lignes.md.
Source65447d0aaa (runner aec264c03df3c864d311ba006aca783e75f03a661db5b91c0aab491d074c0b2a),
RO totale R favorable PREPA23 : deux positifs complets+16mutants isolés+canari,
quatre anciens refusent preuve absente ; cinq corps carry byte inchangés.
PC personnellement réhashé 13checks=9sources+Common6439+3outils. Root1000/700,
recette50cd268d25ca2e054b3dddba312045c2fcfa4acccfc15dd9d63c3c1c2214da8e,
reçu e3418f6efc06789a09428f9b6e751837505d42eabd38fa54fde4e741d97226f7.

Preuves af6b1558f4 : RO R favorable bornée. LOCAL
~/.cache/optimiseur-notes/psql-host-runner-20261117-r1/native-contract-cpu-10-proof,
21/21 tailleSHA zéro écart /120006 octets, manifeste
 d02ae81efec60f7c948e7ff5ba0be6de1e6495c098759c015eed5a09977a21ee.
23 noms verbose uniques PASS=résultat/0ERRORskip/rc0 ; unittest0.004s,
entrée0.233421704s. Commona5rkgdd5/525810/start128961558 TERMINE/VERT0,
erreur null/nettoyé/restes[]. Bus absent déclaré owner, aucun scan R.
Copie PC preuve claim owner. Aucun init réel/capture enfant/PG : contrat pur
sur __new__/captures synthétiques seulement. Collector/handshake/outerwhole5
AVANT imports/terminal runtime externe NON LIVRÉS, contact/fixture HOLD.

19 Python fixtures réelles : preuve2758641c36 déjà RO R favorable, 56 fichiers
locaux manifeste a5b29479956b57ddb1ba44e6074fe111d480a41c5e87618550f24e68f1707cc5.
Sept captures Python, pas psql/PG. Holder EOF Z pendant test ; observation
ultérieure absence APRÈS END ne prouve pas absence avant END. Anciennes 8+4
non rejouées par23. Preuve quatre rouges c90dd112e0 reçue mais réhash R encore
NON FAIT (native-attestation-red-09-proof,20fichiers,2e668f9b5a2b66197e50dc0bb81586638d3b3ee3f4b2ce0acd3a79ae7d0b008c).

### Recherche propre : FTS conservé, aucun global réactivable

Inventaires des branches/WT joints font foi pour les lots existing-only,
maintenance double/gen, fixture-root et pilotes global r1..r5. Dernier r5
46078c4ff1 est une PRÉPARATION, pas un GO. Preuves de blobs149a108bcb bornées :
orphelin froid [9000000] physique, pas historique tracé/maintenance writer.
Tout global6000, tous budgets expirés, coupon SDK29/4fca sont GELÉS/RÉVOQUÉS.
Consulter les rapports de chaque HEAD (git show), pas le master ancien seul.
Tickets ouverts importants : maintenance-du-plein-texte-ne-peut-pas-ouvrir-un-index-existant,
lire-un-index-plein-texte-existant-sans-writer, les-juges-fts-exigent-une-racine-absente-de-la-ci,
l-etat-d-index-ne-voit-pas-les-orphelins-du-plein-texte, un-compte-de-lignes-indisponible-est-ramene-a-zero
(préfixes datés complets dans les arbres de branche, ne pas inventer une clôture).

## Trois prochains pas, inactifs jusqu'à reprise autorisée

1. Lire synthèse C7 du 17nov et ces inventaires ; vérifier HEAD/origin et arrêter
si nouveau code ou absence de pin. Commandes RO :
`git show cx-recherche-passation-claude-20261117-r1:docs/recherche/17-11-2026/99-passation-pour-claude.md`
et `git ls-remote --heads origin cx-recherche-passation-claude-20261117-r1`.
2. Sur reprise RO explicite seulement, terminer Reader3240c0a469 puis source
Lucivy13298f7 ; demander au propriétaire son HEAD exact courant avant lecture.
Commandes de consultation : `git show --stat 3240c0a469` et dans dépôt Lucivy
`git diff bdf83ad 13298f7 -- lucivy_core/tests/common/fs_fixture.rs`.
3. Sur reprise autorisée seulement, vérifier les rouges encore non relus
(b511/c90), puis nouveau correctif rc Ar chi/collector G selon propriétaire.
AUCUN essai au titre de cette liste : paquet exact/RO/GO C7 neuf/poste réel/
budget unique/preuves première cause conservée requis séparément. Ne jamais
réutiliser les anciennes commandes timeout/END/credentials/leases ; ne jamais
ouvrir original vivant, relancer service, nettoyer W/T ni pousser master.

Les avis RO ont été envoyés via transport intersessions aux propriétaires et C7 ;
cette passation remplace la mémoire conversationnelle pour la suite. STOP ensuite.
