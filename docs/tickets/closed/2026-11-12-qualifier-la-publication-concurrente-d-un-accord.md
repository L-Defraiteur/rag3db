# Qualifier la publication concurrente d'un accord

- **État** : corrigé ; publication sérialisée par demande, code054e1f1da repris dans143a21706, verdict composé vert relu.
- **Famille** : produit
- **Gravité** : republication concurrente réarmant un droit CetteFois reproduite ; aucune commande exécutée.
- **Atteignable en service** : route de carte multiworkers relue ; entrelacement reproduit sur la primitive commune, sans HTTP concurrent.
- **Touche rag3weaver** : oui, daemon_host/atelier_fichiers.rs::decider_accord et la publication vers la Garde.
- **Pour** : ED pour qualification et correction bornée, archi en revue seule du parcours et des preuves.
- **Signalé par** : revue ED/archi de la conception du mode auto le 12 novembre, relecture statique confirmée par orchestration.

## Observation et limite

Lecture sur adcf01128527f4cb24027364489bd44e322ca089,
`src/daemon_host/atelier_fichiers.rs` : decider_accord lit une demande non décidée,
relâche le verrou du registre, installe l'accord dans la Garde, puis reprend le
registre pour vérifier et écrire la décision. Deux réponses pourraient franchir
la première vérification ; une installation tardive pourrait intervenir après
une consommation CetteFois, avant que la seconde réponse soit refusée.

Cette lecture initiale ne constituait pas une reproduction (voir le résultat du préfixe ci-dessous). Le verrou de consommation de la
Garde ne suffit pas à démontrer l'atomicité de la publication amont. Le correctif
de reprise d'accord déjà livré n'est pas rouvert par cette hypothèse distincte.

## Qualification et fin

Après autorisation séparée, construire un témoin déterministe non destructif de
deux réponses au même identifiant, de leur publication et du nombre d'usages
effectivement disponibles. Couvrir reçu en erreur, révocation, annulation et
reprise avant de conclure à un défaut et de choisir le contrat commun.
Conserver les preuves ; fermer comme corrigé ou non reproduit selon le résultat,
sans annoncer de course établie sur la seule lecture. Aucun test ou appel de
commande exécuté pour cette note.

[Conception liée](../../orchestration/visions/2026-11-12-mode-auto-contextuel-et-accords.md).

## Reprise parallèle autorisée le 12 novembre

Lucie demande que les sessions libres prennent des chantiers utiles en parallèle
des couches. Orchestration confie à ED la qualification bornée de cette hypothèse,
avec archi en revue seule du parcours. Cette reprise couvre le témoin déterministe
sur registre et Garde synthétiques, sans exécuter de commande réelle ni ouvrir
les données ou accords de Lucie. Aucun modèle de décision nouveau n'est activé.

Premier résultat attendu : établir par barrières contrôlées si deux réponses
peuvent produire plus d'un usage CetteFois, puis qualifier erreur, révocation,
annulation et reprise. Conserver aussi le résultat négatif si le contrat tient.
Si le défaut est reproduit, une correction bornée doit préserver l'unicité de
publication et de consommation déjà attendue, sans changer la politique d'accord.
Publier source, diagnostic et recette avant le ciblé ; arrêt au premier rouge
inattendu, aucun rejeu inchangé. Le ticket reste ouvert jusqu'au verdict relu et,
si nécessaire, à l'intégration du correctif.

Propriété ED : atelier_fichiers et témoins de publication d'accord. Archi relit,
sans édition parallèle ; Arbre garde backend_schema et migrations. Les worktrees,
targets et fixtures sont distincts. Le créneau de push sera coordonné après la
passe des couches ; aucune réservation de master pendant le développement.

## Prise en charge ED

WT/branche cx-ed-accord-publication neuf, base43db9634a. Propriété atelier_fichiers/tests
d'accords, archi en revue seule. Extraction des deux phases existantes dans des helpers
privés réellement utilisés par decider_accord : snapshot None puis publication/commit,
sans changer leur ordre. Les fixtures utilisent cette même primitive et la vraie Garde,
jamais Autorisee::executer. Deux snapshots None figés avant une barrière ; A finit et
son usage CetteFois est consommé avant la réponse B tardive. Variante refus gagnant.
Deux rouges attendus, quatre verts de qualification : erreur de publication retentable,
refus après décision sans API de révocation, reload/restart, erreur journal après commit.
Aucune commande, donnée, accord ou configuration de Lucie ; pas de modèle/GPU/service.
Source et recette seront publiées avant le cibléCPU600 après poste, lib29/j8/Max32/target32.
Le mutex éventuel devra être par demande, hors verrou de registre pendant le backend.
Aucun correctif engagé avant la reproduction.

## Recette du premier ciblé figée avant lancement

Source rouge `b708f7c34`, module exact
`daemon_host::atelier_fichiers::publication_accord_tests::` : six témoins, deux rouges
attendus (double oui, refus gagnant/oui perdant), quatre verts ; aucun ignoré.
`bash scripts/cargo_magasin.sh test -j8 --lib --features rag3db-native,code,daemon,openai-llm -- daemon_host::atelier_fichiers::publication_accord_tests:: --nocapture`.
Recette externe rouge.sh conservée deux postes dans I-luciepc/accord-publication-20261112,
SHA256 `57ff26cdf489deee5c5d39a14b2f977a4ef7feb4ebd21e5c02e4e3dbc657e1c6`. Ciblé CPU600s global après acquisition poste lourd,
non réinitialisé si correction après diagnostic : lib29 complète, Max32/Swap4/Highmax,
RuntimeMax600/KillModecontrol-group/TimeoutStop15, targetunique32Go.
Aucun modèle/GPU/service ni exécution d'Autorisee ; XDG/state/runs/chats/taches privés,
variables de services et identifiants retirés. L'hôte fixture ne possède aucun backend.
Premier rouge inattendu : STOP/diagnostic, pas de retry inchangé. Si rouge attendu établi,
correction minimale publiée puis même module vert dans le reste du même600 ; passe
adaptée proposée ensuite, intégration seulement aprèsA et tourcoordonné.

## Rouge observé et correction bornée

Source b708f7c34, jeu exact F3 : **4 verts,2 rouges attendus,0 ignoré**. Les deux
réponses ont franchi None ; la réponse tardive rend une erreur mais sa publication
a laissé un usage CetteFois effectif dans la vraie Garde (double oui, et refus gagnant).
Aucune commande exécutée. La route HTTP/relance n'est pas exécutée dans ce préfixe ;
son exposition et l'arrêt avant journal/relance du perdant sont des lectures de code.

Correction : mutex partagé PAR demande, pris hors verrou du registre ; relecture de la
décision avant publication, mutex tenu jusqu'au commit. Le registre est relâché pendant
with_backend, préservant l'ordre des verrous backend→accords existant. Aucun changement
de portée/politique ni API de révocation ajoutée. Un septième témoin retient A après
publication avant commit, vérifie verrou par demande tenu/registre libre et B annoncé ;
le callback perdant n'est pas appelé et un seul usage est disponible. Les deux premiers
témoins gardent l'entrelacement Acommit+consommation AVANT B.
Erreur journal après effet qualifiée séparément, sans rollback inventé dans ce lot.
Source corrigée et recette verte publiées avant exécution dans le reste du même600.

Recette verte figée : source03ee5cdcf, même module désormais7/0/0 attendus ;
vert.sh SHA256 `e8ec02cd2acfe5a843c9c6dc9845c055fc6608fc4bf5b25487acd1c645983ea1`, deux postes, même dossier de preuves.
Deadline monotonic858523.253385098 issue du début857923.253385098, jamais réinitialisée ;
la phase ne reçoit que le reste réel après acquisition. Rouge2/4/0 conservé.
Les services/identifiants restent retirés, aucun modèle ni exécution de commande.

## Préfixe vert observé et relu

Source03ee5cdcf : **7 réussis,0 échec,0 ignoré**, compilation28,56s, runtime0,00s,
exit0. Rouge b708f7c34 :4réussis/2échecsattendus/0ignoré. Archi a lu directement
les deux journaux et le correctif, sans objection concrète dans ce périmètre.
ELF rag3weaver-1570695bdc8a8de7 SHA256
6bdcc333c02a04e65db701c1eb747edfe2e0d8131075434c1eef76b046a99f1d ;
source03ee embarquée/220gabarits vérifiés. Quiescence finale[], target1642372104octets.
Fin de log estimée monotonic858340,707<deadline initiale858523,253 : même600 respecté.
Preuves aux deux postes I-luciepc/accord-publication-20261112/verdict-prefixe.json.

Limites : les tests stale-snapshot sont déterministes ; le septième prouve mutexdemande
tenu, registre libre, callback perdant0 et usage unique, pas l'ordonnancement effectif
de B avant libération de A. Aucun HTTP concurrent ni absence de relance du perdant
mesurée par ces fixtures ; l'arrêt avant journal/relance découle du retour ? de l'API.
Pas de nouvelle révocation/annulation ; reload garde la décision, nouveau registre
ne rejoue pas le journal. L'erreur de reçu après effet a son ticket ED séparé.
Le ciblé vert n'est pas un verdict de passe globale et ne ferme pas le ticket.

## Jalon de globale après A et scripts communs

A et confinementcdd sont intégrés ; rebase documentaire/codeA puis branche neuve
cx-ed-accord-publication-2, source fix03 conservée. GO globale6000 de C7 reçu, recette
ED12nov/03 publiée avant lancement ; un seul budget/bus/reçu via LA primitive, pas
d'outerservice perdant l'ascendance poste. Les inventaires exacts des trois ELF seront
publiés avant runtime, sans nombre ancien imposé. ServiceBGE7879 existant coordonné
seulement pour les lectures, aucune relance M8 ni nouvelle instance. Pas de master
avant revue du verdict global et « à toi ».

Intégré sur master `0bf06b21a5fda28520f6ae0b2b2ad07acbeac331` après revue archi/C7
et verdict composé. Nettoyage privé et preuves deux postes consignés au rapport ED
du12novembre ; aucun accord/donnée/configuration utilisateur modifié par les témoins.
