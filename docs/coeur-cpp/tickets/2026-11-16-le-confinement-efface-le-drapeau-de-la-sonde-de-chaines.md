# Le confinement efface le drapeau de la sonde de chaînes

- **État** : ouvert (16 novembre2026), défaut de pilote établi, correction non faite
- **Famille** : moteur
- **Gravité** : diagnostic instrumenté non exécuté malgré environnement déclaré
- **Atteignable en service** : non, pilote privé de qualification uniquement
- **Touche rag3weaver** : indirectement, provenance des preuves moteur
- **Signalé par** : cœur C++, relecture après refus de la paire50e2
- **Pour** : cœur C++, coordination C7, hotfixs

## Contre-preuve existante

[Passe et reçus](../16-11-2026/15-paire-diagnostique-arretee-sur-sigsegv-et-sonde-non-activee.md). controller.py construit env=dict(test_env), y ajoute RAG3DB_STRING_DIAGNOSTIC=1 puis le passe à run. Mais confined(testing=True) ajoute /usr/bin/env -i depuis test_env global, et non env. env.json et argv.json conservés montrent directement la différence ; le drapeau est absent de l'environnement effectif du natif. Sonde non activée, même si le fichier déclaré dit1.

Le cas natif a reçu SIGSEGV ; ce défaut du pilote n'en démontre pas la cause et ne transforme pas son code139 en résultat GTest complet. Aucun log de valeurs ou BUILD_ID émis n'est acquis. Les sources de test et le runtime ne se corrigent pas pour masquer ce refus.

## Suite et fin

Ticket publié avant fix. Ajouter un témoin CPU qui appelle le vrai constructeur et constate le drapeau absent avant correction, puis préserver l'environnement explicite réellement consommé, sans propager d'environnement hôte. Corriger isolément, rejouer les contrôles CPU et publier la recette révisée pour RO. Aucun retry natif, référence, changement de known_red ou nouveau GO automatique. Fermer après preuve du transport exact dans le pilote admis ; ne ferme ni SIGSEGV ni C1.
