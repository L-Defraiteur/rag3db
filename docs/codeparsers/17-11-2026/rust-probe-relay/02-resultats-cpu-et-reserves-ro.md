# Relais782b : vingt témoins CPU verts, trois réserves de couple encore ouvertes

GO distinct **C7-PARSERS-RELAY-782B-17nov60**, source exacte propre publiée
782b9deb1f4c2b96cf91ccff994ad02b32ea35c8 : une seule exécution,20/20 exécutés,
0échec/erreur/skip,rc0,0,094184s externe et0,037s unittest. Source/argv/started
et allocation enregistrés avant l'appel. Aucun SSH,/proc réel,Cargo,natif,
autorité,target ou corpus touché pendant le lot. La copie conservatoire par scp
après le lot ne constitue ni rejeu ni validation du transport réel.

Commande :timeout --signal=TERM --kill-after=2s 58s python3 -B
docs/codeparsers/17-11-2026/rust-probe-relay/test_relay.py -f -v.

Preuves identiques aux deux hôtes sous
~/.cache/codeparsers-notes/rust-relay-cpu-782b9deb1f/ :

| Fichier | SHA256 |
|---|---|
| manifest.json | 2a89eb01fdc1e3ffa25652f365ee9cf9d03662e7bf1bbaee2e013daf9e463567 |
| command.json | e65e39ad2a4aec0ff0ace5f73944ffae0cb3b04f57a4c4bc8938a3f158f5e0c2 |
| stderr | 36a2221f220ad09bf22d81ccf8609a6819f34cbdcbcfda82cdd27e81e614b38d |
| result.json | af90a6e3d027e229fdf3bf4d5ab38276d918cedd0abd7e8682d0bc91be2debc9 |

La RO TOTALE Banc1cdd0babda (docs/banc/17-11-2026/01-ro-relais-sonde-rust.md)
confirme prepared3ed75/profil81969/10copies, BASEfec/CANDc1cd inchangés et carry
non branché, mais relève **trois réserves groupées avant toute phase** :

1. SSHlaunch n'a pas de TTL distant ; file poste avant timeout1798 et abort
   exige actif. Tuer le SSH local n'atteste pas la fermeture distante en attente.
2. relay-launch publie le PID avec un argv futur avant exec ; snapshot peut
   normalement trouver encore l'argvGate et refuser ce bootstrap concurrent.
3. finish écrit/fsync le résultat SSH puis run retourne0 sans recontrôler
   phaseDeadline. ITIMER2100 soft seul ne borne pas physiquement erreur/cleanup.
   Le refus tardif du relais doit rester distinct du résultat natif déjà fermé.

Ces vingt tests ne couvraient pas ces trois chemins. Ce document et le ticket
les consignent **avant correction**. Aucun autre lot CPU n'est implicitement
autorisé ; de nouveaux témoins des vrais chemins doivent être publiés avant
les correctifs, puis revue totale du couple/pin/budget distinct demandé à C7.

Option retenue avant futur pin/RO : Cargo --no-run JSON sur FEC inchangée et
ownerBASE connu. Cargo peut réutiliser son cache, y compris les mêmes bits0c565,
sans rebuild forcé. Le modecarry custom n'est pas branché et n'est pas nécessaire
pour corriger le protocole. P02 reste le rouge d'ACK, P03 un prototype non armé.
Aucun GO natif, aucune source de production changée, REGLES7.

## Rouges complémentaires puis préparation du correctif, pas encore de vert

Surdda9 : deux FAIL attendus (TTL launch et abortqueued), puis ERROR de fixture
bootstrap (tool_root absent), STOP avant le dernier cas. Bruts conservés2hôtes,
manifest9e4d7f0ef3bdc3db32a0955673def5b45fce5721ebd7587611448ecd1701c8b4.
La fixture seule est corrigée/poussée57fac avant toute nouvelle prise.

GO C7-PARSERS-RELAY-57FAC-17nov60 : les deux cas restants sont chacun vus FAIL
unefois (futureargv exposé avant exec ; TimeoutError absent après finishlate),
0ERROR/PASSinattendu,0,179711s externe. Manifest2hôtes
62abd2eb60ce11324055510d0df7386c81f2a43725937efffe9e42c53e963699 et résultat
3f693d1d2237e07caa09018cb1aac025eeb9389870136239ad13a1bff56ecc2f.
Les deux premiers défauts ne sont pas rejoués. Les preuves précèdent les trois
correctifs groupés, désormais implantés mais non exécutés. La suite compte29
méthodes témoins, y compris CLI réel avec exec intercepté, transition snapshot,
pidfd fictif et publication/close tardifs ; pas de nouveau vert revendiqué.
