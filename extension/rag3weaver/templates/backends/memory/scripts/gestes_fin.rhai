// gestes_fin — la fin d’un geste : l’ancienne fiche à retirer (après les recopies), et le compte rendu.
// input : la liste des groupes passés par tous les étages (le list du dernier ForEachNode).
let gs = input;
if type_of(gs) != "array" || gs.is_empty() { throw "geste : aucun groupe n’a passé les étages"; }
for g in gs { if type_of(g) == "map" && g.error != () { throw `geste : un groupe a échoué : ${g.error}`; } }
let premier = gs[0];
let apports = 0;
for g in gs { apports += g.copies.len(); }
#{
    lignes: [ premier.ancienne ],
    rendu: #{ geste: premier.mode, retiree: premier.ancien, fiches: gs.map(|g| g.nom), genre: premier.genre, apports_recopies: apports }
}
