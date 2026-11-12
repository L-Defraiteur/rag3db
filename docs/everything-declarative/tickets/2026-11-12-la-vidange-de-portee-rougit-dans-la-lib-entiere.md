# La vidange de portée rougit dans la lib entière

- **État** : ouvert ; échec de témoin observé dans la passe symboles, assertion et cause à qualifier.
- **Famille** : produit
- **Gravité** : validation globale bloquée ; aucun défaut de commande en service établi.
- **Atteignable en service** : non établi, témoin de fixture dans la lib complète.
- **Touche rag3weaver** : oui, dataflow/run_nodes.rs::tests::la_portee_videe_tue_la_commande_et_le_dit et sa portée RunScope.
- **Pour** : ED pour diagnostic commandes/portées, archi pour la preuve de passe et la revue ; pas d'édition concurrente.
- **Signalé par** : passe de navigation des symboles, 12 novembre2026.

## Faits observés

Source aee6cdec647a0be4748d420fd10bf15d3dbb7441, F5/minimal et trois compilations
verts, liblarge2252 attendus/65 ignorés, première ligne FAILED à run_nodes::tests::
la_portee_videe_tue_la_commande_et_le_dit. La passe s'arrête immédiatement : cargo
capture le panic jusqu'à la fin, donc le journal arrêté n'a ni assertion ni valeurs.
Ne pas inférer timeout, interblocage, course ou commande survivante depuis ce seul
échec. Les huit témoins purs de navigation sont verts dans ce journal ; aucun
verdict global vert n'est acquis ni absence du neuvième témoin transformée en succès.

Preuves PC : ~/.cache/rag3weaver-build/cx-symboles-aee6cdec6-reprise-20261112/
lib-large.log, erreur, inventaire.json, supervision/validation-8hzz58q8/.
La primitive a arrêté les descendants ; quiescence indépendante à relever.
Les commandes des tests concernent uniquement leurs fixtures (tail -f suivi.log),
pas de commande, accord ou donnée de Lucie. Modèles/API retirés de l'environnement,
BurnCPU ; aucun appel7879 n'a eu lieu.

## Qualification et fin

Relire le test, les événements et les opérations de portée ; recueillir l'assertion
manquante avec instrumentation appropriée et bornée (pas de retry aveugle pour
obtenir du vert). Distinguer échec déterministe, interaction du groupe de tests,
et défaut de fixture ; conserver aussi un résultat négatif et ses limites.
ED possède code/tests de commandes ; archi n'en modifie ni délais, ni assertions,
ni politique. Même budget global6000 initial de la passe, pas de reset ; nouvelle
source/recette publiée avant ciblé ou reprise, premier rouge inattendu stop.
Si défaut distinct établi, correction et preuve avant fermeture ; un ciblé isolé
vert seul ne ferme pas une panne observée sous concurrence dans la lib entière.

Lot appelant : docs/rag3weaver-archi/tickets/2026-11-12-exposer-la-navigation-dans-les-symboles.md.


## Diagnostic concurrent instrumenté

Même ELF c0ba45b0… de source aee, aucun filtre, mêmes racines/environnement,
concurrence entière, seule variation --nocapture. Premier FAILED reproduit à
run_nodes.rs1281 : « elle tourne : rien dans la boîte », AVANT portee.vider.
Ni délai de vidange, ni message final, ni kill-0/idempotence ne sont donc la
première assertion en cause. Journaux privés essai-portee-1 : stdout=l1, stderr
vide. Aucun signalement d'échec du lancement ; message prématuré à qualifier.
Preuves : cx-symboles-aee6cdec6-diag-lib-20261112 (driver final f455cc6b…).

Relecture cohérente avec ces faits : la nouvelle fixture de navigation construit
DaemonHost::new sans registre privé puis appelle shutdown. DaemonHost défaut
utilise background::registry global (daemon_host308), RunCommandNode y enregistre
sa commande (run_nodes414), shutdown appelle self.background.cancel_all (1300).
Cela peut tuer la commande du témoin voisin avant sa propre vidange ; le journal
instrumenté dit aussi arrêt d'un run en fond. Le helper hote_isole du dépôt pose
explicitement with_background(Arc::new(Registry::new())).

Correction bornée dans la fixture navigation seulement : même registre privé.
Aucune édition de run_nodes/commande/RunScope, aucun délai/assertion desserré.
Cette lecture identifie un mécanisme concret ; le ticket reste ouvert jusqu'à
la lib complète concurrente verte incluant les neuf témoins navigation ET le
même témoin de portée, suivie de la passe requise. Ne pas appeler cette correction
une preuve d'absence de toute course runtime ou une révocation ajoutée.
