# Passe restante après la sonde : dépendances explicites, aucun calcul autorisé

Le lot porte le ticket du chemin standard homonyme et celui des mots vides Rust.
Les résultats118 (1+1+2+41verts) et la paire historique décrite à côté sont des
validations ciblées, pas une passe sur le master courant. Aucun blocage de source
pour la sonde : d395/bd et118/34 sont connus, mêmes dépendances/SDK59/configuration.

La passe complète dépend du jugement de cette paire. Elle ne peut pas recevoir
maintenant un vrai pin de source finale : il faut d'abord décider sur les liens
réels, puis rebâtir le lot sur le master entrant et relire les changements.
Ce n'est pas une attente des RO favorables déjà acquises.

Ordre restant, sans lancer de commande maintenant :

1. Qualifier les deux sorties de sonde et les couples, garder REGLES7 si une
   régression ou ambiguïté de preuve reste. Tous les rouges restent conservés.
2. Si la règle est étayée : documenter REGLES8 et « toutes les bases repassent une
   fois », puis préparer la fermeture des tickets avec le vrai périmètre prouvé.
3. Lire/fetch le master entrant à ce moment-là ; si rebase, branche neuve sans
   forcepush, aucun ancien arbre/rouge détruit. Source/parser/lock ensemble, liste
   des paquets et scripts réellement relus ; rien dans l'arbre principal de Lucie.
4. Relire tools/poste/required_suites.sh entre base et HEAD neufs. code.rs impose
   e2e_code_sync ; la passe comporte aussi e2e_code,e2e_usages,e2e_lectures,
   e2e_postgres,check_features,preuve de recompilation,lib entière sous le jeu
   large et étape navigateur exigée. Une suite requise absente n'est pas verte.
5. Préparer le pilote canonique de passe sur ces pins neufs avec les scripts
   **entrants**, pas le passe_epinglee de l'ancien d395. En particulier l'entrée
   lib doit utiliser le chemin canonique run_e2e --lib actuellement livré.
   SDK59/SHA connus ne dispensent pas de relire si le C++ entrant exige un autre
   pin ou la qualification explicite du moteur ancien ; aucune variable ancienne
   héritée sans justification.
6. Réserver **réellement** PostgreSQL auprès de C7 pour cette phase : la copie
   actuelle de passe_epinglee exporte rag3weaver_test à luciepc. Un env différent
   ne l'isole pas. Pas de prétention de base privée en conservant cet export.
   Modèles/services/React/Chromium nécessaires à la passe sont à constater et
   coordonner selon la recette réelle, jamais démarrés par ce document.
7. Budget neuf et enveloppe32GiB/high=max/quota5/j8/targetPC unique ; PQS/END de
   chaque propriétaire précédent et vrais garde/poste avant adoption. La passe
   entière aura son GO distinct, pas la somme implicite d'anciens créneaux.

Forme des commandes à figer après ces conditions (placeholders, NON EXÉCUTABLES
comme recette autorisée en l'état) :

```sh
bash tools/poste/required_suites.sh FRESH_BASE FRESH_HEAD
CIBLE=target-import-pointe-pile SUITE_DU_TEMOIN=e2e_usages \
  SUITES_EN_PLUS=e2e_postgres \
  bash tools/poste/passe_epinglee.sh NEW_WORKTREE_NAME NEW_LOG_NAME on_the_same_line -
```

Le W et le dossier de journaux du script sont des **noms**, pas des chemins
absolus. Cette commande est une charge de la portée canonique bornée, pas un
lancement direct depuis le shell utilisateur. Le témoin std/deuxordres est déjà
rouge056f puis vert7601/118 ; ne pas fabriquer un nouveau rouge en modifiant les
sources ou les dates d'un arbre pendant la passe.

Enfin : rapport/liste des suites effectivement jouées, preuves2postes,
status propre/pasrebase/origin lus, puis « prêt ». Tout code sur master attend
le seul « à toi » de C7 ; ce document n'accorde ni tour ni push ni service.
