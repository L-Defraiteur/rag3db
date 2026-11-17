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
