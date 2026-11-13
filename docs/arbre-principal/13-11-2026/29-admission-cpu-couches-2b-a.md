# 2b-A : admission CPU écrite, qualification à autoriser

Autorisation distincte C7 après8711 : code et témoins2b-A, sans attendre le landing de la garde pour PRÉPARER/QUALIFIER en CPU ; l'intégration de cette garde garde la priorité avant tout master Rust2b. WT/branche cx-wt-couches-2b-admission/cx-arbre-couches-2b-admission-r1 sur8711. Ticket fils visibilité actualisé AVANT diff. B/C restent hors implémentation.

## Frontière de cette implémentation

Module interne `publication::admission`, composé avec le résolveur2a existant : références de générations exactes, sélection inconnue/ambiguë refusée, valeurs T/H/coordonnées non réécrites. La carte gagnante reste par OccurrenceKey(entity,uuid), pas par H. Liaison des adresses de stockage par occurrence/cellule/P/modèle/offset explicitement FOURNIE PAR FIXTURE ; aucune preuve physique n'est créée. Le seul constructeur est `from_fixture`, sous cfg(test), pas de désérialisation ni paramètre d'outil créant une admission.

Arc d'admission unique porté par les services d'appel et ReadView ; copie L6 sur nouvelleconnexion conserve le même Arc et son état d'invalidité. Deux contextes distincts ou une cellule incompatible refusent, pas une précédence implicite entre registres. `sans_instantane` invalide irréversiblement ; `check_at` évalue aussi l'échéance effective, même si aucune lecture suivante n'a posé de flag. Contrôle avant projection/provenance et au point final de réponse du backend ; aucun état valide reconstruit depuis une copie après invalidation.

Les graphes d'outils réels refusent toute admission fixture (`FixtureOnly`) avant leur exécution. Lectures libres/Plan/find/meta sur ReadView sous cette admission sont refusées, get typé restreint aux occurrences admises. Il n'y a donc pas de vue physique ou d'outil public branché par ce diff ; le chargeur durable et les snapshots composés de B n'ont pas été implémentés. Le getter de provenance interne est opt-in ; aucun nouveau schéma de rendu public C n'est annoncé.

## Points communs réellement modifiés

- `occurrence_mask_options` est partagé par les trois signaux. Sans projection, intersection obligatoire par UUID avec le filtre effectif existant ; avec projection, intersection des racines AVANT résolution des versions/chunks, puis même ensemble de racines pour la projection finale. Un masque vide reste vide, jamais None signifiant toute la table.
- Dense : backend incapable de garantir le filtre refusé, pas un warning ; absence/échec de compilation du masque ne dégrade pas. Le chemin BM25 natif sans garantie de cette admission refuse ; lucivy/creux gardent la conversion stricte vers offsets. Aucun test sur index réel ne se déduit de cela.
- Scan demandé refusé sous admission, et mode auto ne contourne pas le masque par les fichiers. `strict_signals=false` n'assouplit pas la visibilité obligatoire. La finition commune vérifie le contexte et refuse un UUID invisible rendu par un backend, au lieu d'en faire un filtrage tardif servant de preuve top-k.
- `FetchRelatedNode` et le cadre de groupe passent par la même aide de lecture admise. Identité source conservée avec l'entité, collision de UUID entre entités refusée ; sources réduites avant requête, cibles admises avant la troncature par parent. Le Hop existant n'a pas de LIMIT SQL ; aucune page visible n'est perdue derrière des voisins masqués. Doublons d'arêtes visibles conservés ; sens entrant/sortant jugés. Ce n'est pas une nouvelle réécriture de Cypher par chaîne.
- `read_for_tool` ne peut oublier une admission invalide ou une cellule incompatible dans son repli. Les chemins ordinaires SANS admission gardent leurs comportements existants ; les treize témoins visent le nouveau contexte, pas une réécriture de la politique domaine/utilisateur.

## Témoins écrits, NON JOUÉS

Treize fonctions nouvelles, noms exacts dans `2b-a-juges-attendus.json`, helpers non comptés comme juges. Écrites avant l'implémentation puis renforcées :

1. Les trois signaux reçoivent le masque commun avant k sur un backend témoin CPU,20 invisibles dominants/deux visibles. Pas une qualification des trois moteurs physiques.
2. Absence de contexte, ensemble admis vide et erreur de liaison distincts.
3. H partagé : racines filtrées AVANT versions dans le vrai helper commun (connexion callback) ; pair visible/positions90/provenanceparente conservés, fuite de UUID rejetée par la finition commune.
4. Remplacement fichier dans la même révision sans effacer l'autre cellule du résolveur ; l'appel courant garde néanmoins sa frontière de cellule.
5. Filtre utilisateur ne ressuscitant pas un UUID tombstoné.
6. Relation deuxbouts/avantlimite/paramètres sources et doublons exacts, dans les deux sens.
7. Même origine, publication avancée distincte refusée comme liaison du parent épinglé.
8. Perte d'instantané invalide toutes les copies ; nouvelleconnexion ne répare pas le contexte.
9. Scan et backend incapable refusés même si les signaux sont souples.
10. Vue inconnue/ambiguë et contextefixture refusé comme autorité d'outil physique.
11. Provenance interne opt-in et différente pour occurrences héritée/locale.
12. Offset n'autorisant pas alias entre modèles ou publications de la même origine.
13. Échéance après dernièrelecture : contrôle effectif à la finition, erreur mémorisée, aucun retour valide via une horloge passée ni un rendu sans provenance.

Le témoin backend est une fixture de sélection CPU ; les connexions sont Mock/Callback. Aucune base/service/index physique n'est ouvert dans ces nouveaux tests. Portée partielle assumée : origine des lignes physiques, ouverture durable, transactions DB+handles synchronisées et accessibilité utilisateur B/C restent à qualifier séparément.

## Contrôles d'écriture et recette CPU proposée

Rustfmt a analysé les nouveaux modules et les fichiers modifiés sans erreur de syntaxe ; seuls les nouveaux fichiers ont été formatés. Diffcheck et ticketchecker sont verts. **Aucun Cargo/rustc ni test exécuté : pas de verdict de compilation, de rouge runtime ou de vert runtime.**

Proposition à arbitrer C7 sur couple source/pilote exact après RO courte :1800s uniques depuis reçu APRÈS vrai poste lourd32GiB/HIGHmax/jobs8 ; quatre rôlesCPU, target session unique cx-target-contenu-derive (retiré après2a, doit être absent avant nouveau lot), plafond32GiB, aucun SDKexport/service/GPU demandé. Busmanager attesté puis bus privé commun pour toute validation. Sources figées, sousmodule exact pour résolutionCargo, pilote/common copiés hashés horssources, whitelist sans secrets. Ce délai est une proposition, pas un GO.

Ordre prévu avec primitive de confinement connue :

1. `cargo test --lib --no-default-features --no-run --message-format=json-render-diagnostics`, cap900 intersectéEND−15. ArtifactJSON unique profiltest rag3weaver, features[] et SHA/source embarquée/copie contrôlés.
2. Inventorier ELF entier/ignorés, nouveau module13/0ignoré et publication complète23/0ignoré (10historiques+13A). Liste exacte des13 obligatoire, aucun juge deviné. Ledger publié/copiedSHAdeuxpostes AVANTruntime.
3. Treizeexacts --nocapture, cap30chacun ; premier SEUL sousGDBmain/captureprofilCPU puiscontinue MÊMEinvocation. Aucun test temporel de la lib entière gardé sousptrace.
4. MêmeELF CPU, lib ENTIÈRE --show-output/threads8 cap300 ; comparer toussuccès àinventairemoinsignorés, arrêter premierFAILED (panics attendus préservés). Les13 rejouées dans la lib restent13nouvelles, N+13invocationsbrutes pour Nfonctionsdistinctes de la lib, pas deuxfois13nouveauxjuges.
5. Hashoriginal/copie, événementsmémoire/du, arrêtquiescence et busabsent, finaliseur relisant END après hashes et avant publication. PremierrougeSTOP, logs/fixtures conservés, diagnostic avant tout correctif/GOdistinct, pasreset1800.

Pas de compilation native/SDK29 ni runtime DB qualification déduits de cette recette CPU. Aucun master Rust2b avant priorité gardeRecherche puis revues/ÀTOIexact. Le plan28/B et ses13cas restent la référence de limites ; le présent diff matérialise seulement A et ne clôture pas le ticket2b.
