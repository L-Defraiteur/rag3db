# Extraction commune : préparer le déplacement constant avant la capacité

PRÉPA C7 uniquement, branchecx-codeparsers-publication-core issue du contrat2c8d.
D45/P05 et Q229bArchi restent figés. Aucun CPU,SSH,/proc,Cargo,natif,signal,
autorité,service ou target dans cette préparation.

1. Publier avant code les contre-exemples test_publication_counterexamples.py
   sur le vrai Relay D45 : charge build supplémentaire ignorée, référenceELF
   runtime ayant seulement une copie locale sans ACK antérieur, absence d'entrée
   plan4labels. Ils sont NONJOUÉS. Le manque de paramètreplan est un FAIL nommé
   de capacité absente, pas un ERROR d'import ni un faux noyau dans la fixture.
2. Déplacer à comportement constant les boucles et gardes D45 dans l'unique
   scripts/publication_relay.py ; wrappers Rust lisent/valident leur domaine et
   délèguent. Porter les points de patch/import des36témoins vers ce vrai module,
   sans changer leurs assertions. Pas de capacité quatre publications livrée.
3. Après publication/RO et budgetsCPU distincts, constater les contre-exemples
   puis préparer le plan commun/capacité. Le cas plan4 sera construit via les
   types validés, sans assouplir ordre5+1+1+1 ou identité/provenance ; aucune
   archive/inventaire de façade pour remplir les deux slots de l'ancien moteur.

Les deux premiers cas décrivent des garanties de la nouvelle frontière commune
avec un fournisseur défectueux, pas une qualification native d'une faille du
pipeline Rust : son Gate réel garantit déjà son propre jeu de fichiers et son
ordre. Le troisième montre une capacité non encore livrée ; les nouveaux types
n'existent pas à l'exécution dans la baseline. Aucun de ces échecs n'est encore
revendiqué, et la seule présence du fichier n'autorise pas à le jouer.

Le refactorconstant doit être une publication distincte de toute correction de
capacité : deux étapes et nouveaux pins avant toute admission. Le module commun
n'importe aucun pilote spécifique. Une politique/fournisseur Rust fournit les
lectures de provenance ; le module garde bootstrap/active/copies/fsync/ACK/run/
cap/cleanup/final_roles/cancel/finish/reap. Les wrappers ne conservent aucune
seconde boucle. Les modules communs chargés sont eux-mêmes épinglés dans Q.

## Étape constante matérialisée, NONEXÉCUTÉE

Le module unique contient les fonctionsD45 atomic_json/digest/publish_result,
Relay et SSHTransport, les gardes GateBase bootstrap/launcher/active/export/ack/
cancel/final_roles, et les vrais chemins relay_main/gate_main. Les adaptateurs
Rust relay.py/gate.py délèguent ; le seul fournisseur Rust lit/vérifie son
ledger, sources/parser/SDK/features et inventaires. Il n'a aucune boucle ni
copie/ACK/annulation propre. Le moteur reste explicitement celui à deux stages,
sans plan4 ni WindowEvent runtime : cette étape ne livre pas cette capacité.

Le préparateur copie et épingle publication_relay.py en plus des dix contrôleurs.
Les gardes vérifient son chemin Q et son SHA ; la ready garde l'identité de
l'entrée réelle relay.py, pas celle du module importé. Le wrapper source charge
le module de ce dépôt ; le wrapper Q exige sa copie locale et le même blob,
sans fallback PATH. Le module commun n'importe aucun pilote de domaine :
la politique de sources/provenance arrive par l'adaptateur Rust, sous les
contrôles effectifs de phase et de cycles de vie conservés.

Les36méthodes de témoins existantes conservent leurs assertions ; seuls les
imports/points de patch visent le module réellement exécuté, et la fixtureCLI
contient la nouvelle copie/empreinte de module. Aucun faux noyau ni classe de
remplacement ne fait passer ces cas. Les trois contre-exemples nouveaux restent
NONJOUÉS et la demande4stages demeure non supportée à cette publication.
Sa fixture de transport déclare les vrais quatre labels et huit occurrences,
sans implémentation de copie ou noyau caché. Aucun résultat rouge/vert de cette
étape n'est encore revendiqué ; budgetsCPU/RO exacts distincts encore requis.
