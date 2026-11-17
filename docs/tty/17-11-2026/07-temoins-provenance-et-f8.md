# Témoins R2 supplémentaires, avant correction et sans exécution

Arbitrage C7 32b9388439 : différer le coupon8 verts, grouper provenanceELF
et F8extra avant nouvelle qualification CPU. Ticket anglais OPEN complété
et publié e30c138eec avant les sources ci-dessous.

`test_qualification_review.py` est BYTEinchangé (d998…4fb6f), huit assertions
et should_panic conservés. `test_qualification_provenance.py` lance ces huit
cas puis six nouveaux, sans importer l'entrée du driver. Les fonctions réelles
results et feature_results sont extraites par AST ; stage reste celui du driver.

Pour rendre le vrai raccord F8 appelable, le bloc existant est extrait dans
feature_results puis appelé par le pipeline. Même algorithme volontairement
défectueux, aucun refus supplémentaire ajouté avant observation rouge.
Les tests ELF présentent noms et chemins identiques ; seules changent les
vraies bytes des fichiers virtuels/empreintes de catalogue et d'exécution :
remplacement, chemin restauré après exécution du mauvais contenu, mutation
postérieure. Un contrôle positif utilise le même contenu partout.
Les valeurs de recette d'exécution sont des fakes, pas des preuves runtime.
Elles ne rendent aucun ELF natif ou capacité admissible.

F8 : contrôle des huit jeux exacts et contre-exemple neuvièmeVERT étranger
avec une dernière ligne8/8 inchangée. Le raccord courant filtre encore ce
neuvièmeVERT : c'est précisément le refus manquant à observer.

Prévision statique pour une future invocation sansfailfast :14cas,
10PASS (8baselines+2positifs nouveaux),4FAILvisés,0ERROR0skip. **NON JOUÉ**.
Un ERROR de fixture/import/thread n'est pas le rouge du mécanisme demandé.
La syntaxe AST et le SHA des huit témoins sont seuls vérifiés ici.

Qv2c003 et Qv3cafd restent conservés INACTIFS. Ils ne sont pas repinés sous
cette source et ne doivent pas être lancés. AucunCPU/native/no-run/list,
service/acteur/SQL/slot/3600 ni mutation target. Suite : pins/source publiés,
ROtémoins puis GOCPUbaseline14distinct, RAW2hosts avant nouvelle correction.
La chaîne réelle d'exécution ELF dans le canon reste à composer : un chemin,
un catalogue ou une recette déclarée ne prouve pas à lui seul le contenu joué.
