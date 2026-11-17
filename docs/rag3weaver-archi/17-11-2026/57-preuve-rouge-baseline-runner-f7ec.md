# Rouge nominal du runner fixture acquis avant correction

GO **C7-ENTRY-RUNNER-BASELINE-F7EC-17nov60**, publié8b54d1bbac et lu avant
entrée. Source exacte **f7ec23843801b1972244f493aa242977c19db593**, WT propre,
runnerbb46/test92467/Pythonc81d/pur565b réempreintés frais conformes. Trois RO
baseline M/R/TTY et lecture C7 conservées.

UNE invocation du seul nominal de runner-baseline-pins.json, timeoutTERM58/
KILL2/Python3.14-B/unittest-v-f, cwd et PYTHONPATHscripts exacts. Résultat :
**1 FAIL, 0 ERROR, 0 skip, rc1**, message exact
`fixture entry runner backend is not implemented`. La trace montre l'import du
vrai module partagé, son RunnerRefusal(backend-not-implemented), puis self.fail
du seul nominal. Pas d'erreur d'import ou de fixture. Les23autres fonctions
restent NON JOUÉES. Aucun retry ni test supplémentaire.

START monotonic local **893779.094259265**, END **893779.138851763**,
enveloppe **0.044592497986741364s**. Unittest affiche0.000s, mesure différente.
Il s'agit de l'API refusante d'orchestration MODELED ; aucun fork, ptrace, signal,
fichier ou clock réel du protocole natif n'a été déclenché par ce test.

Racine exclusive hors WT, identique LOCAL et PC :
`~/.cache/rag3weaver-build/cx-archi-runner-baseline-red-f7ec-17nov-r1`.
Neuf fichiers : sources runner/test/pur, pins, source-command, stdout, stderr,
résultat et manifeste. Huit entrées et manifeste réhashés sur les deux hôtes,
**0 écart**, AVANT toute correction du runner. SHA manifeste :
`db4805bc3165f0a617a083121f82adea82bea3bf2033d52d07b0583c86067528`.
Sources et pins archivés intacts ; les anciennes archives f74/2a/892bc ne sont
pas modifiées. Les51verts du module pur565b restent une preuve distincte.

Ce rouge permet seulement la PRÉPARATION du corps FIXTURE_ONLY selon le GO.
Nouveaux sources/pins/RO puis GOverts distinct avant test ; pas deGO24 ou natif
implicite. LIVE, CP/Common physiques, SDK/capture/profile natif, appelants/Cargo,
services/targets/privilèges/sandbox ne sont ni qualifiés ni modifiés. Aucun P/Q/S
natif créé ; l'archivage des bruts sur deux hôtes ne lance pas le protocole.
