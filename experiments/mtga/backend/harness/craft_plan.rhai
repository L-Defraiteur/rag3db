// Décode une carte du fichier figé (champs séparés par des tabulations, ordre de prepare_deck_harness.py).
fn letters(s) { let out = []; for ch in s.chars() { out.push(ch.to_string()); } out }
fn card(line) {
    let f = line.split("\t");
    #{name:f[0], arena_line:f[1], owned:parse_int(f[2]), craftable:f[3] == "1", rarity:f[4],
      is_land:f[5] == "1", basic:f[6] == "1", required:letters(f[7]), hybrid:f[8] == "1",
      sources:letters(f[9]), fetch:letters(f[10]), copy_limit:parse_int(f[11])}
}
// Jokers à dépenser, pour relecture humaine ; rien n'est fabriqué.
let cards = input.data.cards;
let wildcards = input.data.wildcards;
let deck = input.arguments.result;
let side = if deck.contains("sideboard") && deck.sideboard != () { deck.sideboard } else { [] };
let names = #{}; let rows = #{};
for entry in deck.mainboard + side {
    let card = card(cards[entry.arena_id.to_string()]);
    if !names.contains(card.name) { names[card.name] = 0; rows[card.name] = card; }
    names[card.name] += entry.quantity;
}
let lines = []; let cost = #{common:0,uncommon:0,rare:0,mythic:0};
for name in names.keys() {
    let card = rows[name];
    if card.basic || names[name] <= card.owned { continue; }
    let missing = names[name] - card.owned;
    cost[card.rarity] += missing;
    lines.push("  " + missing + " × " + name + " (" + card.rarity + ")");
}
let text = "Jokers : " + cost.common + " C / " + cost.uncommon + " U / " + cost.rare + " R / " + cost.mythic + " M"
    + " (disponibles : " + wildcards.common + " / " + wildcards.uncommon + " / " + wildcards.rare + " / " + wildcards.mythic
    + ", relevé du " + wildcards.captured_at + ")";
if lines.len() == 0 { text += "\nTout est déjà dans la collection."; } else { text += "\nÀ fabriquer :\n" + lines.reduce(|s, l| if s == () { l } else { s + "\n" + l }); }
text
