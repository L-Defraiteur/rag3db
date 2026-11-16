# Arrêt de la qualification sur l'attente d'un accord Vertex

Source exécutée `713433bd602a6d35266fe5cd528925ba664924c9`, branche cx-archi-pas-va-au-bout-r2, produit37771e225c/base68590270. Lanceur et script épinglé publiés avant exécution, admission C7 distincte après le refus de préflight conservé au rapport03. Lib `rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`, aucun MOTEUR_ANCIEN.

## Résultats et arrêt

- Témoin exact de continuation :1/0/0,1.14s ; compile sur target neuf.
- e2e_demon_hote :24/0/0,16.28s ; filets distincts2/0/0.
- e2e_lectures :27/0/0,13.86s ; filets distincts2/0/0. Les paniques de deux témoins de refus attendus n'en font pas des rouges.
- e2e_equipier sous vrai Vertex/Gemini exigé :1passé/1échec/0ignoré en900.94s. `l_equipier_rend_le_texte_demande_et_non_un_compte_rendu` passe ; `l_equipier_ecrit_la_doc_d_un_depot_de_test_par_pas_juges` échoue au délai15min. Aucun retry.
- Hooks Claude Code, e2e_code, F8, preuve Checking, lib entière large, NAV : NON JOUÉS après le premier rouge. Aucun verdict de qualification globale vert, aucun prêt au push master.

Premier rouge immuable first-red.json : observed1174127.743181545, ligneFAILED nommée dans e2e_equipier.log. Le watcher arrête la passe ; aucun résumé vert n'est construit. Le résumé canonique garde seulement les étapes effectivement acquises et se termine sur l'entrée e2e_equipier.

## Diagnostic borné

La boîte imprimée avant la panique montre, pour la tâche de documentation : request documentaire ; ouverture agent/doc ; approval « commande en attente d'accord : rm Cargo.lock » ; report disant README.md créé ; step_done réussi ; host « en attente du superviseur depuis12:09 : approval3fc179ffee95/doc:2 ». Note9 du juge déjà émise. À la ligne134 du témoin, l'équipier tourne encore après900s.

Il s'agit du blocage observé, pas d'une attribution causale à la nouvelle continuation ou au moteur. La priorité des approvals était explicitement préservée par le lot et sa RO ED. Aucun accord n'a été donné, aucune porte élargie, aucun délai augmenté. [Ticket spécifique ouvert](../tickets/2026-11-16-le-temoin-documentaire-vertex-attend-un-accord-sans-superviseur.md) avant proposition de suite. Relecture de la fixture et témoin déterministe à décider avec C7 ; D6 réel reste non joué pour ce candidat.

## Preuves et ressources conservées

PC : `~/.cache/rag3weaver-build/cx-archi-pas-va-au-bout-r2-59ac-pass1/`. Portable : même nom suffixé `-local`.23 fichiers réhashés individuellement localement (tailles+SHA), zéro écart avec manifeste PC `a0df1f0b43898af8f2a1336a4cd60243799a3e89fc1579ff4e85f4521217fa3c`. Manifeste joint [JSON](pas-va-au-bout/manifest-red-pass1.json). Les contenus sensibles de l'autorité ne sont pas recopiés dans Git ; les preuves privées hors W gardent leur reçu terminé.

Scope validation sous autoritéPID3509552/start117264515 : ROUGE/code1/erreurNone/restes[]/nettoyétrue. Bus `/tmp/r3v-n5fsr663` absent ; cgroup terminé absent ; scan postmortem owned_live[]/owned_unknown[]. Pas d'absence universelle GPU inférée des journaux de charge, dont la charge card0 est celle du poste entier. Slot Archi rendu ; la liste après arrêt ne montrait que TSan et le nouveau slot ED, aucun processus arrêté hors lot.

Target privé `target-cx-archi-pas-va-au-bout-r2` et W PC/portable conservés. Résidu original `e2e-gardes/e2e_equipier-20261116-122352-3639876` conservé et copié sous proofs/preserved-e2e-residue (marqueur et deux livres de manifeste). charge-last.tsv/e2e-last.log copiés avant toute autre passe. Aucun service, base de Lucie, branche distante, master ou ressource d'une autre session touché. Source713433 reste figée PC jusqu'à décision ; les présents docs n'annoncent aucun nouveau run.

## Attribution disponible et limites des traces

Dernier événement bloquant identifié : approval3fc179ffee95/doc:2 pour rm Cargo.lock, puis report et step_done ; aucune ligne ended pour doc. Code relu : open_question inclut l'approval ; teammate.rs1634 donne explicitement priorité à cette attente avant le jugement de fin, et la suspension relit cette question à1418. Cela explique le chemin d'attente visible sans imputer au moteur une erreur ni changer le contrat des accords. L'origine de la décision du modèle de demander rm et les autres appels exacts ne sont pas établis.

Dans le journal e2e conservé : aucune ligne401/429, aucune valeur stopped_by ; le seul timeout probant est le délai900s du témoin. Ce n'est pas une preuve universelle d'absence d'erreur réseau : les tours détaillés n'ont pas été conservés par le témoin avant l'unwind. Sa source place runs/chats/tâches dans un TempDir local supprimé après la panique ; le résidu sauvegardé contient uniquement marqueur et livres de gabarits (clés builtin/content/written), pas ces tours. La boîte et le retour du juge restent les extraits imprimés par le témoin, tronqués à300caractères par message ; ne pas fabriquer un transcript complet.

La suite de diagnostic RO proposée à C7 est de confronter les témoins existants d'approval/attente et le contrat du scénario documentaire, puis, si nécessaire, isoler le chemin par CallbackLlm et superviseur de fixture explicite. Aucune permission de suppression accordée à la place de Lucie ; aucune règle rm/Cargo.lock ajoutée ; aucun S1/S2/S3 contourné. Nouveau run et D6 nécessitent décision distincte après correction réellement justifiée. L'arrêt annoncé de l'API7970 par G n'est pas imputé à ce témoin privé et n'a suscité aucun appel7970 de cette session.

Contrôle documentaire : un en-tête Famille qualification a été refusé par check_tickets à la première publication ; corrigé en produit dans le commit suivant, contrôle652en-têtesVERT. Ce rouge de contrôle n'a lancé aucune passe et ne modifie aucune preuve runtime.
