# Classification PURE des RAW Vertex : préparation

Le rouge CPU après publication f4b11685ca a donné1FAIL attendu
(vrai library_summary sur lectures à deux phases) et1ERROR de harnais
StopIteration : sélection du premier for name au lieu du for de suites.
Logs conservés ~/.cache/rag3weaver-build/cx-ed-vertex-raw-cpu-red-v1.
Le StopIteration n'est pas un rouge qualifiant le filtre. Correction de la
sélection AST du témoin seulement avant second jugement ; produit/pilote
historique/RAW restent inchangés. Pas de Cargo/ELF/DB/service exécuté.

Premier jugement du classificateur6f62 :2OK puis1ERROR sousfailfast
(ValueError inventory name type), conservé green-v1 malgré le nom du dossier.
Cause exacte : un vrai nomRust contient des accents, module
search::tests_des_littéraux_bornés. L'ASCII arbitraire du classificateur
refuse le RAW valide ; pas un défaut produit ni un problème de framing.
Correction : chaque segment de chemin doit être un identifiant Unicode,
comparaison exacte aux listes figées inchangée. Aucun nom n'est réécrit.

Jugement20fefd sousfailfast :2OK+1ERROR, logs green-v2 conservés.
Le précontrôle tickets et celui de l'analyse utilisent également VERT.
Le source check_features.sh joue d'abord ses précontrôles (dont
ETAT_SUITE_E2E VERT), puis les huit jeux avec leur verdict final. Correction
bornée : suffixe APRÈS l'unique marqueur ETAT_SUITE_E2E VERT exigé égal aux
huit lignes de jeux + verdict8/8 ; pas un take-last ni une liste auto-ajustée.

## Couple PURE proposé pour revue

Code classificateur f2d6845ec2 (aucun delta code depuis), tests et listes
publiés424b52b350 AVANTimpl6f62, deux correctifs20fefd/f2d6845 avant jugement.
11 fonctionsCPU vertes0.259s (0.297s processus), mutants manquant/doublon/
nométranger/statut/ignored/shouldpanic/types/compte/filtres/header/filets/
résumédupliqué/jeuF8/final/proof. Le nombre provisoire12 communiqué était
erroné :11fonctions effectivementjouées. Souscas non comptés comme fonctions.
Ancienhelper :2FAIL/0ERROR réels sous c260, logsred-v2 conservés.

Le CLI PURE a également relu les54SHA de l'archive existante et produit
une NOUVELLE analyse RAW_CANONICAL_CLASSIFIED_ONLY dansgreen-v3 ; il ne
réécrit aucun RAW, firstred, composé ou portée. Aucun Rust, ELF--list,
chargement natif, Cargo, DB, HTTP ou cloud exécuté. Les ELF sont lus pour
SHA seulement. AncienEND expiré conservé, sans nouvelleautorité/reset.

Phases strictes : témoin7 avec2677filtrés ; lib2612OK+72ignored/0filtré ;
NAV45/0ignored/2641filtrés ; chaque e2e deux filets connus puis27lectures
ou36code,944cas du monobinaire donc942/917/908filtrés. Chaque événement
nommé/status/annotationpanic doit coïncider exactement avec l'inventaire
figé. Les diagnostics running-over ne sont pas des résultats ; un nom
inconnu, ligne malformée ou événement hors phase refuse. F8 huit jeux
fixes source/script, et résumé canonique contrôlé par phase/ordre/final.

Les listes sont OBSERVÉES APRÈS exécution et figées AVANTclassificateur ;
elles n'ont PAS l'indépendance d'un ELF--list antérieur. Un test absent des
RAW ne devient pas visible par ces listes : limites explicites, pas de
complétude indépendante revendiquée. Cardinalités/72ignored/filtres viennent
des contrats de recette et du monobinaire, ne s'ajustent pas aux listes.
Les72ignored de la lib sont consignés, jamais comptés comme passes.

Paquet ~/.cache/rag3weaver-build/cx-ed-vertex-raw-review-v1 identique deux
postes :22fichiers485696octets, manifeste
000e53dd15a047a34b2fef8805740ef3f8b3fc243e1c304f415df010d5824934 ;
preparedSHA c653aa17271c82892d92faa3ff43c52db49714583287aa530da50299e9fd189b.
22tailles/SHA réempreintés deuxpostes0écart. Fichiers de code, inventaire,
manifest54, anciennesrouges etverts intermédiaires/final y sont conservés.
Les noms green-v1/v2 sont des noms de tentative ; leurs rc1 restent ROUGES.
Inventaire SHA02455373cba66f02d8aba186c5a7de7c7f5ef2eb5bfc8fa985381c41e00cbec3.

Exécution proposée pour lecture PURE (aucune autorité runtime) :
`python3 -B classify_raw.py --archive <archive54locale> --output <fichierNEUF>`.
Le CLI refuse tout manifeste/inventaire non épinglé et toute sortie existante.

RO classificateur/paquet demandée à C7 et relecteurs. Aucune qualification
composite déduite, aucun GOmaster/retrait, pas de rerun. Ticket resteOUVERT,
commentaire obsolète connect_llm et OAuth/F300 toujours non réglés ici.
