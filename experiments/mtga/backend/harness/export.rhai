// Décode une carte du fichier figé (champs séparés par des tabulations, ordre de prepare_deck_harness.py).
fn letters(s) { let out = []; for ch in s.chars() { out.push(ch.to_string()); } out }
fn card(line) {
    let f = line.split("\t");
    #{name:f[0], arena_line:f[1], owned:parse_int(f[2]), craftable:f[3] == "1", rarity:f[4],
      is_land:f[5] == "1", basic:f[6] == "1", required:letters(f[7]), hybrid:f[8] == "1",
      sources:letters(f[9]), fetch:letters(f[10]), copy_limit:parse_int(f[11]),
      commander:f[12] == "1", legendary:f[13] == "1"}
}
// Texte d'import Arena, regroupé par nom (une impression possédée si possible).
let deck = input.arguments.result;
let cards = input.data.cards;
let side = if deck.contains("sideboard") && deck.sideboard != () { deck.sideboard } else { [] };
let out = "";
for board in [["Deck", deck.mainboard], ["Sideboard", side]] {
    if board[1].len() == 0 { continue; }
    let order = []; let qty = #{};
    for entry in board[1] {
        // Le nom seul : Arena choisit une impression possédée (ou celle par
        // défaut). « Nom (SET) numéro » échouait sur les impressions sans code
        // importable — Special Guests numérotées 0, éditions Commander, promos.
        let line = card(cards[entry.arena_id.to_string()]).name;
        if !qty.contains(line) { qty[line] = 0; order.push(line); }
        qty[line] += entry.quantity;
    }
    if out != "" { out += "\n"; }
    out += board[0] + "\n";
    for line in order { out += qty[line].to_string() + " " + line + "\n"; }
}
out
