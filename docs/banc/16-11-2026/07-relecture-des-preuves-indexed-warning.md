# Relecture RO des trois preuves ciblées indexed:false

Ticket existant :
`docs/lucivy/tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md`.
Preuves figées : cae3f3276e9ec473ba3ea7d4f551b786e48dff3b,
`docs/lucivy/16-11-2026/preuves/indexed-warning-green-890c86a/`.
Source890c86af44989f3fea6c6fa62fd40d688b4084df/pilote de36488bd948babf25548f78e80dcf7e89c5fe95
selon la recette et le ticket. Aucune exécution nouvelle : lecture des seuls
objets git publiés, sansCargo/rustc/décodeur, scan/proc, base ni service.

## Cohérence effectivement vérifiée

Les28 fichiers de SHA256SUMS sont intégralement rehachés ; tailles non annoncées
par ce format, aucune vérification de taille inventée. Les empreintes internes
logs/hashes.json concordent avec ce même manifeste. SHA256 de SHA256SUMS :
`a99c45adcd03fea6dce9f700cc3e17a204ae4548ae24e6161f008b656d8e3b2d`.

Les trois sorties brutes concordent avec les commandes --exact et l'inventaire3 :

| Test exact | Réussis | Échoués | Ignorés |
| --- | ---: | ---: | ---: |
| unsupported_text_indexing_options_are_reported_once_per_field | 1 | 0 | 0 |
| warning_escapes_field_names_without_extra_lines | 1 | 0 | 0 |
| reporting_preserves_existing_field_indexing | 1 | 0 | 0 |

La compilation enregistrée est --offline/--no-run/-j4 ; son événement
build-finished est success=true. Le fichier nommé compile.jsonl contient aussi
8lignes nonJSON de journal : elles sont conservées, pas traitées comme des
événements structurés. Le troisième test montre deuxWarning réels, champs body
et label, disant que text/string restent indexés. Un avertissement ne constitue
pas l'implémentation du désindexage demandé par indexed:false.

Reçus ELF vert d10152c78102ee701ef2b808fc2627d7e8044866fcbc5cebe9e5a23550958adc,
rouge conservé c2478e972de2295a79c41cd651461653f38706e02cccd2ceace45c49fe33b01b,
cohérents entre preuves. Leurs **octets binaires ne sont pas rehachés par Banc** :
aucun accès au target n'était nécessaire pour cette relecture documentaire.

## Portée du superviseur et métadonnées partielles

Reçu final VERT/code0/erreur=null/restes[]/nettoyétrue. Mémoire effective32Gio,
jobs4, même cgroup aux reçus d'entrée/commun. Le contrôle final est avant
l'échéance commune héritée ; aucun nouveau budget déduit du résultat provisoire.
Autorité enregistrée termine, green_common_owned_alive vide, source_clean vrai.
Ce sont des attestations enregistrées corroborées, pas un nouveau contrôle de
processus effectué aujourd'hui par Banc.

Les UNKNOWN non attribués ne sont pas une absence : entrée12, scan après tests13,
relevé final12. Les listes, erreurs de permission et métadonnées partielles sont
conservées ; tous ces UNKNOWN sont owned=false/attributed=false dans les pièces
vérifiées, blocked vide. Ce contrat borné n'affirme aucune quiescence universelle
de/proc, aucun retrait ni accès à un cache tiers.

## Avis borné et suites

**Avis favorable de cohérence aux trois tests ciblés et à leur clôture de
portée**, sans élargir ce vert. Core complet, corpus réel7.2 et ULP restent NON
QUALIFIÉS par ces pièces. Aucune permission de fusion/PR42/tag, téléchargement
ou autre lancement n'est déduite ; le refus d'approbation de PR42 reste HOLD.
Le ticket Lucivy reste ouvert pour son intégration et sa qualification propres.

Consolidé indépendant : `lucivy-indexed-warning/review.json`, SHA256
`768865d8cdd7312c82943bad9368782119b3abc5bd5212ca215bdc699ee522fc`.
Script de relecture et consolidé local conservés sous
`~/.cache/rag3db-banc-notes/lucivy-indexed-warning-review-16nov/`.
Cette mission ne relance ni le banc de corpus terminé ni l'ancien oracle de
migration, toujours séparé. Aucun RUN ou correctif effectué dans cette relecture.
