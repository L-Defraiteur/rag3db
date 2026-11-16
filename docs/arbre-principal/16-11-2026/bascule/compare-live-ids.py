#!/usr/bin/env python3
"""Strict complete ID observations; equal cardinality never substitutes for set equality."""
import json,re,sys
MAX_ID=(1<<64)-1

def unique_ids(values):
 if not isinstance(values,list) or any(type(v) is not int or not 0<=v<=MAX_ID for v in values):raise ValueError('invalid ID type/range')
 if len(values)!=len(set(values)):raise ValueError('duplicate IDs')
 return set(values)
def decimal(value):
 if not re.fullmatch(r'[0-9]+',value):raise ValueError('invalid decimal ID')
 result=int(value)
 if result>MAX_ID:raise ValueError('ID overflow')
 return result

def parse_fts(text):
 lines=text.splitlines()
 if not lines:raise ValueError('missing snapshot header')
 header=re.fullmatch(r'IMMUTABLE_SNAPSHOT\tshards=([0-9]+)\tdocuments=([0-9]+)\tbytes=([0-9]+)\tcommit=REFUSED',lines[0])
 if not header:raise ValueError('invalid snapshot header')
 ids=[]
 for line in lines[1:]:
  if not line.startswith('LIVE_ID\t'):raise ValueError('unexpected snapshot output')
  ids.append(decimal(line[len('LIVE_ID\t'):]))
 if len(ids)!=decimal(header[2]):raise ValueError('incomplete FTS observation')
 unique_ids(ids)
 return ids

def parse_graph(text):
 lines=text.splitlines()
 if len(lines)<2 or lines[0]!='BEGIN_QUERY\t0' or not lines[-1].startswith('END_QUERY\t'):raise ValueError('incomplete graph observation')
 ids=[]
 for line in lines[1:-1]:
  if not line.startswith('ROW\t'):raise ValueError('unexpected graph output')
  ids.append(decimal(line[len('ROW\t'):]))
 if len(ids)!=decimal(lines[-1][len('END_QUERY\t'):]):raise ValueError('graph row count mismatch')
 unique_ids(ids)
 return ids

def compare(fts,graph):
 f=unique_ids(fts);g=unique_ids(graph)
 return dict(fts_count=len(fts),graph_count=len(graph),orphans=sorted(f-g),missing=sorted(g-f),equal=f==g)
if __name__=='__main__':
 from pathlib import Path
 result=compare(parse_fts(Path(sys.argv[1]).read_text()),parse_graph(Path(sys.argv[2]).read_text()))
 print(json.dumps(result,indent=2));sys.exit(0 if result['equal'] else 1)
