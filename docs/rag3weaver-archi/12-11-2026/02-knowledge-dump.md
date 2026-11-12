# Reprise archi — 12 novembre

Lire le rapport voisin et le ticket canonique ED d'accord extérieur. ED possède le runtime,
archi uniquement le témoin navigateur chat_contextuel.rs ; un ticket commun, pas deux lots
fonctionnels concurrents. Le nominal intérieur livré n'est pas une preuve de l'accord extérieur.

Le témoin nouveau 5611fc98e garde le script au parent de rw, avec une garde starts_with sur
les chemins canoniques. La variante ajoute un argument au même script pour vérifier la ligne
exacte, pas seulement la famille ou le chemin. Le signal ne se libère qu'après ToolCallStarted
wait_output visible dans la page. Ledger des args/reçus réels ; aucune sortie inventée.

Première compilation verte, fermeture CDP Chrome à 0,49 s : pas de rouge produit, aucun clic.
Instrumenter stderr via wrapper externe aux sources, ne pas modifier les assertions pour
faire disparaître un défaut de montage. Les traces sont hors target, dossiers nommés par SHA.

La mesure a priorité. Poste peut lancer la commande dans sa propre scope systemd : placer
l'unité bornée **dans la commande après acquisition**, avec KillMode=control-group, afin que
son échéance couvre les vrais descendants et ne se consomme pas dans la file de mesure.
Pas de nouveau lot ou correctif hors du ticket d'accord sans signalement/autorisation.


Diagnostic Chrome confirmé : le socket SingletonSocket dépassait sa longueur permise avec
le scratch long. La seule correction a été TMPDIR=cx-ae ; aucun changement de fixture.
Cela a permis le rouge produit561. Après intégration aed, l'accord/reprise et wait_output sont
atteints, mais Python reçoit Permission denied sur le script extérieur : refus physique non
couvert par le ciblé backend sans sandbox. Garder ce rouge et ses stdout/stderr, ne pas masquer
le défaut en déplaçant le script ni en accordant implicitement des accès supplémentaires.
Source ED/raccord sous revue. La variante non accordée n'a pas encore été éprouvée dans Chrome.


Actualisation : la variante est désormais éprouvée dans Chrome, vert996 1/0/0 1,32s,
contre rouge8b 0/1/0 0,90s avec fixture/helper identiques. Précondition explicite : extra_read
sur le script seul avant ouverture atelier (contrat34f avant fixture996), sans droit implicite
issu de l'accord. Les rouges sans déclaration restent conservés. Attestation ELF220,
preuves complètes recopiées dans leurs dossiers distincts sur le portable. ED conduit la
globale du candidat groupé ; aucun master archi. Lire la section actualisée du rapport.

## Raccord ED final

Verdict composé sourceb42 confirmé par ED : features5/native1609/large2176/nav40 et
lectures24+démon13. Les deux e2e ont une preuve réelle de PID/cgroup/Max32/KillMode ;
les droits noyau restent déclarés séparément. La fermeture fonctionnelle accompagne le
lot ED ; le nettoyage des ressources archi est consigné par leur propriétaire, pas présumé ici.

## Nettoyage archi attesté

Archi confirme le retrait aux deux postes de son worktree cx-wt-accord-exterieur, de son
target cx-archi-target, des scratches et de ses trois branches locales, après quiescence,
archives administratives fidèles (sous-module PC inclus) et garde courante dca13abb… verte.
Remotes et preuves conservés dans cx-accord-exterieur-nettoyage-garde-20261112 ; copies
PC et portable distinctes. Une branche PC sans upstream a d'abord refusé -d ; son upstream
distant exact a été vérifié et renseigné avant le -d normal, sans -D. Cette attestation
vient d'archi ; aucun nettoyage ED n'y est assimilé.


Reprise nuit symboles : lire en PREMIER section État courant du rapport voisin.
Lot accord terminé/nettoyé, ne pas recréer ses worktrees. Symbole : branche2/docs741
préservées, source ciblée30ab sur branche1 =8/0/0, ELF223. Primitive commune requise
pour globale ; pas de réinitialisation silencieuse du600 épuisé. Scope corrective120
était séparée/autorisée, terminée verte. Ticket navigation reste ouvert, README/news
préparent uniquement première tranche. Prochain pas = recette globale publiée puis
coordination7879/PG privée, provenance de chaque binaire et garde au nettoyage.
