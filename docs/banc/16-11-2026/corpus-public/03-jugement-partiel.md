# Premier jugement indépendant, préfixe en cours

Oracle avant sorties905bf17f36,12fichiers/109faits. À cette capture, huit lignes
d'échantillon sont disponibles : C/C++/JavaScript/Python, deux fichiers chacun.
Rust et TypeScript ne sont pas encore représentés dans le préfixe figé. Aucun
reparse ni lecture de corps autre que les sources préparées pour l'oracle.

Capture PC language-corpus-02b/campaign/results.jsonl, ELF bff1cafb...5565e2.
Préfixe compté :2500C,2500Cpp,2500C#,1041CSS,2500Go,1761HTML,2500JS,
2500Markdown,211MDX,2500Python,45Rust. **Ce ne sont pas des comptes finaux**.
Copie figée : oracle-review-01/sample-results.jsonl, SHA256
b701cd4ebf9c8b2444328dacfcbedf90fa6176129dbee354a712fe5bd8e661f5.

Les faits du JSON avant résultats sont jugés sans changer les attentes :
53observed,16not_exposed,40unjudged, aucune définition obligatoire déclarée
manquante sur ces huit lignes. Les40incluent les quatre fichiers non encore
capturés ; ce n'est pas un taux de réussite. Qualificateurs static/async/signatures
et références individuelles ne sont pas prouvés par la seule présence d'un nom.
Les spans de scope sont comparés aux ancrages ligne ET octet ; les membres ne
retiennent parfois qu'une ligne, et cette limite est conservée.

| Langue de l'échantillon | Faits effectivement corroborés | Limites |
| --- | --- | --- |
| C | six fonctions aux bons spans, includes de curl aux lignes attendues | callback-table/macro/qualificateurs non retenus individuellement par le compact |
| C++ | trois SliceCast distincts, hiérarchie des namespaces, classe ResourceTracker et méthodes/membres correctement parentés | forward intentionnellement non exposé comme définition ; type pointeur/qualificateurs à distinguer de la simple chaîne de type |
| JavaScript | quatre fonctions Certificate aux bons spans, vraie classe CopyrightBannerPlugin/constructor/héritage, bindings CommonJS | prototype/static assignment ne sont pas une seconde définition de fonction ; références individuelles non retenues |
| Python | Loader et ses quatre méthodes parentées ; douze classes statistiques et deux __init__ de propriétaires distincts ; héritages/imports | attributs de classe et références individuelles non corroborés par les champs compacts disponibles |

Anomalie supplémentaire découverte après ouverture des sorties, **hors des109
attentes préfixées** : Namespace C++ internal reçoit les paramètres génériques
Result,T du template forward suivant. AST valide et aucune troncature ; c'est
un attribut wrong confirmé. Ticket créé avant transmission/correctif :
`docs/codeparsers/tickets/2026-11-16-un-template-forward-cpp-contamine-le-namespace.md`.
L'extracteur ne crée intentionnellement pas un scope de définition pour le
forward sans body ; cette limite n'est pas transformée en faux défaut.

Jugements détaillés locaux : oracle-review-01/judgments.json, SHA256
953eb7915c2685ec3f676414a7b298fe982fd562fe3101be9ae5f2a51e797e87.
Tous les champs d'identité d'entrée et leurs hash sont conservés. Les dimensions
non retenues par l'outil ne sont pas attribuées à tort comme absences de l'IR.
La campagne n'a pas de verdict final à ce stade, le parent reste ouvert. Pas de
correctif parser, pas de modèle/GPU/service/DB. Les584générés restent une annexe
corrective, sans modifier gel02 ni redistribuer les quotas après les résultats.
