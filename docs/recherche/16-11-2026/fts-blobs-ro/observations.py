"""Strict observation parsing, no inference of execution history or repair."""
import re

def native_queries(text,count):
    result=[];current=None
    for line in text.splitlines():
        if line.startswith('BEGIN_QUERY\t'):
            if current is not None or line!='BEGIN_QUERY\t'+str(len(result)):raise ValueError('query order differs')
            current=[]
        elif line.startswith('ROW\t') and current is not None:current.append(line[4:])
        elif line.startswith('END_QUERY\t') and current is not None:
            if line!='END_QUERY\t'+str(len(current)):raise ValueError('native row count differs')
            result.append(current);current=None
        else:raise ValueError('unexpected native output')
    if current is not None or len(result)!=count:raise ValueError('incomplete native queries')
    return result

def snapshot_ids(text):
    lines=text.splitlines()
    if not lines:raise ValueError('missing snapshot output')
    header=re.fullmatch(r'IMMUTABLE_SNAPSHOT\tshards=([1-9][0-9]*)\tdocuments=([0-9]+)\tbytes=([0-9]+)\tcommit=REFUSED',lines[0])
    if not header:raise ValueError('snapshot is not qualified read-only')
    ids=[]
    for line in lines[1:]:
        match=re.fullmatch(r'LIVE_ID\t(0|[1-9][0-9]*)',line)
        if not match or int(match[1])>=2**64:raise ValueError('invalid live ID')
        ids.append(int(match[1]))
    if len(ids)!=int(header[2]) or len(set(ids))!=len(ids) or ids!=sorted(ids):raise ValueError('snapshot count, uniqueness or ordering differs')
    return ids

def compare_ids(graph_rows,live_ids):
    graph=[]
    for row in graph_rows:
        if not re.fullmatch(r'0|[1-9][0-9]*',row) or int(row)>=2**63:raise ValueError('invalid physical offset')
        graph.append(int(row))
    if len(set(graph))!=len(graph) or len(set(live_ids))!=len(live_ids):raise ValueError('duplicate IDs')
    return dict(graph_count=len(graph),fts_count=len(live_ids),fts_without_row=sorted(set(live_ids)-set(graph)),rows_without_fts=sorted(set(graph)-set(live_ids)))
