let deck = input.arguments.result;
let cards = input.data.cards;
let policy = input.data.policy;
let total = 0; let lands = 0;
let counts = #{}; let names = #{};
let basics = #{W:0,U:0,B:0,R:0,G:0,C:0};
let sources = #{W:0,U:0,B:0,R:0,G:0,C:0};
let needed = #{W:0,U:0,B:0,R:0,G:0,C:0};
let known = []; let playable = []; let ownership = []; let copies = [];
let mana = []; let low_mana = []; let hybrids = [];
let i = 0;
for entry in deck.mainboard {
    let id = entry.arena_id.to_string(); let qty = entry.quantity;
    let path = "/result/mainboard/" + i;
    i += 1; total += qty;
    known.push(#{exists:cards.contains(id),params:#{id:id},path:path});
    if !cards.contains(id) { continue; }
    let card = cards[id];
    playable.push(#{primary:card.primary,token:card.token,params:#{name:card.name},path:path});
    if !counts.contains(id) { counts[id] = 0; }
    counts[id] += qty;
    if !names.contains(card.name) { names[card.name] = 0; }
    names[card.name] += qty;
    if card.is_land {
        lands += qty;
        for color in card.sources { if sources.contains(color) { sources[color] += qty; if card.basic { basics[color] += qty; } } }
    } else {
        for color in card.required { needed[color] += qty; }
        hybrids.push(#{hybrid:card.hybrid,params:#{name:card.name},path:path});
    }
}
for id in counts.keys() {
    let card = cards[id]; let qty = counts[id];
    if !card.basic {
        ownership.push(#{params:#{name:card.name,id:id,number:card.owned,actual:qty}});
        copies.push(#{params:#{name:card.name,number:card.copy_limit,actual:names[card.name]}});
    }
    for color in card.fetch { if basics[color] > 0 && !card.sources.contains(color) { sources[color] += qty; } }
}
for color in needed.keys() {
    if needed[color] > 0 {
        mana.push(#{params:#{color:color,number:policy.minimum_sources,actual:sources[color]}});
        low_mana.push(#{params:#{color:color,number:policy.warning_sources,actual:sources[color]}});
    }
}
#{total:total,lands:lands,policy:policy,known:known,playable:playable,ownership:ownership,copies:copies,mana:mana,low_mana:low_mana,hybrids:hybrids}
