"""Validate the exact native mapping before an inferior is allowed to continue."""
import os

def validate_sdk_maps(maps,ledger,library,digest,resolve=os.path.realpath):
 expected=library+'/librag3db.so'
 if resolve(library)!=library:raise ValueError('SDK root alias')
 paths=set()
 for line in maps.splitlines():
  fields=line.split(None,5)
  if len(fields)!=6:continue
  path=fields[5]
  if path.removesuffix(' (deleted)').endswith('/librag3db.so'):paths.add(path)
 if paths!={expected}:raise ValueError('mapped native SDK differs from the exact pinned path')
 if resolve(expected)!=expected:raise ValueError('mapped SDK file alias')
 actual=digest(expected)
 if actual!=ledger['sdk']['librag3db.so']:raise ValueError('mapped SDK bytes differ from ledger pin')
 return dict(path=expected,sha256=actual,validated_before_continue=True)
