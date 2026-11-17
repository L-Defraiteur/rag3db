# Runner fixture : baseline refusante et témoins d'effets prépubliés

Autorisation C728bccefaec/003950c01b lue avant écriture. Archi owner ; Recherche
reste relecteur et porteur des futurs raccords e2e. Ticket commun complété avant
ce diff. Protocole69db accepté SOURCE/FIXTURE_ONLY, pas coupon d'exécution.

Le module réel `extension/rag3weaver/scripts/execution_entry_runner.py` existe :
RunnerRefusal typée et run_fixture_invocation refusent explicitement la capacité
`backend-not-implemented`, AVANT tout effet. LIVE est unsupported sans appeler
le port ; Common final non qualifié est unsupported, pas de fallback. Le CLI
refuse encore toute capacité native. Aucun fork/ptrace/pipe/fichier/clock réel,
processus/signal/publication ou OS n'est implémenté derrière le refus.

[24 fonctions de témoins exécutables prépubliées](execution-entry-native-protocol/test_runner_effects.py)
portent les21scénarios d'effets N01–N21 et trois compléments (SIGTRAP seul, signal
final, contrôle writable avant exec). FakeEffects ne contient que données et
journal d'appels, aucun autre runner ni boucle CP, aucun syscall. Les tests
appellent LE vrai run_fixture_invocation ; ils ne l'implémentent pas dans la
fixture. Tokens object() y sont des tokens de fixture, pas capacités OS.

Seul le nominal convertit RunnerRefusal(code=backend-not-implemented) en
self.fail «fixture entry runner backend is not implemented». Autre code,
ImportError, exception de fixture restent ERROR ; assertions positives et ordre
prepare→fork/seal→publication→release→wait sont présents. Aucun import/test
n'a été exécuté. Ce nominal doit encore être vu rouge AVANT correction.
Les23autres fonctions ne sont pas proposées pour ce premier coupon.

Les témoins observent les effets MODÉLISÉS : ils ne démontreront ni ETXTBSY sur
hôte, ni PDEATHSIG/EXITKILL, ni wait réel, ni Common/CP physiques. N14 avance une
horloge de fixture au-delà du cutoff logique et garde résidu UNKNOWN ; aucune
récolte physique àSTART+125 n'est présumée. La recette native reste INACTIVE,
les helpers/runtimeQ/P/S absents. Le native-witnesses.json distingue désormais
source de tests d'effets prépubliée et tests natifs NON IMPLÉMENTÉS/NON JOUÉS.

[Pins du seul nominal proposés](execution-entry-native-protocol/runner-baseline-pins.json)
ferment module réel, tests et interpréteur, cwd/import/PYTHONPATH exacts, commande
UN nominal timeoutTERM58/KILL2/Python-B/unittest-v-f. executed=false/go=null,
prévision1FAIL/0ERRORskip/rc1 NON OBSERVÉE. Source .py homonyme absente du cwd ;
aucun import de module absent ne sera converti en rouge fonctionnel.

Demande RO baseline/témoins/pins AVANT GO rouge distinct. Après rouge réel et
conservation des bruts2hôtes seulement, préparer l'implantation des effets ;
publication/RO/GO à chaque étage avant exécution, pas de code OS testé par ce
seul étage. Le pur565b et ses51verts892bc, les rougesf74/2a sont intacts. Aucun
appelant/Cargo/canon/SDK/service/DB/target/privilège/sandbox touché. La recette et
l'API opaque restent propositions, pas capacités natives livrées.
