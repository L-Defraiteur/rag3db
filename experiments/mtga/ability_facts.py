"""Structured facts derived from an ability's English Oracle text.

Nothing is invented: every value comes from a pattern in the text. It answers
what the text alone didn't let an agent filter on. How is the ability used
(`kind`), what does activating it cost (`costs`), what triggers it
(`triggers`), what does it do (`effects`). Example: around Paradox Engine
("Whenever you cast a spell, untap all nonland permanents you control"),
look for `costs` containing `tap`. Vocabularies are closed and declared in
the schemas; an unrecognised text simply gets no value.
"""
import re

KINDS = ['keyword', 'activated', 'triggered', 'static']
# What limits an ability: a mana ability under an activation condition (Mox
# Jasper: "Activate only if you control a Dragon") produces nothing outside it.
RESTRICTIONS = ['activation_condition', 'spend_restriction', 'sorcery_speed', 'once_per_turn']
COSTS = ['tap', 'untap', 'mana', 'sacrifice', 'discard', 'life', 'exile', 'remove_counter',
         'tap_other', 'return', 'reveal', 'loyalty']
TRIGGERS = ['cast', 'enters', 'landfall', 'dies', 'leaves', 'attacks', 'blocks', 'combat_damage', 'damage',
            'becomes_tapped', 'becomes_untapped', 'upkeep', 'draw_step', 'combat_step', 'end_step', 'draw',
            'discard', 'sacrifice', 'life_gain', 'life_loss', 'counters', 'token', 'targeted', 'graveyard', 'main_phase', 'state']
EFFECTS = ['untap', 'tap', 'draw', 'discard', 'token', 'counter_spell', 'counters', 'destroy', 'exile', 'damage',
           'gain_life', 'lose_life', 'add_mana', 'search_library', 'bounce', 'reanimate', 'regrowth', 'mill',
           'sacrifice', 'pump', 'shrink', 'copy', 'scry', 'surveil', 'cost_reduction', 'fight', 'extra_combat',
           'grants_ability']

_REMINDER = re.compile(r'\([^()]*\)')
_ABILITY_WORD = re.compile(r'^[A-Z][\w\' ,-]{0,40}? — ')
_CLASS = re.compile(r'^CLASSLEVEL \[[^\]]*\] \[[^\]]*\] \[(.*)\]$', re.S)
_COST = re.compile(r'^([^:"]{1,160}?):\s')
_COST_PART = re.compile(r'^(?:(?:\{[^}]+\})+|(?:[+−-]?\d+|X|Sacrifice|Discard|Pay|Exile|Remove|Tap|Untap|Return|Reveal|Put|Mill|Collect|Forage|Waterbend|Earthbend|Airbend|Firebend)\b)')


def _clean(text):
    text = _REMINDER.sub('', text or '').strip()
    m = _CLASS.match(text)
    if m:
        text = m.group(1).strip()
    return _ABILITY_WORD.sub('', text).strip()


def _cost(text):
    """The activation cost part, or None: every comma-separated part looks like a cost."""
    m = _COST.match(text)
    if not m or '.' in m.group(1):
        return None
    parts = [p.strip() for p in m.group(1).split(',')]
    return m.group(1) if all(_COST_PART.match(p) for p in parts) else None


def _costs(cost):
    found = set()
    if '{T}' in cost:
        found.add('tap')
    if '{Q}' in cost:
        found.add('untap')
    if re.search(r'\{(?:\d+|X|[WUBRGC](?:/[WUBRGP])?|S)\}', cost):
        found.add('mana')
    if re.match(r'^\s*[+−-]?\d+\s*$', cost):
        found.add('loyalty')
    for word, name in [('Sacrifice', 'sacrifice'), ('Discard', 'discard'), ('Exile', 'exile'),
                       ('Remove', 'remove_counter'), ('Return', 'return'), ('Reveal', 'reveal')]:
        if re.search(rf'\b{word}\b', cost):
            found.add(name)
    if re.search(r'\bPay [\dX]+ life\b', cost):
        found.add('life')
    if re.search(r'\bTap (?:an|two|three|X|\w+) untapped\b', cost):
        found.add('tap_other')
    return found


def _triggers(clause):
    c = clause.lower()
    rules = [
        ('cast', r'\bcasts?\b'), ('landfall', r'\blands? (?:you control )?enters?\b'),
        ('enters', r'\benters?\b'), ('dies', r'\bdies\b|\bdie\b|put into (?:a|your|an opponent\'s) graveyard from the battlefield'),
        ('leaves', r'leaves? the battlefield'), ('attacks', r'\battacks?\b'), ('blocks', r'\bblocks?\b|becomes blocked'),
        ('combat_damage', r'deals? combat damage'), ('damage', r'deals? (?:non)?(?:\w+ )?damage|is dealt damage'),
        ('becomes_tapped', r'becomes? tapped'), ('becomes_untapped', r'becomes? untapped'),
        ('upkeep', r'\bupkeep\b'), ('draw_step', r'draw step'), ('combat_step', r'beginning of combat'),
        ('end_step', r'end step'), ('draw', r'\bdraws? (?:a|your|one|two|\w+) cards?\b|\bdraw (?:a|your) '),
        ('discard', r'\bdiscards?\b'), ('sacrifice', r'\bsacrifices?\b'), ('life_gain', r'\bgains? life\b|\bgain \w+ life'),
        ('life_loss', r'\bloses? life\b|\blose \w+ life'), ('counters', r'counters? (?:is|are) put|put (?:one or more )?\S* ?counters?'),
        ('token', r'\btokens?\b'), ('targeted', r'becomes? the target'), ('graveyard', r'graveyard'),
        ('main_phase', r'main phase'),
        ('state', r'^when there (?:are|is)|^when you control no|^when .* has no'),
    ]
    return {name for name, pattern in rules if re.search(pattern, c)}


def _effects(effect):
    e = effect.lower()
    rules = [
        ('untap', r'(?<!doesn\'t )(?<!don\'t )\buntap\b(?! step)'), ('tap', r'\btap (?:or untap )?(?:target|up to|all|each|another|it|that|those|x|two)\b'),
        ('draw', r'\bdraws? \w+ cards?\b|\bdraw (?:a|that many|cards)'), ('discard', r'\bdiscards?\b'),
        ('token', r'\bcreate\b[^.]*\btokens?\b'), ('counter_spell', r'\bcounter (?:target|that|it|up to)[^.]*\b(?:spell|ability)'),
        ('counters', r'\bput (?:a|an|one|two|three|x|that many|\w+) [^.]*?counters? on\b'), ('destroy', r'\bdestroy\b'),
        ('exile', r'\bexiles?\b'), ('damage', r'\bdeals? [^.]*?damage\b'), ('gain_life', r'\bgains? [^.]*?life\b'),
        ('lose_life', r'\bloses? [^.]*?life\b'), ('add_mana', r'\badd (?:\{|one mana|two mana|three mana|x mana|an amount|mana)'),
        ('search_library', r'search (?:your|their) library'), ('bounce', r"return [^.]*? to (?:its|their) owners?'? hands?"),
        ('reanimate', r'return [^.]*?from (?:your|a|their) graveyard to the battlefield'),
        ('regrowth', r'return [^.]*?from your graveyard to your hand'), ('mill', r'\bmills?\b'),
        ('sacrifice', r'\bsacrifices?\b'), ('pump', r'\bgets? \+\d+/\+\d+|\bget \+x/\+x|\bgets? \+x'),
        ('shrink', r'\bgets? -\d+/-\d+|\bgets? -x'), ('copy', r'\bcop(?:y|ies)\b'), ('scry', r'\bscry\b'),
        ('surveil', r'\bsurveil\b'), ('cost_reduction', r'\bcosts? [^.]*?less\b'), ('fight', r'\bfights?\b'),
        ('extra_combat', r'additional combat phase'),
        ('grants_ability', r'\b(?:has|have|gains?) "'),
    ]
    return {name for name, pattern in rules if re.search(pattern, e)}


def ability_facts(text_en):
    """kind, costs, triggers and effects of one ability, each in a declared vocabulary."""
    text = _clean(text_en)
    cost = _cost(text)
    if cost is not None:
        kind, costs, triggers, effect = 'activated', _costs(cost), set(), text[len(cost) + 1:]
    elif re.match(r'^(When|Whenever|At)\b', text):
        clause, _, effect = text.partition(', ')
        kind, costs, triggers = 'triggered', set(), _triggers(clause)
    elif '.' not in text and len(text.split()) <= 6:
        kind, costs, triggers, effect = 'keyword', set(), set(), ''
    else:
        kind, costs, triggers, effect = 'static', set(), set(), text
    effects = _effects(effect)
    # Une capacité accordée (« Lands you control have "{T}: Add…" ») : son coût
    # compte pour qui cherche ce qui s'engage, même si elle vit sur l'autre.
    for accorde in re.findall(r'"([^"]+)"', effect):
        inner = _cost(accorde.strip())
        if inner is not None:
            costs |= _costs(inner)
    t = text.lower()
    restrictions = {name for name, pattern in [
        ('activation_condition', r'activate only if|activate only during|activate only while'),
        ('spend_restriction', r'spend this mana only|this mana can\'t be spent|can\'t be spent to'),
        ('sorcery_speed', r'activate only as a sorcery|activate only any time you could cast a sorcery'),
        ('once_per_turn', r'activate only once each turn|only once each turn'),
    ] if re.search(pattern, t)}
    return {'kind': kind,
            'restrictions': [r for r in RESTRICTIONS if r in restrictions],
            'costs': [c for c in COSTS if c in costs],
            'triggers': [t for t in TRIGGERS if t in triggers],
            'effects': [e for e in EFFECTS if e in effects]}


def card_facts(abilities):
    """A card's facts: the union of its abilities', in vocabulary order."""
    facts = [ability_facts(a['text_en']) for a in abilities]
    union = lambda key, vocab: [v for v in vocab if any(v in f[key] for f in facts)]
    return {'ability_kinds': [k for k in KINDS if any(f['kind'] == k for f in facts)],
            'ability_costs': union('costs', COSTS),
            'triggers': union('triggers', TRIGGERS),
            'effects': union('effects', EFFECTS),
            'ability_restrictions': union('restrictions', RESTRICTIONS)}
