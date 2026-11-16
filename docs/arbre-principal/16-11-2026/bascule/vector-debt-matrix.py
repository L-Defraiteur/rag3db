"""Classify every indebted chunk against all linked occurrence candidates, without vector/text values."""
LEVELS=['field','rank','hash','exact','valid','text']
def identifier(value):
 if not isinstance(value,str) or not value or value=='NULL' or any(c in value for c in '|\t\r\n'):raise ValueError('invalid identifier protocol')
 return value

def build_matrix(debt_ids,universe,candidates,expected_count):
 if len(debt_ids)!=expected_count or len(set(debt_ids))!=len(debt_ids):raise ValueError('debt cohort size/duplicates')
 records={identifier(n):dict(chunk_uuid=n,content_uuid=None,selected_scope=None,occurrences=[],candidates={level:{} for level in LEVELS}) for n in debt_ids}
 for n,v,s in universe:
  identifier(n);identifier(v);identifier(s)
  if n not in records:raise ValueError('universe contains a non-debt chunk')
  row=records[n]
  if row['content_uuid'] is not None and row['content_uuid']!=v:raise ValueError('chunk has multiple content parents')
  if s in row['occurrences']:raise ValueError('duplicate occurrence link in cohort')
  row['content_uuid']=v;row['occurrences'].append(s)
 for level in LEVELS:
  for n,s,count in candidates[level]:
   identifier(n);identifier(s)
   if n not in records or s not in records[n]['occurrences'] or type(count) is not int or count<=0:raise ValueError('candidate outside its occurrence cohort')
   matches=records[n]['candidates'][level]
   if s in matches:raise ValueError('duplicate candidate observation')
   matches[s]=count
 categories={};unknown=0;text_mismatch=0
 for row in records.values():
  owners=row['occurrences'];matches=row['candidates']
  if not owners:
   row.update(category='UNKNOWN_NO_PARENT_OCCURRENCE',reason='missing content/occurrence chain');unknown+=1
  else:
   row['selected_scope']=min(owners);row['occurrences']=sorted(owners)
   for earlier,later in zip(LEVELS,LEVELS[1:]):
    if not set(matches[later]).issubset(matches[earlier]) or any(count>matches[earlier][owner] for owner,count in matches[later].items()):raise ValueError('non-monotone candidate levels')
   valid=matches['valid'];rep=row['selected_scope']
   if rep in valid:row.update(category='COMPATIBLE_AT_SELECTED_REPRESENTATIVE',reason='valid production-key candidate was not transferred')
   elif valid:row.update(category='COMPATIBLE_ONLY_AT_OTHER_OCCURRENCE',reason='selected representative has no valid exact-key vector')
   else:
    if not matches['field']:reason='no_old_parent_field'
    elif not matches['rank']:reason='no_old_rank'
    elif not matches['hash']:reason='different_text_hash_or_new_split'
    elif not matches['exact']:reason='different_title'
    else:reason='old_vector_null_or_stale'
    row.update(category='NO_COMPATIBLE_CANDIDATE',reason=reason)
   row['literal_text_mismatch']=any(matches['text'].get(owner,0)!=count for owner,count in valid.items())
   if row['literal_text_mismatch']:text_mismatch+=1
  categories[row['category']]=categories.get(row['category'],0)+1
 matrix=[records[n] for n in sorted(records)]
 return dict(matrix=matrix,audit=dict(count=len(matrix),duplicates=0,unknown=unknown,categories=categories,literal_text_mismatch=text_mismatch,complete=len(matrix)==expected_count and unknown==0),predicate='normalized field/index/text_hash/title + persisted model nonnull/fresh marker; literal text audited separately')
