# Le harnais tty perd ses traces par séance et juge l'affichage sur les octets

- **État** : ouvert — diagnostic de validation après continuation9d, aucune cause produit prouvée, aucun correctif joué.
- **Famille** : produit
- **Gravité** : coût et fiabilité du verdict de validation ; un affichage absent ne se distingue plus d'un oracle de flux brut insuffisant.
- **Atteignable en service** : non, concerne le harnais privé de qualification tty, pas un hôte partagé.
- **Touche rag3weaver** : oui, tty/mesures/mesure.py et faux_hote.py.
- **Pour** : tty.
- **Signalé par** : qualification du ticket `docs/tty/tickets/2026-11-16-les-commandes-et-options-du-terminal-en-anglais.md`, GO C7-TTY-CONT-9D8-16nov-RESTE, demande de diagnostic RO C7.

## Constat, preuves et limites

Produit6d/sourcecontrôleur9d, bâtiTTYrc0 ; F8 8/8 porté. Harnais tmux local
ouvert : premier rouge « après o, la fin ne paraît pas », puis indicateurs
Kitty absents malgré texte multiligne correct. Interruption Common propriétaire
au premier rouge **observé**, aucune reprise. Deuxième observation traitée dans
le ticket tmux15 existant, pas de doublon de défaut clavier.

Preuves figées `~/.cache/rag3weaver-build/archives/cx-tty-cont-9d8-raw/evidence.tar`
aux deux postes, SHA7cf79c5a681ac16900067227020d18d8d5524df817cab482e5c5dcd14812d188.
P `cx-tty-aliases-continuation-v1` : harness.log et first-red-stop.json.
Observation/TERM via pidfd851326.401709478, autorité705646/start85125068 ;
fincomposée851326.668131734, CommonTERMINÉ/restes[]/nettoye true, erreur15
explicite de l'arrêt propriétaire, pas une panne spontanée attribuée au produit.

**Absences assumées** : Terminal.recu garde ses chunks et timestamps seulement
en mémoire ; ecrire ne persiste pas les frappes/horodatages. hote_demarrer
supprime journal.jsonl/faux.pid à chaque séance. Le journal restant (3POST,
requestgo et describe) appartient à la dernière séance et ne prouve rien sur
l'accord de la séance principale. Pas de capture-pane de cette séance conservée,
pas de harness-result.json final après interruption. Aucun de ces éléments ne
peut être recréé a posteriori. La séquence de touches dans le script est un plan,
pas un journal observé. Le temps d'émission du premier rouge n'est pas connu ;
seul son premier constat par le contrôleur est horodaté.

## Analyse statique, sans essai ni API réelle

- La séance principale utilise le **terminal riche**, appelle Ctrl-G puis Échap,
  puis o ; aucun --plain ni ended-suivi-de-relance n'est exercé. Ce n'est donc
  pas le même contrat que `2026-11-16-le-mode-plain-et-la-fin-d-un-pas-relance.md`.
- Le faux hôte ne publie ended/stateended que pour answer avec approvedtrue.
  Les requêtes observées de la séance concernée n'étant plus conservées, on ne
  peut pas conclure si l'accord a été posté, la fin publiée ou son rendu absent.
  Ne pas modifier le protocole du faux hôte ni les états du produit par hypothèse.
- L'oracle extrait une phrase d'animation et cherche ses octets contigus après
  la frappe o, ou « fini sans répondre ». `Terminal.attendre` concatène les
  chunks du flux brut ; il n'applique pas les séquences VT/diff de ratatui/tmux.
  Une phrase visuellement présente peut être assemblée par des écritures
  partielles ; cette possibilité n'est pas une preuve du faux rouge observé.
- Le harnais sait déjà capturer un écran tmux (`ecran_tmux`, séance1f), mais
  cette primitive n'est pas utilisée pour ces deux assertions antérieures.
- Vue/clavier/animations/fauxhôte/harnais/thèmes sont sans diff entre927 et6d.
  Le lot d'alias change commandes/main/README, pas ces mécanismes. Absence de
  diff ne prouve pas l'absence de régression ; elle interdit l'attribution par
  simple coïncidence. L'aide CLI anglaise n'est imprimée qu'en cas d'option
  refusée ; les arguments français du harnais sont acceptés et le bâti est vert.
- Les seules API jouées sont celles du faux hôte privé17990. Ni l'hôte927 ni
  ses bases n'ont été interrogés ; aucune incompatibilité API927 établie.

## Plan de diff proposé, pas exécuté

1. `mesures/mesure.py` : un dossier exclusif par séance ; persister chunks bruts
   avec timestamps, frappes réellement relayées, commande/env non secrets,
   chronologie des assertions ; conserver les journaux du faux hôte avant
   d'ouvrir la séance suivante. Pas de suppression rétrospective des rouges.
2. Pour l'accord : exiger et archiver le POST answer/approved/reply_to ainsi que
   la ligne ended/state du faux hôte, séparément du rendu. Puis juger le rendu
   sur un écran reconstruit/capture-pane privée pendant la séance, sans tester
   une chaîne ANSI contiguë comme si c'était une image.
3. Réutiliser la primitive capture-pane existante avant les premières séances,
   garder le brut en complément. Tests de fragments VT factices et d'isolement
   de journaux avant tout runtime ; ne pas affaiblir les assertions produit.
4. Seulement après revue/autorisation et reçu frais : scénario ciblé borné,
   comparaison baseline avant aliases et candidat si nécessaire ; pas de
   rejeu F8/build sans changement de source justifié. Tout défaut produit
   confirmé aura son ticket/rouge avant correctif.

## Critères de fin

Les traces par séance survivent à une interruption et au démarrage de la suivante ;
les oracles distinguent protocole, état, écran et transport. Un témoin discriminant
rouge puis vert sur flux fragmenté, et un scénario d'accord privé qualifié,
permettent une attribution concrète des observations. Aucun vert déduit du
présent diagnostic ; aucune nouvelle allocation ou ancien END réactivé.

## Reprise du correctif de harnais autorisée, avant diff

C7 autorise les traces append par séance, raw/input/chrono/journal fauxhôte/env
non secrets avec fsync aux assertions/arrêts ; oracles protocole et écran
séparés, primitive capture-pane déjà utilisée1f. Témoins hermétiques publiés
avant essai : frame correcte dont le raw est fragmenté refusée par l'ancien
observateur, acceptée par l'observation d'écran ; journaux conservés entre
séances. Pas de requalification du run perdu ni de correctif produit.
SourceRustTTY reste inchangée. L'END852082.791694931 est constaté expiré à
now852108.599496 : aucun run natif possible sur ce budget, aucune nouvelle
allocation inventée. Prochain paquet RO devra refuser allocation absente/expirée.

## Correctif de harnais publié et témoins observés

Contre-exemple24aef900cb publié avant essai : deux échecs hermétiques réels,
frame correcte fragmentée refusée par l'oracle raw et ancien texte raw accepté
malgré écran courant différent. Correctif7daf342c95 : oracles tmux par
capture-pane déjà présente, protocole answer-approved/reply_to/ended/state
jugé séparément ; trois objets sont donc distincts : transport, protocole, écran.

Quatre témoins hermétiques passent ensuite : ces deux discriminants, isolation
append des traces/raw/input/screen/journal entre phases et refus de réutiliser
un dossier, indépendance du verdict protocole et écran. Aucun terminal, Cargo,
hôte, tmux ou natif lancé pour ces essais ; le run perdu reste non qualifié.

Trace par séance privée : chunksraw et frappes réellement écrites horodatés,
commande/env non secrets, journal fauxhôte non supprimé ; captures persistées ;
fsync aux assertions, aux arrêts normaux et à SIGTERM. Hôte faux local détenu
par Popen, arrêté via ce propriétaire, aucun nom de processus partagé. Modes
SSH/mosh conservent leurs oracles historiques raw et ne sont pas requalifiés
par ce correctif ciblé tmux ; captures tmux hors local restent voie existante.

Paquet `prepare_harness_review.py` prépare copies/hash/sourcepropre, F8porté,
ELFtty déjà bâti rehashé et sourceRust inchangée (seuls3fichiers mesuresPython
changent sous extension/rag3weaver). Sa gate native refuse ENDoriginal expiré,
refuse de transformer préparation en nouvelle allocation ; aucun GO réactivé.
Avant tout natif futur : revue de l'intégration, commande/env privés et budget
neuf explicitement décidé par C7, pas de fenêtre inventée dans le paquet.

## Réserve unique de revue97c, avant correction

Mémoire : approval_and_end accepte tout reply_to non vide ; le faux hôte publie
ended/stateended pour tout answerapprovedtrue et ne valide pas l'identifiant.
Un answer reply_to faux/mesure:999, avec approval réelle faux/mesure:0 et
ended/stateended, ferait donc passer l'oracle à tort. Le préfixe seul vérifié
plus loin ne suffit pas. Corréler exactement à l'unique approval attendue de
**cette phase** ; contre-exemple publié avant fix et essai hermétique. Aucun
changement produit ni assouplissement du faux hôte, pas de nouveau runtime.
Revue97c fin totale : un seul point, autres contrôles bornément reconnus.

## Identifiant d'accord corrélé exactement, réserve97c corrigée

341624693f publié avant essai :6tests,3échecs observés (identifiant étranger
au bon préfixe, approval absente et approvals multiples). Le correctif exige
une seule approval avec id non vide dans la boîte de cette phase, et un POST
answerapprovedtrue dont reply_to égale **exactement** cet id. ended et state
ended restent requis indépendamment. Aucun changement du faux hôte/du produit.
Le paquet de relecturev2 est distinct de v1 ; gate native toujours fermée sur
l'ENDoriginal expiré. Aucun témoin natif ni validation du run perdu déduit.
