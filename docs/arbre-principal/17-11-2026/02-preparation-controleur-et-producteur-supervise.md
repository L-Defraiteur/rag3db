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
