# Préparation corrective Vertex après le rouge observé

C7 autorise la préparation source et pilote, pas une exécution verte. Rouge sourcefe40/piloteeb026 conservé dans branchepreuves4d54cf4a47, manifeste39SHA141a63c6d1c0172d01a17077de5deb341ff303d107aaf0e0c7f04e644097524b. Trois positifs puis401refreshrouge ; persistant NONJOUÉ. Aucun rouge attendu repeintvert.

Avant implémentation, les scénarios transport existants sont raccordés à la future capacité AccessTokenSource conservée et deux juges ajoutés : vraie branche déclarative connect_llm avec constructeur injecté (projet/région/modèle/origine/contexte et acquisition zéro à la connexion), Debugsansjeton. Aucun env/credential réel lu dans les fixtures. Ce seam ne prétend pas tester from_env/OAuth réel : constructeurfake repointe le vrai Vertex vers HTTPloopback. Les sept fonctions restent NONJOUÉES dans cette préparation.

Le mécanisme prévu conserve TokenSource via Arc, appelle token à chaque tentative et invalide UNE fois sur HTTP401 Vertex avant réponse200. Une seconde401 est une erreur nommée. Aucune branche de reprise401 dans le lecteurSSE ; aucun changement des Bearer/APIkeys fixes, Header, retry429/transient existants. Aucun changement de protocole ou endpoint déclaré supplémentaire. Le constructeur vertex(tokenfixe) existant reste compatible ; connect_llm utilise le constructeur à capacité conservée.

Fichiers réservés : gcp_auth.rs(capacité/impl), openai_llm.rs(capacité conservée/constructeur/401), model_source.rs(connect_llm seulement), modulefixture. Pas activation.rs ni fichiers Embar. Après publication source et pilote isolé, GO CPU distinct requis ; aucun cloud/F300/service/déploiement ne découle de ce plan.

Relecture : Auth est aussi partagé par Anthropic ; la capacité Vertex reste donc un champ privé exclusif du client OpenAiLlm. with_auth remplace cette capacité par les en-têtes fixes, sans modifier Auth ni Anthropic. Le juge Debug vise le vrai client. Rustfmt parse-seulement/diffcheck valides, aucun Cargo/rustc/test lancé.
