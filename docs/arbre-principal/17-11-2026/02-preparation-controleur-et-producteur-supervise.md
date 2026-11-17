# Préparation : contrôleur et producteur réellement supervisé

Préparation seulement, après le [rouge3c2](01-cycle-3c2-rouge-avant-armement-producteur.md).
Le [ticket C7 avant correctif](../../orchestration/tickets/2026-11-17-le-cycle-demarre-avant-l-armement-effectif-du-producteur-de-grant.md)
reste ouvert. Aucun ancien budget, processus, grant ou paquet n'est repris.

Un contrôleur PC dans le common cgroup possède le producteur et le driver.
Il démarre le producteur à la politique préépinglée, puis refuse d'exécuter
les précontrôles copie/DB tant qu'ARMED n'est pas vérifié. Une annonce texte,
un fichier, Popen sans poll ou un processus avant exec ne suffisent pas.
Le parent mesure indépendamment PID/start/UID/exe/argv/scriptSHA/cgroup ;
allocation, source HEAD, prepared et deux nonces lient les pièces.

Le mécanisme est commun à la préparation globale Recherche et au cycle ;
les politiques restent distinctes. Le global exige ses pièces G qualifiées
avant armement, sinon HOLD. Le cycle garde l'interdiction embedding et le
helper owner strict bcfd. Aucun PG/modèle/SDK n'entre dans le mécanisme.

L'acteur publie ARMED dans sa première itération après exec et préflight
statique. Le heartbeat est une observation de progression bornée, liée à
la même identité et au config SHA ; il ne remplace ni poll ni l'observation
OS indépendante. Avant READY et chaque action, le consommateur recontrôle
la liveness et le premier rouge. Le parent supervise également pendant les
lectures bloquantes du driver. Refus ou disparition, même rc0 inattendu,
déclenchent un premier rouge immuable, pas une attente de120s.

L'acteur utilise seulement la borne common originale, jamais start+900.
Après le grant il reste vivant sans contact santé supplémentaire jusqu'à
RELEASE contrôlé, après fin du driver. La récolte terminale est bornée par
la même échéance ; cleanup ne transforme aucun inconnu en observation verte.
Un seul watchdog physique de portée complète conserve le budget proposé
900, avec common830, réserveRO500 et phases MIN(END...). Ces nombres ne sont
pas un nouveau GO.

Premiers témoins publiés19dd4ec680 : six appels de la vraie publication READY
sont rouges0.004s sur les états non admis. Ils montrent que l'ancien raccord
ne vérifie pas le producteur ; les fixtures OS complètes, le parcours positif
et les fakes du nouveau superviseur restent à publier/juger. Aucun résultat
CPU vert ou compilation/native supplémentaire n'est revendiqué ici.

## Groupe en préparation, publication avant vérification CPU

L'acteur et son test sont portés strictement depuis4f54eef8af, après les
contre-exemples source5a0 (admission),1ddd (samplers) etae75 (chemins protégés).
Le helper owner83139 et le transport communedf45 restent inchangés.
Le sampler watcher et ceux du contrôleur/driver sont distincts et refusent
les chemins protégés avant lecture de contenu. Le contrôleur utilise les
processus basiques pour la portée et les identités Python complètes pour
les rôles exécutés ; aucun simple flag Python ne fait exemption.

P/Q/Sv5 neufs sont proposés ;22 fichiers de source/donnée attendus, Python
interpréteur capturé+hashé dans le prepared et recoupé en runtime. Configuration
SHA externe (argv_prefix+[SHA]), deux nonces avant spawn, ARM10s max sous
END−500, heartbeat10s max, release2s max sous END ; ces caps sont inclus dans
le même budget, pas des autorisations supplémentaires. Le grant reste dans
Q/PROVIDER_GO.json ; les autres traces sont dans P.

Le nouveau protocole exige producer_watcher et les observations process/loop
dans le snapshot, sans fallback legacy. Les fixtures CPU sont enrichies de
ces données synthétiques ; le code owner reste byte-identique, les fixtures
ne sont pas des permis physiques. Les six assertions initiales restent et
un chemin positif est ajouté. Les nouveaux fakes couvrent le superviseur,
le vrai raccord AST common→contrôleur, la publication READY et les hooks
policy sans contacts. Le groupe assemblé est NONJOUÉ avant publication.
Aucun dossier de runtime, processus acteur, DB, service, health, CLI, slot ou
GO correctif n'est créé par ce code publié de préparation.
