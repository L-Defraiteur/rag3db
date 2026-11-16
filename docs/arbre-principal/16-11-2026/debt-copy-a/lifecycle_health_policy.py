"""Cycle policy for the shared health transport; no HTTP and no grant creation."""
from pathlib import Path
import hashlib,json,os,time
import lifecycle_protocol as protocol
import lifecycle_grant_producer as producer
import lifecycle_provider_observation as owner


def canonical_sha(value):
 return hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':'),allow_nan=False).encode()).hexdigest()


def worker_gate(context):
 """Before contact and after observation: keep admission distinct from its lease."""
 if context.get('address')!='127.0.0.1:7878' or type(context.get('admission')) is not bool:
  raise ValueError('cycle health address/admission policy invalid')
 grant=context['grant'];pins=grant['owner_pins']
 if canonical_sha(pins)!=grant['owner_pins_sha256']:raise ValueError('owner pins canonical digest differs')
 owner._pins(pins)
 if context.get('expected_process')!=pins['expected_process']:raise ValueError('worker process pin differs from observed owner pins')
 ready=producer._ready(Path(context['ready_path']),context['ready_sha256'],context['nonce'])
 if ready.get('no_embedding_calls_authorized') is not True or ready.get('expected_embedding_calculations')!=0 or type(ready.get('expected_embedding_calculations')) is not int:
  raise ValueError('cycle health cannot authorize embedding')
 if pins['pilot']!=ready['phase']['pilot'] or pins['allocation']!=ready['phase']['allocation']:
  raise ValueError('owner pins differ from admitted cycle')
 cfg=protocol.decode((Path(__file__).parent/'prepared.json').read_bytes())
 if cfg['pilot']!=pins['pilot']:raise ValueError('prepared cycle differs from owner pins')
 for name,expected in cfg['files'].items():
  if Path(name).name!=name or owner.digest(Path(__file__).parent/name)!=expected:
   raise ValueError('prepared health source differs '+name)
 for key,name in [('protocol_sha256','lifecycle_protocol.py'),('producer_sha256','lifecycle_grant_producer.py'),('observation_sha256','lifecycle_provider_observation.py'),('transport_sha256','provider_health_transport.py')]:
  if pins[key]!=cfg['files'][name]:raise ValueError('owner code pins differ from prepared cycle')
 if grant['identity']!=pins['identity'] or grant['pid']!=pins['expected_process']['pid'] or grant['start']!=pins['expected_process']['start'] or grant['exe_sha256']!=pins['expected_process']['exe_sha256']:
  raise ValueError('grant differs from observed owner identity')
 phase=protocol.read_phase_state(ready);now=time.monotonic()
 validate=protocol.validate_grant_phase if context['admission'] else protocol.validate_admitted_grant_phase
 validate(ready,grant,now,phase,context['ready_sha256'])
 deadline=protocol._clock(context['contact_deadline'],'health contact deadline')
 bound=min(ready['deadline'],grant['valid_until_monotonic'])
 if context['admission']:bound=min(bound,ready['grant_deadline_monotonic'])
 if not now<deadline<=bound:raise ValueError('cycle health effective deadline expired or extended')
 owner._validate_process(owner.process_snapshot(pins['expected_process']['pid']),pins['expected_process'])
 # The last phase observation follows process inspection, before the shared GET.
 phase=protocol.read_phase_state(ready);now=time.monotonic()
 validate(ready,grant,now,phase,context['ready_sha256'])
 if now>=deadline:raise ValueError('cycle health deadline after process observation')
