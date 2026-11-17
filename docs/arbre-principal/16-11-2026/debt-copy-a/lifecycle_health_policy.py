"""Cycle policy for the shared health transport; no HTTP and no grant creation."""
from pathlib import Path
import hashlib,json,os,subprocess,time
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


def _watch_inputs(config):
 """Only frozen file/HEAD/profile checks; no service discovery or contact."""
 prepared_path=Path(config['prepared_path']);q=prepared_path.parent
 if owner.digest(prepared_path)!=config['prepared_sha256']:raise ValueError('watcher prepared SHA differs')
 prepared=protocol.decode(prepared_path.read_bytes())
 if prepared['pilot']!=config['pilot']:raise ValueError('watcher source differs')
 if subprocess.check_output(['git','-C',prepared['source_root'],'rev-parse','HEAD'],text=True).strip()!=config['pilot'] or subprocess.check_output(['git','-C',prepared['source_root'],'status','--porcelain'],text=True):raise ValueError('watcher source HEAD not exact/clean')
 for name,expected in prepared['files'].items():
  if Path(name).name!=name or owner.digest(q/name)!=expected:raise ValueError('watcher source copy changed '+name)
 if protocol.decode((q/'allocation.json').read_bytes())['reference']!=config['allocation']:raise ValueError('watcher allocation differs')
 context=config['policy_context'];profile_path=Path(context['expected_provider_path'])
 if profile_path!=q/'lifecycle-provider-expectations.json' or owner.digest(profile_path)!=context['expected_provider_sha256'] or context['expected_provider_sha256']!=prepared['files'][profile_path.name]:raise ValueError('watcher expected profile not prepared/pinned')
 profile=protocol.decode(profile_path.read_bytes());origin=profile['provenance'];origin_path=Path(origin['path'])
 if not origin_path.is_absolute() or '.env' in origin_path.parts or '.vault' in origin_path.parts or origin_path.is_symlink() or owner.digest(origin_path)!=origin['sha256']:raise ValueError('expected fingerprint provenance changed')
 if profile.get('kind')!='EXPECTED_PROVIDER_ONLY_NOT_FRESH_HEALTH':raise ValueError('expected profile must not claim a fresh observation')
 pins=dict(protocol_sha256=prepared['files']['lifecycle_protocol.py'],producer_sha256=prepared['files']['lifecycle_grant_producer.py'],observation_sha256=prepared['files']['lifecycle_provider_observation.py'],transport_sha256=prepared['files']['provider_health_transport.py'],pilot=config['pilot'],allocation=config['allocation'],c7_reference=config['allocation'],embar_reference='qualified-watcher/'+config['actor_nonce'],expected_process=profile['expected_process'],identity=profile['identity'])
 owner._pins(pins)
 return pins


def preflight(config):
 _watch_inputs(config)
 return None


def publish_ready(config,ready_path,ready_sha256,nonce):
 if str(ready_path)!=config['paths']['ready'] or nonce!=config['ready_nonce']:raise ValueError('watcher READY path/nonce differs')
 pins=_watch_inputs(config)
 if config['paths']['grant']!=str(Path(config['prepared_path']).parent/'PROVIDER_GO.json'):raise ValueError('watcher grant destination is not its prepared control root')
 ready=producer._ready(Path(ready_path),ready_sha256,nonce)
 if ready.get('expected_embedding_calculations')!=0 or type(ready.get('expected_embedding_calculations')) is not int:raise ValueError('watcher must not calculate embeddings')
 if ready['phase']['pilot']!=config['pilot'] or ready['phase']['allocation']!=config['allocation']:raise ValueError('watcher READY job differs')
 return owner.publish_observed_grant(ready_path,config['paths']['grant'],expected_ready_sha256=ready_sha256,expected_nonce=nonce,pins=pins,evidence_path=config['paths']['evidence'])
