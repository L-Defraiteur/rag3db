# Reprise — grep et navigation commune

Lire le rapport voisin puis le ticket canonique. Code partagé : code_tools.rs,
son sous-module privé code_tools/grep_symbols.rs, dataflow/code_nodes.rs,
symbol_navigation.rs et les deux gabarits grep/atelier_grep_files.mmd.
Lib.rs/Cargo/mail/activation appartiennent à d'autres lots et ne sont pas édités ici.

Le sélecteur de position emploie le premier match regex de chaque ligne, en octets
UTF-8 exacts du contenu observé ; la recherche reste ligne par ligne. Une position
à fin exclusive n'est pas contenue par cette plage. Le minimum n'est retenu que
si tous les autres candidats le contiennent strictement ; sinon candidats explicites.
Les parents partagent la même préparation que navigate_symbols, sans seconde
lecture des corps. L'ancien scope indicatif du défaut0 n'est pas promu en preuve.

Pour les fixtures hôte : background Registry privé, embeddings.catch_up_on_open
(faux), pas workspace.catch_up_on_open ; watches={} pour le cas périmé.
Les appels MockLlm with_tool_calls demandent String explicite, pas .into ambigu.

Ciblé1800 initial terminé vert (12+2) après deux rouges documentés. Aucun budget
renouvelé pendant les correctifs. Future globale = étape distincte à publier et
faire relire par C7 ; pas de rejeu pour occuper. Native29 fixée, CPU/j8/Max32G/
target32Go, primitive confiner_validation.py directe. Garde courante au nettoyage,
archives fidèles avec sous-module privé et inventaire des dépendances du scratch.
