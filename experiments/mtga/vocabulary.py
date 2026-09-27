"""Closed vocabularies and examples declared in the backend's JSON schemas.

The engine reads them to describe the filters of the exposed tools (values,
example): an agent then writes `"Red"` for a color and `"R"` for a mana
symbol without guessing. Only rules concepts that are stable are listed here;
subtypes and land types change with every set and stay open.
"""
from ability_facts import COSTS, EFFECTS, KINDS, RESTRICTIONS, TRIGGERS

COLORS = ['White', 'Blue', 'Black', 'Red', 'Green']
MANA_SYMBOLS = ['W', 'U', 'B', 'R', 'G', 'C']
CARD_TYPES = ['Creature', 'Instant', 'Sorcery', 'Artifact', 'Enchantment', 'Land', 'Planeswalker', 'Battle',
              'Kindred', 'Dungeon']

LISTS = {
    'colors': COLORS, 'color_identity': COLORS,
    'mana_symbols_possible': MANA_SYMBOLS, 'mana_symbols_explicit': MANA_SYMBOLS,
    'card_types': CARD_TYPES,
    'ability_kinds': KINDS, 'ability_costs': COSTS, 'costs': COSTS, 'triggers': TRIGGERS, 'effects': EFFECTS,
    'restrictions': RESTRICTIONS, 'ability_restrictions': RESTRICTIONS,
}
EXAMPLES = {'card_types': [['Instant']], 'printed_mana_value': [2], 'costs': [['tap']]}


def declare(properties):
    """Add enums and examples to the schema properties present, in place."""
    for name, values in LISTS.items():
        if name in properties:
            properties[name] = {'type': 'array', 'items': {'type': 'string', 'enum': values}}
    if 'kind' in properties:
        properties['kind'] = {'type': 'string', 'enum': KINDS}
    for name, examples in EXAMPLES.items():
        if name in properties:
            properties[name]['examples'] = examples
    return properties
