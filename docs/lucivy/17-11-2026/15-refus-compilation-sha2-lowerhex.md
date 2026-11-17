# Refus de compilation FS RED v2 — aucun rouge d'admission

[Ticket FS ouvert](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
GO exact `C7-LUCIVY-FS-RED-V2-17nov300`, suivi7e9b91368c, consommé une fois.
Commande physique14 lancée sans changement : timeout295/KILL5 AVANTPython,
sans-B, flagworker, source8b/pilotea5c/admit166d/export51a9/préflight39cc.

## Résultat observé et cause

**Compilation rc101, deux diagnostics E0277, zéro juge FS et zéro inventaire
ELF. Aucun des six rouges d'admission ni le positif n'a été observé.**

`common/fs_fixture.rs:88` appelle `format!("{:x}", Sha256::digest(bytes))`.
Avec la version exacte sha2.11.0 du lock, le tableau retourné n'implémente pas
LowerHex. Les deux targets demandés échouent sur cette même expression.
Ce n'est ni un défaut d'admission prouvé, ni un résultat de comparaison natif.
Le ledger ELF, ready et ACK n'existent pas, donc aucun --list/test n'est lancé.

Le pilote publie first-unexpected-red.json pour la phase compile, puis s'arrête.
Common annonce ROUGE101. Le wrapper Common revient1 ; l'admission écrit sa
preuve de fin avant de rendre1. Le lanceur constate le retour SSH avant ledger,
échoue par `AssertionError: launch ended before ledger publication` et rend1
(session outil6720). Aucun retry ni correction en cours de passe.

## Ressources rendues et temps disponibles

ARM PC :1261845.271922174, durée reçue276.882267260, physicalDeadlinePC
1262122.154189434. START admission1261845.569367615, END1262117.154189434.
Common **265273/start126184561**, échéance1262104.663907156, **TERMINE101**,
erreur null/restes[]/nettoyé true. Preuve externe après retour Common :
1261875.336975631, donc avant les deux échéances ; Common propre absent,
scan admis/0bloqué/12UNKNOWN nonattribués conservés. Restitution effective
signalée à C7 et Recherche après retour réel SSH/poste et session locale rc1.

Le début et la durée totale portable n'ont pas été persistés par le CLI ; ils
ne sont pas inventés. Son journal SSH est réellement vide, son stdout/traceback
est resté dans le résultat outil. La commande physique vérifiée et le retour
réel sont constatés ; cela ne mesure aucun timeout expiré/killOS/300s.

Le scan d'archivage frais1262191.736284720 est **postérieur** à END, pas une
extension de validation : Common propre toujours absent,0bloqué/11UNKNOWN.
La preuve de fin **dans** la borne reste final-evidence.json à1261875.
Aucune quiétude universelle revendiquée et aucun UNKNOWN supprimé.

## Archives avant correction

[Preuves PC recopiées et SHA](preuves/fs-red-v2-compile-refusal/SHA256SUMS),
[journal SSH portable et son reçu de copie](preuves/fs-red-v2-compile-refusal/PORTABLE_SHA256SUMS).
PC privé : `~/.cache/rag3weaver-build/cx-lucivy-fs-red-v2-compile-refusal-17nov-v1`,
fichiers444. Aucune capacité autorite.json exportée : seulement ses champs
publics PID/start/état/échéance/SHA. Logs compiler JSON, identités, recette/
enfants/verdict Common, scans et preuve finale restent lisibles deux postes.

Lock avant ce719 ; après **dbdb560202562f17536624472a0e730627df163f4bb00c53b2baf604e77637f6**.
Audit après refus : seul ajout direct sha2 dans lucivy-core, aucune autre
modification sémantique du lock. Le contrôle du pilote n'a pas été atteint
puisque compile a échoué ; cet audit est explicitement postérieur au refus.

Neuf copies inchangées, source8b propre, markerT inchangé, scratch vide.
Trois ELF historiques1ee sauvegardés RO avant compile, puis leurs SHA et les
originaux T relus identiques à l'archivage. Aucun nouveau binaire test admis.
T unique conservé, aucune purge/import/cible sœur. Corpus/natifs A/B/core/
ULP7.2/PR42/main/tag/publication non touchés et toujours hors qualification.

Suite : préparer seulement l'encodage hex du digest sha2 standard et un contrôle
de valeur connue, garder les défauts d'admission/panel/spies inchangés. Nouveau
pin/source, RO et GO distinct avant Cargo. Le GOv2 est clos. La preuve future
du portable doit aussi conserver ses métadonnées de début/fin, sans fabriquer
un journal à la main après coup. Aucun correctif de gate avant les quatre
familles de rouges Rust réellement observées.
