"""Pure catalog projection and conservative wildcard planning; no database access."""
import re
import unicodedata
from collections import defaultdict

RARITIES = {0: 'unknown', 1: 'basic', 2: 'common', 3: 'uncommon', 4: 'rare', 5: 'mythic'}
WILDCARD_RARITIES = ('common', 'uncommon', 'rare', 'mythic')


def normalized_name(name):
    return ' '.join(unicodedata.normalize('NFKC', name).casefold().split())


def ownership_model(cards):
    """Faces/rebalanced versions share a printing entitlement: take max, not sum.

    Different printing families with the same canonical front name are additive.
    This intentionally does not fuzzy-merge mechanically different card names.
    """
    by_id = {c['arena_id']: c for c in cards}
    parent = {i: i for i in by_id}

    def find(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i

    for c in cards:
        for linked in [*c.get('linked_faces', []), c.get('rebalanced_arena_id')]:
            if linked in by_id:
                a, b = find(c['arena_id']), find(linked)
                parent[max(a, b)] = min(a, b)
    # Some Arena snapshots omit the rebalance link entirely. An exact printing
    # identity (name + set + collector number), with one unambiguous original
    # front, still identifies a shared entitlement; never merge just by name.
    originals = defaultdict(list)
    def printing_key(c):
        if not c.get('set') or not c.get('collector_number'):
            return None
        return (normalized_name(c['name_en']), c['set'].casefold(), str(c['collector_number']))
    for c in cards:
        if c['is_primary'] and not c['is_rebalanced'] and not c['is_token']:
            key = printing_key(c)
            if key is not None:
                originals[key].append(c['arena_id'])
    for c in cards:
        matches = originals.get(printing_key(c), [])
        if c['is_rebalanced'] and not c['is_token'] and len(matches) == 1:
            a, b = find(c['arena_id']), find(matches[0])
            parent[max(a, b)] = min(a, b)
    families = defaultdict(list)
    for c in cards:
        families[find(c['arena_id'])].append(c)
    info, totals = {}, defaultdict(int)
    for family, members in families.items():
        front = min(members, key=lambda c: (not c['is_primary'], c['is_rebalanced'], c['arena_id']))
        name = normalized_name(front['name_en'])
        quantity = max(c['owned'] for c in members)
        if not front['is_token']:
            totals[name] += quantity
        for c in members:
            info[c['arena_id']] = {'printing_family_id': family, 'canonical_name': front['name_en'],
                                  'name_normalized': name, 'owned_printing_family': quantity}
    for row in info.values():
        row['owned_name_total'] = totals[row['name_normalized']]
    return info


def printed_mana_value(cost):
    symbols = re.findall(r'\{([^}]+)\}', cost)
    total = 0
    for symbol in symbols:
        if symbol.isdigit():
            total += int(symbol)
        elif symbol in {'X', 'Y', 'Z'}:
            pass
        elif all(p in {'W', 'U', 'B', 'R', 'G', 'C', 'S', 'P', '2'} for p in symbol.split('/')):
            total += 2 if '2' in symbol.split('/') else 1
        else:
            return -1
    return total


def wildcard_plan(requests, catalog_rows, inventory):
    """Plan desired total quantities by canonical name, never execute a craft.

    Candidate printing is the lowest-rarity front in the local catalog. Availability
    and format legality still require Arena validation; unknown names fail closed.
    """
    desired = defaultdict(int)
    for request in requests:
        if set(request) != {'name', 'quantity'} or not isinstance(request['name'], str):
            raise ValueError('Each request must contain name and quantity')
        q = request['quantity']
        if type(q) is not int or q < 0 or q > 250:
            raise ValueError('quantity must be an integer between 0 and 250')
        desired[normalized_name(request['name'])] += q
    result, costs, unresolved = [], dict.fromkeys(WILDCARD_RARITIES, 0), []
    for name, q in desired.items():
        candidates = [r for r in catalog_rows if r['name_normalized'] == name]
        if not candidates:
            unresolved.append({'name': name, 'reason': 'not_found'})
            continue
        owned = max(r['owned_name_total'] for r in candidates)
        basic = any(r['is_basic_land'] for r in candidates)
        missing = 0 if basic and owned > 0 else max(0, q - owned)
        fronts = [r for r in candidates if r['is_craft_candidate']]
        chosen = min(fronts, key=lambda r: (r['rarity_code'], -r['owned_printing_family'], r['arena_id'])) if fronts else None
        if missing and not chosen:
            unresolved.append({'name': name, 'reason': 'no_craft_candidate', 'missing': missing})
            continue
        if missing:
            costs[chosen['rarity']] += missing
        result.append({'name': candidates[0]['canonical_name'], 'desired_total': q,
                       'owned_across_printings': owned, 'missing': missing,
                       'rarity': chosen['rarity'] if chosen else 'basic',
                       'craft_arena_id': chosen['arena_id'] if chosen and missing else None,
                       'status': 'proposed_requires_arena_validation' if missing else 'already_owned'})
    available = {r: inventory[r] for r in WILDCARD_RARITIES}
    return {'items': result, 'wildcards_required': costs, 'wildcards_available_in_snapshot': available,
            'wildcard_shortfall': {r: max(0, costs[r] - available[r]) for r in costs},
            'fits_snapshot_budget': not unresolved and all(costs[r] <= available[r] for r in costs),
            'unresolved': unresolved, 'snapshot_id': inventory['snapshot_id'],
            'captured_at': inventory['captured_at'], 'crafts_executed': False,
            'limitations': ['Snapshot quantities may be stale; no purchase is assumed.',
                           'Craft availability and format legality are not inferred from rarity.',
                           'Quantities are desired totals, not additions; repeated names are summed.']}
