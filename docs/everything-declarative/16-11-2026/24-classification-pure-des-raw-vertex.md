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
