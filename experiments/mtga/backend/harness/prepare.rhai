// Décode une carte du fichier figé (champs séparés par des tabulations, ordre de prepare_deck_harness.py).
fn letters(s) { let out = []; for ch in s.chars() { out.push(ch.to_string()); } out }
fn card(line) {
    let f = line.split("\t");
    #{name:f[0], arena_line:f[1], owned:parse_int(f[2]), craftable:f[3] == "1", rarity:f[4],
      is_land:f[5] == "1", basic:f[6] == "1", required:letters(f[7]), hybrid:f[8] == "1",
      sources:letters(f[9]), fetch:letters(f[10]), copy_limit:parse_int(f[11])}
}
// Faits du deck proposé, calculés une fois ; chaque règle décide seule de sa validité.
let deck = input.arguments.result;
let cards = input.data.cards;
let policy = input.data.policy;
let wildcards = input.data.wildcards;
let main = deck.mainboard;
let side = if deck.contains("sideboard") && deck.sideboard != () { deck.sideboard } else { [] };
let total = 0; let side_total = 0; let lands = 0;
let names = #{}; let rows = #{};
let basics = #{W:0,U:0,B:0,R:0,G:0,C:0};
let sources = #{W:0,U:0,B:0,R:0,G:0,C:0};
let needed = #{W:0,U:0,B:0,R:0,G:0,C:0};
let known = []; let fetchers = [];
let boards = [["mainboard", main], ["sideboard", side]];
for board in boards {
    let i = 0;
    for entry in board[1] {
        let id = entry.arena_id.to_string(); let qty = entry.quantity;
        let path = "/result/" + board[0] + "/" + i;
        i += 1;
        if board[0] == "mainboard" { total += qty; } else { side_total += qty; }
        known.push(#{exists:cards.contains(id),params:#{id:id},path:path});
        if !cards.contains(id) { continue; }
        let card = card(cards[id]);
        if !names.contains(card.name) { names[card.name] = 0; rows[card.name] = card; }
        names[card.name] += qty;
        if board[0] != "mainboard" { continue; }
        if card.is_land {
            lands += qty;
            for color in card.sources { if sources.contains(color) { sources[color] += qty; if card.basic { basics[color] += qty; } } }
            if card.fetch.len() > 0 { fetchers.push(#{card:card,qty:qty}); }
        } else {
            for color in card.required { needed[color] += qty; }
        }
    }
}
for row in fetchers {
    for color in row.card.fetch { if basics[color] > 0 && !row.card.sources.contains(color) { sources[color] += row.qty; } }
}
let copies = []; let uncraftable = [];
let cost = #{common:0,uncommon:0,rare:0,mythic:0}; let to_craft = [];
for name in names.keys() {
    let card = rows[name]; let qty = names[name];
    copies.push(#{params:#{name:name,number:card.copy_limit,actual:qty}});
    let missing = if card.basic { 0 } else if qty > card.owned { qty - card.owned } else { 0 };
    if missing == 0 { continue; }
    if !card.craftable || !cost.contains(card.rarity) {
        uncraftable.push(#{params:#{name:name,missing:missing}});
        continue;
    }
    cost[card.rarity] += missing;
    to_craft.push(#{name:name,missing:missing,rarity:card.rarity});
}
let budget = [];
for rarity in ["common","uncommon","rare","mythic"] {
    budget.push(#{params:#{rarity:rarity,number:wildcards[rarity],actual:cost[rarity]}});
}
let mana = []; let low_mana = [];
for color in needed.keys() {
    if needed[color] > 0 {
        mana.push(#{params:#{color:color,number:policy.minimum_sources,actual:sources[color]}});
        low_mana.push(#{params:#{color:color,number:policy.warning_sources,actual:sources[color]}});
    }
}
#{total:total,side_total:side_total,lands:lands,policy:policy,known:known,copies:copies,
  uncraftable:uncraftable,budget:budget,to_craft:to_craft,mana:mana,low_mana:low_mana}
