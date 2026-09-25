#!/usr/bin/env python3
"""Fetch typed UUID/kind/payload records over HTTP, preserving a consistent snapshot."""
import argparse
import json
from pathlib import Path
from urllib.parse import urlencode
from urllib.request import urlopen

def fetch(url):
    with urlopen(url, timeout=60) as response:
        return json.load(response)

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--url', default='http://127.0.0.1:8731')
    p.add_argument('--dataset', choices=['collection','cards','decks','mechanics'], default='collection')
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    base = args.url.rstrip('/')
    snapshot = fetch(base+'/v1/status')['snapshot_id']
    pending = args.output.with_suffix(args.output.suffix+'.tmp')
    offset, count = 0, 0
    with pending.open('w') as f:
        while offset is not None:
            page = fetch(base+'/v1/records?'+urlencode(dict(dataset=args.dataset,limit=500,offset=offset,snapshot_id=snapshot)))
            if page['snapshot_id'] != snapshot: raise RuntimeError('Snapshot changed')
            for document in page['items']:
                f.write(json.dumps(document,ensure_ascii=False)+'\n')
                count += 1
            offset = page['next_offset']
        if count != page['total']: raise RuntimeError('Incomplete export')
    pending.replace(args.output)
    print(f'{count} records typés, snapshot {snapshot} → {args.output}')

if __name__ == '__main__': main()
