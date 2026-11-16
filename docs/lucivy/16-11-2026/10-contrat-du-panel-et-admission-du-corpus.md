# Le panel doit être admis avec ses données avant de construire les index

Ticket existant, complété AVANT cette analyse et toute correction future :
`docs/lucivy/tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md`.
Préparation RO demandée par C7 après STOPa05e40eb28. Aucun changement de code
Lucivy, de panel, de sélection, d’assertion ou de corpus ; aucun test/Cargo/ELF.
Les anciens trois verts warnings ne sont pas rejoués. Ce document propose une
correction de recette à relire, il n’annonce pas son implémentation.

## Le contrat réel du test arrêté

Source890c86af44989f3fea6c6fa62fd40d688b4084df,
`lucivy_core/tests/test_derived_in_ram.rs`, cas
`derived_in_ram_answers_like_the_files` : le but est de comparer une même vue
ordonnée de documents entre représentation sur disque et reconstruction en RAM.
Les identités/node_id, contenus et positions doivent rester les mêmes.

Le test construit trois index : sfx3 normal, sfx3 derived_in_ram, sfx4 avec
dictionnaire et derived_in_ram. Trois garanties physiques sont vérifiées :
sidecars dérivés présents en référence et absents en RAM, réduction de taille,
option de métadonnées correctement persistée. Les documents arrivent par
commits de40 (huit commits pour300), les merges sont drainés. La comparaison
porte ensuite sur les ensembles de documents ET spans, sur onze modes de
requêtes, avant puis après réouverture des deux variantes RAM.

Pour chaque requête, la référence doit d’abord rendre au moins un document,
avant toute assertion de parité. Ce prérequis rend l’épreuve exercée plutôt
qu’une égalité entre ensembles vides. Il est pertinent et ne doit pas être
retiré. La fixture doit en revanche le satisfaire **avant** de payer construction
et I/O. Le rouge actuel a atteint et passé les garanties physiques, puis s’est
arrêté sur ce premier prérequis de requête ; il n’a pas établi de divergence RAM.

La sélection est codée en dur : `/tmp/lucivy-cmp`, DFS trié par chemin,
extensions c/h/rst/txt, taille1..60000, lecture UTF8, premier300. `common::usable`
admet dès50 fichiers. Il ne connaît ni le panel ni les témoins de contenu.
Une identité et une quantité de fichiers ne prouvent donc pas que cette vue
exerce le contrat de recherche. Les positions/relations entre documents et
leurs occurrences ne se remplacent pas par un simple hash du corpus complet.

## Observation sur les onze requêtes, sans moteur

[Audit complet et provenance](preuves/derived-panel-corpus-ro/SHA256SUMS).
15007 fichiers réguliers publics2.6.0 admis rehashés ;12133 répondent aux
critères de contenu du test. Inspection RO6,55s. Outil documentaire
`outils/audit-derived-panel-corpus.py`, SHAb7886900633e74dd340b2307db603cd74b4e32609430ff5a76a36d259f4b4074.
RapportSHA518f47c410fb4b9479268e6f5d5390202cdd1e1d2f247e9d3f65fbc4d6e329ec.
Aucune lecture.env/.vault/base ; parcours limité aux chemins du manifeste
public, contenus rehashés avant comptage. Aucun nouveau sous-ensemble choisi.

Le tableau compte des **documents repérés**, pas des hits Lucivy. Les littéraux
ASCII insensibles à la casse sont des comptages RAW exacts ; les autres lignes
emploient les locateurs indiqués, dont la portée ne remplace pas les oracles
existants. En particulier un zéro du locateur flou ne prouverait pas l’absence
floue, et une présence ne qualifie pas les spans ou l’index.

| Requête et mode exact du panel | Premier300 | Ensemble12133admissibles | Repérage employé |
| --- | ---: | ---: | --- |
| mutex_lock strict | 0 | 64 | Littéral ASCII casefold |
| mutex lock relax | 0 | 79 | Séparateurs retirés, ASCII seulement |
| spin_lock strict | 10 | 1498 | Littéral ASCII casefold |
| sched term (anchor+exact) | 6 | 1663 | Frontières alphanumériques ASCII |
| printk starts-with (anchor) | 17 | 3481 | Frontière gauche ASCII |
| return relax | 89 | 7144 | Séparateurs retirés, ASCII seulement |
| if ( strict | 26 | 6046 | Littéral ASCII casefold |
| schdule fuzzy1 | 18 | 974 | Témoins schedule à une édition, locateur incomplet |
| regsiter fuzzy2 | 103 | 4618 | Témoins register à deux éditions, locateur incomplet |
| spin_lock_[a-z]+ regex | 7 | 1300 | Regex Python ASCII, pas une qualification RustRegex |
| Mutex relax-case | 0 | 442 | Séparateurs retirés, ASCII seulement |

Pour mutex_lock, la lecture des15007 fichiers réguliers (y compris les fichiers
hors critères de sélection) trouve75 documents RAW casefold positifs. Parmi les
admissibles, un exemple épinglé est `net/sunrpc/sched.c`, hash
c8a456326c8672f0993973132c05c23989758a72b5deb4836b4241184a429f46,
première occurrence casefold à l’octet21695. C’est une preuve de présence
**ailleurs**, jamais une proposition de déplacer ce fichier dans les300.

Le manque est donc celui du **préfixe sélectionné**. Aucune absence de ces API
dans Linux2.6.0 entier ne peut être avancée. Cette observation ne suffit pas non
plus à admettre2.6.0 pour tous les modes : normalisation Unicode, tokenisation,
frontières, regex et flou/spans doivent suivre leurs contrats réels. Le test ne
code pas une version du noyau dans ses assertions de résultat ; la recette
common::recipe indique7.2 et un comparatif épinglé8d3ae59288f1. Le corpus7.2
est toujours absent ; sa compatibilité avec ces préconditions n’a pas été
observée ici. Aucune équivalence2.6/7.2, ULP, calibration ou golden n’en résulte.

## Correction proposée : composer les primitives de test existantes

Ne pas ajouter un flag de tolérance, un troisième moteur de recherche ou un
sélecteur qui cherche des données jusqu’à faire passer ce rouge. Garder pour
l’instant ce cas HOLD et le défaut d’admission explicite.

La frontière utile serait une **admission de fixture pour un panel déclaré**,
dans les utilitaires de tests : vue ordonnée/manifeste de documents + QueryConfig
existants + préconditions/témoins associés. Elle ne changerait ni API produit
ni options d’indexation. Le fichier common sait déjà exprimer absence et
scratch ; ground_truth possède déjà les logiques d’oracles grep_spans, grep_spans_fuzzy,
grep_spans_regex et filter_boundaries. Leurs enveloppes ne sont pas toutes
pures : repo_path/lecturesFS, erreurs ignorées et UTF8 avec pertes existent.
Vérifier leur couverture sémantique puis extraire/composer leurs seuls cœurs
sur les octets admis épinglés dans common,
sans importer le fichier entier avec ses tests/diagnostics. Coût : déplacement
de quelques helpers et dépendance de tests, à relire ; pas de nouvelle crate,
cache persistant, mécanisme runtime ou duplication dans les clients.

Invariants proposés pour cette admission :

1. Résoudre les identités/contenus et leur ordre exact, refuser tout chemin privé,
   lien ou empreinte inattendue avant lecture/construction. Les node_id restent
   liés aux mêmes occurrences pour les trois index et la réouverture.
2. Évaluer **tous** les prérequis du panel avec les oracles indépendants existants
   et les mêmes paramètres de QueryConfig. Publier chaque requête sans témoin,
   pas seulement la première. Un comptage RAW ou le présent audit ne suffit pas
   pour les modes flous/regex/frontières/Unicode.
3. N’admettre qu’une vue couvrant chaque précondition nonvide ; conserver les
   témoins et positions de référence, le hash de la vue ordonnée et du panel.
   Aucun ajout, permutation ou duplication implicite pour atteindre la couverture.
4. Terminer l’admission **avant** writer/index/scratch lourd. Un refus garde le cas
   nonqualifié et le profil incomplet ; ce n’est ni un skip vert ni une permission
   de synthétiser un corpus pour le test réel.
5. Après admission, conserver l’assertion nonvide de la référence ET les assertions
   de documents/spans et de fichiers : si un témoin indépendant existe mais que
   la référence reste vide, c’est un rouge de recherche à examiner. Une divergence
   entre référence et RAM devient, elle, un rouge de parité. Ne pas rebaptiser
   automatiquement tout résultat vide « problème de corpus ».

Cela distingue contenu inadéquat, référence en défaut et représentation en
RAM divergente. Le test garde son objectif ; il cesse de découvrir une fixture
inadaptée seulement après avoir bâti trois index.

## Rouge pure/mock proposé — NON codé, NON joué

Avant une correction et après RO/autorisation distinctes, poser un témoin
léger qui reproduit le défaut de l’admission par quantité :50 documents mémoire
épinglés sans aucun témoin mutex_lock, un panel strict contenant cette requête,
compteur de construction d’index initialement nul. L’admission actuelle par
`common::usable(...,50,50,...)` accepte ; l’assertion « doit refuser avant toute
construction » doit donc être vue rouge avant son correctif. Ce n’est pas une
nouvelle exécution de derived_in_ram ni un test qui rougit faute d’API compilable.

Cas à confronter ensuite, toujours sans index lourd :

| Entrée mémoire / mock | Garantie attendue |
| --- | --- |
| Quantité suffisante, témoin absent | Refus avec requête identifiée, zéro construction |
| Panel complet avec plusieurs préconditions absentes | Rapport de toutes les absences, pas un arrêt au premier motif |
| Témoins présents mais hash/ordre modifiés | Refus d’identité, pas d’adoption d’un ancien résultat |
| Un littéral présent sans frontière exigée | Refus du mode term/prefix, pas simple substring vert |
| Cas flou/regex/Unicode validés par les oracles existants | Témoins et spans exacts, pas les locateurs approximatifs de cet audit |
| Chemin privé ou lié dans le manifeste | Refus avant son contenu, compteur de lecture interdit égal à zéro |

Les verts de ces futurs témoins ne vaudraient que pour l’admission. Une passe
réelle comparative resterait soumise à une nouvelle recette avec corpus/panel
épinglés, inventaire et ledger publiés **avant** les tests, ressources/quiescence
et GO distinct. Le ledger tardif de la passe arrêtée n’est pas réhabilité.

RO demandée à Recherche sur le sens des modes et la composition des oracles,
reliée au même ticket ; aucun code Lucivy ou RUN avant retour et décision C7.
NeufHOLD/ignorés restent visibles, full_core=false, ULP7.2/PR42/main/tag horslot.

## Retour RO Recherche sur3b48e94ddc

Conception favorable bornée après lecture du core890, sans Cargo/test/ELF/base,
service ou lecture corpus par Recherche. Séparation admission/référence/parité
confirmée. Précision impérative avant code : les enveloppes d’oracles existantes
ne sont pas actuellement toutes pures. Extraire leurs cœurs sur des **octets
admis et épinglés** ; toute erreur de lecture, de hash ou de décodage contraire
au contrat est **fatale avant admission**, jamais convertie en contenu vide ou
en « aucun hit ». Aucune réouverture ambiguë par repo_path au milieu d’un oracle,
aucune conversion UTF8 avec pertes introduite pour sauver une entrée. La lecture
unique vérifiée définit les octets de la vue, les occurrences et les spans.
Cette précision remplace la qualification trop générale « oracles purs existants »
du brouillon : ce sont leurs cœurs réutilisables qui doivent devenir purs.
Ajouter au futur témoin mock une erreur de lecture/UTF8 : refus explicite avant
construction, jamais une absence de résultat. Témoin toujours NONcodé/NONjoué ;
pas d’autorisation de correction ou de passe déduite de cette RO.
