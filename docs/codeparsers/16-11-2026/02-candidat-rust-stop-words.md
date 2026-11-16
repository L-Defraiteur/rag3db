# Candidat mots vides Rust et chemins sûrs — préparation sans calcul

Tickets :
[les mots vides Rust](../tickets/2026-11-15-la-liste-des-mots-vides-de-rust-est-une-copie-de-celle-de-javascript.md)
et [le chemin std homonyme](../tickets/2026-11-16-un-chemin-de-la-bibliotheque-standard-se-relie-a-un-homonyme.md).
Porteur codeparsers. Reprise autorisée depuis 5ef2935d29/b48da05, fichiers réservés
code.rs, e2e_usages.rs et données/chargeur/parseur/tests Rust du sous-module.

## État publié, pas de verdict neuf

Branche rag3db cx-codeparsers-rust-stop-words, worktree
/home/lucied/git_workspaces/cx-codeparsers-rust-stop-words sur le portable.
Sous-module : branche cx-rust-stop-words publiée sur son origin avant le pointeur
parent. Le sous-module hérite du lot b48da05 ; seules ses données/fonction de
chargement/générateur neufs sont alignés en anglais : language/names, StopWords,
generate.py. Aucune liste de mots modifiée volontairement, aucun renommage des
listes anciennes std_methods et aucun changement de RUST_BUILTIN_IDENTIFIERS.
Cette adaptation **n'a pas été exécutée ou compilée**, générateur non relancé.

Les deux témoins supplémentaires sont committés avant tout correctif de la
collision : std puis appel projet nu sur une même ligne, ordre inverse avec
préfixe UTF-8. Ils restent #[ignore] comme les autres e2e, à jouer par le harnais
qui les exécute explicitement. **NON JOUÉS, rouge pas encore observé.**
Le filtrage contextuel de 5ef2935d29 reste intact ; ce candidat n'est pas prêt
à intégrer. REGLES_DE_L_ANALYSE reste7, pas8 avant la preuve.

## Contrat du correctif à faire après rouge

Utiliser l'identité de la mention avant agrégation : ligne1-based et colonne
AST en octets0-based. Lire la ligne originale dans raw[abs], vérifier frontière
UTF-8 et égalité exacte de l'identifiant à cette colonne. Le contexte trimé
line_at_byte ne s'indexe pas directement par cette colonne. Ne pas chercher
un autre homonyme sur la ligne si la position manque ou ne correspond pas.
Lire seulement le chemin contigu de cette occurrence puis qualifier sa racine
par les primitives existantes est_builtin/provenance sûre. Une référence nue
ne devient pas std parce qu'une autre occurrence qualifiée est sur la ligne.
Une position non établie ne justifie pas une abstention nouvelle devinée.

Les 14 fautes historiques collect/collections et F/Formatter restent à rejuger.
Préserver les résultats de la première forme et comparer les vrais couples,
pas les seuls totaux. Le WIP actuel a ses anciens témoins verts selon la
passation ; cela ne valide ni les nouveaux témoins ni le changement de format.

## Recette proposée après FINI G et nouveau script de validation

Aucune commande ci-dessous n'est lancée ni armée pendant la bascule.
Les anciennes bornes de passe ne se réactivent pas.

1. Fixer avec l'orchestration le master et la lib réellement posée après FINI G ;
   relire les entrants, reprendre sous nom cx neuf avant tout rebase. Préserver
   Cargo.toml/lock Lucivy4.3.9 arrivé depuis la base c039a1bfa3 ; pas générer le
   lock pendant cette fenêtre. Pointeur et lock sont revus dans le même lot.
2. Jouer les nouveaux témoins contre 5ef+format identique (sans correctif de
   ciblage), sous primitive commune acquise et borne neuve ; arrêt et diagnostic
   au premier rouge inattendu. Conserver le rouge attendu avant correction.
   Ensuite seulement implémenter la lecture de la mention et revoir ces témoins
   avec le std seul, la frontière de mot et les appels projet conservés.
3. Vérifier le générateur et les tests unitaires Rust du sous-module, puis
   e2e_usages par le harnais : aucun ignoré revendiqué vert. Source et recette
   publiées avant jeu. Ni test, Cargo, rustc ni générateur exécuté maintenant.
4. Après fenêtre et coordination banc : sonde master/bd13385 contre candidat
   sur corpus figé, racines rag3weaver/src et moteur séparées (<2000 fichiers
   par acquisition), lire les14 anciennes fautes et15–25 couples ôtés, vérifier
   disparition des107std::env::var sans retirer des appels projet justes.
5. Si les preuves soutiennent la règle, commentaire daté et REGLES8, nouveautés
   « Toutes les bases repassent une fois », fermeture des deux tickets avec
   chiffres ; puis passe_epinglee adaptée au nouveau script et à la lib posée :
   features/lib/navigateur/e2e_code/e2e_usages/e2e_code_sync/PostgreSQL requis.
   Enveloppe32G/j8, target unique ; durées proposées seulement après qualification
   des ressources. Relecture et « prêt », jamais master sans « à toi ».

Ressources actuelles : aucun build/test/service/target nouveau lancé par cette
préparation ; aucune opération dans les anciens worktrees/admins. Les WIP et
les rouges historiques restent préservés. Import ES externe hors de ce lot.
