#!/usr/bin/env python3
"""Export a local draft or captured Arena deck to Arena clipboard text."""
import argparse
import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[1] / 'data'


def arena_clipboard(deck, catalog=None):
    if 'main_deck' in deck:
        sections = [('Deck', deck['main_deck']['nonlands'] + deck['main_deck']['lands']),
                    ('Sideboard', deck.get('sideboard', []))]
    else:
        labels = {'MainDeck': 'Deck', 'Sideboard': 'Sideboard', 'CommandZone': 'Commander',
                  'Companion': 'Companion'}
        unknown = [key for key, rows in deck['piles'].items() if rows and key not in labels]
        if unknown:
            raise ValueError(f'Unsupported nonempty deck piles: {unknown}')
        sections = []
        for key in ['CommandZone', 'Companion', 'MainDeck', 'Sideboard']:
            rows = []
            for entry in deck['piles'].get(key, []):
                card = catalog[entry['cardId']]
                rows.append({**card, 'quantity': entry['quantity']})
            sections.append((labels[key], rows))
    blocks = []
    for label, rows in sections:
        if not rows and label != 'Deck':
            continue
        lines = [label]
        for row in rows:
            quantity = row['quantity']
            if type(quantity) is not int or quantity <= 0:
                raise ValueError('Card quantities must be positive integers')
            fields = [str(row[key]).strip() for key in ('name_en', 'set', 'collector_number')]
            if any(not field or '\n' in field or '\r' in field for field in fields):
                raise ValueError('Missing or multiline card name/set/collector number')
            name, set_code, number = fields
            lines.append(f'{quantity} {name} ({set_code.upper()}) {number}')
        blocks.append('\n'.join(lines))
    return '\n\n'.join(blocks) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument('--draft', type=Path, help='Draft JSON with main_deck.lands/nonlands')
    source.add_argument('--deck-id', help='Captured Arena deck UUID from data/decks.json')
    parser.add_argument('--output', type=Path, help='Output text file; otherwise stdout')
    args = parser.parse_args()
    if args.draft:
        deck = json.loads(args.draft.read_text(encoding='utf-8'))
        if deck.get('status', '').startswith('superseded'):
            parser.error('This draft is superseded; select a current draft')
        result = arena_clipboard(deck)
    else:
        decks = json.loads((DATA / 'decks.json').read_text(encoding='utf-8'))
        deck = next((d for d in decks if d['id'] == args.deck_id), None)
        if deck is None:
            parser.error('Deck ID not found in local snapshot')
        with (DATA / 'cards.jsonl').open(encoding='utf-8') as stream:
            catalog = {c['arena_id']: c for c in map(json.loads, stream)}
        result = arena_clipboard(deck, catalog)
    if args.output:
        args.output.write_text(result, encoding='utf-8')
    else:
        print(result, end='')


if __name__ == '__main__':
    main()
