# Trois juges verts config — préparation, aucun Cargo lancé

Ticket : `docs/lucivy/tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md`.
Source exacte890c86af44989f3fea6c6fa62fd40d688b4084df, RO Recherche et Mémoire
favorables statiquement ; ce n’est pas encore une preuve d’exécution du fix.

## Contrat de quiescence borné décidé par C7

Refus initial/proc1247 conservé, pas d’élévation ni permission contournée.
Le contrôle vise les identités PID/start détenues par l’ancienCommon et l’activité
attribuable au seul target/worktree. Pas une preuve d’absence universelle/proc.
Chaque relevé UID/start/CG/exe/cwd/FDaccessibles+erreurs est écrit AVANT assertion.
Les liensFD horsT/W ne sont pas publiés : compteaccessible, liensattribués et
erreurs suffisent au contrat. Identitéowned présente ou référenceT/W connue
bloque ; UNKNOWNattribué ouowned bloque aussi. Métadonnées opaques nonattribuées
restent UNKNOWN, avec indicateur d’identitébaseline ; elles ne deviennent jamais
« absentes » et ne suffisent pas seules à bloquer ce contrat borné. Aucune
exception de permissions ne permet import/purge/invalidation d’un cache tiers.

Trois témoins synthétiques publiés avant exécution : activitéownedconnuebloque,
activiténouvelleattribuéeopaque bloque (même identitébaseline), et opaquenonattribué
baseline conservéUNKNOWN. **3/0**, sansCargo ni scanproc dans ces simulations.
Le classifier testé est identique au paquet final ; source récente ajoute le
scanaprèsjuges et conserve marqueurcréateur deT intact.

## Réemploi unique, sources et empreintes

[Précontrôle et relevés partiels](preuves/indexed-warning-green-preflight-v4/preflight.json).
Q neuf `/home/lucied/.cache/rag3weaver-build/cx-lucivy-indexed-warning-green-16nov-v4`,
UID1000/dev55/inode165399134 ; nonce neuf `b8c7e19b6185c7d2a4ac2aa690ed2bfe`.
Q/logs et Q/scope ABSENTS. Q/scans conserve déjà les relevés de préparation,
dont12UNKNOWN nonattribués ; aucun n’est blanchi en zéro.

W `/home/lucied/git_workspaces/cx-lucivy-indexed-false-warning-16nov` propre890.
T UNIQUE `/home/lucied/.cache/rag3weaver-build/cx-lucivy-target-16nov`,
UID1000/dev55/inode164536077, anciennonce3552903efab590b44f44cd8002f7b028,
créateur0c795f99 ; marqueurSHA5a1671d22dc784bec78f60cf25c8b3003acc5f21e28290f05d0dfc22e0c394df.
Reçuancien PID856197/start117998665 TERMINE, scopeROUGE1attendu/restes[]/nettoyé,
identitésoldCommon absentes. Ni purge, ni nouveauT, ni adoption d’autrepropriétaire.
ELFrougeSHAc2478e972de2295a79c41cd651461653f38706e02cccd2ceace45c49fe33b01b
copié en lecture seule dansQ/red-test-elf avant toute compilation ; mêmeSHA
vérifié sur l’exécutableancien deT. Le fichierancien dansT pourra être remplacé
parCargo, sa copie et ses brutsrouges resteront inchangés.

Hashes copies figées :
- qualify-indexed-false-green.py `90fbed91205ce9ea4a75944e77c5c4354cf95c36e6e34d561048ef8e253795cf` ;
- admit-indexed-false-green.py `f8f04e0fa8375cf6fe2a841ac13d9af0d0da89662c5024d3ff98949a9fcf325a` ;
- contrôlespartagésrouges `0a269dd90b0ca745811713e21f66b45fda563074bfc06dd6993f04ab03bd2352` ;
- Common `6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea` ;
- poste `79e3a8bbf457e3bc9104aa1558e7d03072216c70b68e08ae606ace6aa4584c7b`, mode755.

## Commande littérale proposée pour GO600 distinct, NON lancée

```bash
POSTE_MEM_MAX=32G RAG3WEAVER_BUILD_MEMORY_HIGH=0 /home/lucied/.cache/rag3weaver-build/cx-lucivy-indexed-warning-green-16nov-v4/support/poste lourd timeout --signal=TERM --kill-after=5s 610 python3 /home/lucied/.cache/rag3weaver-build/cx-lucivy-indexed-warning-green-16nov-v4/admit-indexed-false-green.py
```

Après vrai poste : recontrôlechemins réels/UID/inodes/nonce/copies/source,
oldreceiptTERMINE/ELFrouge/scanspartiels frais ; scopeNEUF/END=START+600 puis
confineur seulementpourreste(minus2s), pasreset. Mémoire32G/HIGHmax, jobsMIN4,
affmoins2 etquotaCPUhiérarchique. Compileoffline de test_text_field_indexed_false,
exécutableattesté sousT, inventaireEXACT3, puis3appels --exact --nocapture :
1. unsupported_text_indexing_options_are_reported_once_per_field ;
2. warning_escapes_field_names_without_extra_lines ;
3. reporting_preserves_existing_field_indexing.
Premiercode nonzeroSTOP/firstredopenx, aucunretry. Scanpartielaprès, creatorT
inchangé, empreintes puis recontrôleEND. Résultatpilote provisoire seulement ;
VERT final exige Commoncode0/restes[] et scanownedaprèsfin, toujoursUNKNOWN
nonattribués conservés. Pascoreentière/ULP/corpus/PR42/tag/fusion/service par ce GO.

## Résultat du GO ciblé

Couple890/de364 exécuté : compileofflineOK6,24s puis3exacts1/0/0 chacun ;
CommonVERT0/restes[] et ownedabsents, sourcespropres. Relevés partiels après
fin :12UNKNOWNnonattribués conservés, pas d’absenceuniverselle déduite.
Bruts `preuves/indexed-warning-green-890c86a/`, ticketcomplété. Aucun corecomplet,
corpus7.2 ou ULP qualifié, aucune fermeture/fusion/publication automatique.
