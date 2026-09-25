#!/usr/bin/env python3
"""Replay recorded submissions through their unchanged backend and export accepted data.
Run only after benchmark_chat and its backend have exited. Never repairs a proposal.
"""
import argparse
import json
from pathlib import Path
import subprocess
from export_deck import arena_clipboard

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('run',type=Path)
p.add_argument('--snapshot',type=Path,default=Path(__file__).resolve().parents[1]/'data/engine-collection.jsonl')
a=p.parse_args();run=a.run.resolve()
report=json.loads((run/'evidence/report.json').read_text())
config=json.loads((run/'chat.json').read_text())
events=[json.loads(line) for line in (run/'evidence/events.jsonl').read_text().splitlines()]
submissions=[e for e in events if e.get('event')=='tool_start' and e.get('name')==config['completion_tool']]
source={c['arena_id']:c for c in map(json.loads,a.snapshot.open())}
results=[];last_accepted=None
with (run/'audit-host.log').open('w') as log:
    process=subprocess.Popen(config['backend_command'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=log,text=True)
    def request(payload):
        process.stdin.write(json.dumps(payload)+'\n');process.stdin.flush()
        reply=json.loads(process.stdout.readline())
        if not reply['ok']:raise RuntimeError(reply)
        return reply['result']
    try:
        for n,event in enumerate(submissions,1):
            arguments=json.loads(event['arguments']);deck=arguments['result']
            (run/'evidence'/f'submission-{n}.json').write_text(json.dumps(deck,ensure_ascii=False,indent=2)+'\n')
            receipt=request({'op':'call','name':config['completion_tool'],'arguments':arguments})
            (run/'evidence'/f'submission-{n}.audit.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n')
            rows=deck.get('mainboard',[])
            results.append({'attempt':n,'elapsed_seconds':event['elapsed_seconds'],'total':sum(c['quantity'] for c in rows),'lands':sum(c['quantity'] for c in rows if source.get(c['arena_id'],{}).get('is_land')),'validation':receipt.get('validation'),'delivery':receipt.get('delivery')})
            if receipt.get('validation',{}).get('accepted') and receipt.get('delivery',{}).get('ok'):
                row=receipt['result'];restored=request({'op':'call','name':'get_result','arguments':{'record':{'key':row['data']['key']}}})['result']
                assert restored['uuid']==row['uuid']
                assert json.loads(restored['data']['payload_json'])==deck
                last_accepted=(deck,restored)
        request({'op':'shutdown'})
    finally:
        process.stdin.close()
        assert process.wait(timeout=60)==0
        process.stdout.close()
(run/'submission-audit.json').write_text(json.dumps(results,ensure_ascii=False,indent=2)+'\n')
if report.get('answer',{}).get('result',{}).get('task_accepted') is True:
    assert last_accepted is not None
    deck,receipt=last_accepted
    rows=[{**source[e['arena_id']],'quantity':e['quantity']} for e in deck['mainboard']]
    prepared={'main_deck':{'lands':[r for r in rows if r['is_land']],'nonlands':[r for r in rows if not r['is_land']]},'sideboard':[]}
    text=arena_clipboard(prepared)
    assert sum(int(line.split()[0]) for line in text.splitlines() if line[:1].isdigit())==sum(c['quantity'] for c in deck['mainboard'])
    out=run/'exports';out.mkdir(exist_ok=True)
    (out/'deck.json').write_text(json.dumps(deck,ensure_ascii=False,indent=2)+'\n')
    (out/'deck.arena.txt').write_text(text)
    (out/'cards.json').write_text(json.dumps(rows,ensure_ascii=False,indent=2)+'\n')
    (out/'receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n')
    print('EXPORTED',out)
print(json.dumps([{k:r[k] for k in ['attempt','total','lands']}|{'accepted':r['validation']['accepted'],'errors':len(r['validation']['errors']),'warnings':len(r['validation']['warnings'])} for r in results],indent=2))
