# Capture stable puis détachement avant les juges

Préparation suite arbitrageC7/diagRO ED sur v3 rouge. APP236e1e9f46f7e73eda21ac8cfa765f45a29f4595/ELF/ledgerF8v2 inchangés, aucun Cargo/F8/build/cache à rejouer. Captures SDKstables v3 acquises mais librouge nonqualifiée. Aucun nouveau runtime exécuté pendant cette préparation.

Constats source : daemon/embeddings.rs1147 embed400ms attend retour<1500ms ; daemon/mod.rs474 HTTP300ms attend<2s ; embeddings.rs1600 rerank/OCR500ms + attentes5s. Logs821140octets/plus7000threads : GDBrestaitattaché jusqu'aubout. Cause instrumentation plausible, non démontrée ; assertionsprécises nonimprimées avantshow-outputfinalinterrompu, aucun défautproduction attribué ni innocenté.

Nouveau contrôle couches-2a-union-detache/branchecx-arbre-couches-union-detache-r1, P/Q/Sv4 neufs. Mêmesbornesinitiales906224.174319861/911024.174319861, reçurestantaprèsposte−3/refus<=60 et capsMINEND−15/finalposthashprépublication conservés. Mêmesguardes/cachecarried/ledgerfixed/4SHA/copiespausepublicationdeuxpostes ; lib600 puis CPU4exacts/pub10 uniquement, premierrougeSTOP.

## Cycle exact de la SEULE invocation lib

ExportSDKUNEfois → GDB lance Python superviser-elf.py. Ce superviseur fork UNEfois, parent détaché attend waitpid(child,0). GDB follow-fork-mode child/detach-on-fork on suit cet enfant ; child exécute sdk-exec via runpy dans le mêmePython/PID, puis exec ELF. Catch execELF puis main inchangés : SDK7strict/SHAexe/CG/mêmeinferior/PIDstart/helper/mapschargées, attemptsappend-only. À main, attestation écrite avanttoutjuge puis GDB DETACH, sans continue sousptrace. MêmeELF poursuit librement ; contrôleprocTracerPid0/mêmePIDstart écrit immuablement. Parentnaturel waitpid recueille statutnatif deCETenfant (pas celui deGDB), codeexit/signalmappés conservés dans elf-exit-real.json open(x). GDB attend ce reçu sousdeadline, compare identitéparent/enfant, relaie le vraicode ; driver ET finaliseur exigent waitstatus0/exitcode0/detachTracer0/identité+ordre. GDBrc seul ne suffitjamais.

Aucune nouvelle invocation ni testfiltré, pasallongementtimeout. Logs thread-events masqués seulement pour lisibilité ; suppression d'interférence ptrace portée par DETACH avantjuges, pas par ce masquage. Si enfant disparaît avantpreuveTracerPid0/receipt absent/nonconforme, refusfermé, aucunretry. Premierrougeimmutable et cleanupCGseul inchangés, pasd'attestationaprèsrouge.

Commande APRÈS RO/GOexact C7 uniquement : python3 /home/lucied/.cache/rag3weaver-build/cx-wt-couches-union-detache/docs/arbre-principal/13-11-2026/couches-2a-union-detache/lancer.py SHA_CONTROLE_EXACT

Vérifications préparation : AST7, produitAPP→pilotevide, sdk-execinchangé, diffcheck. Aucun runtime/GDB/Cargo neuf joué ; ancienrougev3 conservé. Pasmaster/retrait/service/NAV/BGE/PG déduit.

Les reçus parent/waitpid sont publiés COMPLETS par fichiertemporaireprivé exclusif+flush/fsync+link sansécrasement ; paslectureJSONpartielle. Le driver continue supervision si GDBrc0 arrive avantreçuwaitpid, sousmêmecap, puis exige le vrai statut et quiescence.
