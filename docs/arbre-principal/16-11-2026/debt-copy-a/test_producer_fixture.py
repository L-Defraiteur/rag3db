"""Synthetic declared process/loop observations for pure validators, never OS proof."""
from pathlib import Path
import copy,hashlib,json

def encoded(value):return (json.dumps(value,indent=2,allow_nan=False)+'\n').encode()
def digest(value):return hashlib.sha256(encoded(value)).hexdigest()

def producer_state(ready,now):
 phase=ready['phase'];authority=copy.deepcopy(phase['authority']);root=Path(phase['first_red_path']).parent;q=root/'Q';config_path=root/'watcher-config.json';arm_time=ready['ready_observed_monotonic']-1.
 actor=dict(script_path=str(q/'grant_watch_actor.py'),script_sha256='c'*64,exe='/fixture/python',exe_sha256='a'*64,argv_prefix=['/fixture/python','-B',str(q/'grant_watch_actor.py'),'--watch',str(config_path)])
 prepared=dict(pilot=phase['pilot'],files={'grant_watch_actor.py':'c'*64},source_root='/fixture/source')
 config=dict(clock=ready['clock'],pilot=phase['pilot'],allocation=phase['allocation'],prepared_path=str(q/'prepared.json'),prepared_sha256=digest(prepared),actor_nonce='a'*32,ready_nonce=ready['nonce'],issued_monotonic=arm_time-1.,arm_deadline_monotonic=arm_time+1.,common_end_monotonic=ready['deadline'],controller=dict(pid=654,start='65'),authority=authority,actor=actor,paths={key:str((q if key=='grant' else root)/name) for key,name in dict(ready='READY_FOR_PROVIDER_COORDINATION.json',grant='PROVIDER_GO.json',armed='watcher-armed.json',heartbeat='watcher-heartbeat.json',grant_ack='watcher-grant.json',result='watcher-result.json',release='watcher-release.json',first_red=Path(phase['first_red_path']).name,evidence='watcher-owner-evidence.json').items()},policy=dict(path=str(q/'policy.py'),sha256='b'*64),policy_context={})
 config_sha=digest(config)
 process=dict(pid=987,ppid=654,start='98',uid=authority['uid'],cgroup=authority['cgroup'],exe=actor['exe'],exe_sha256=actor['exe_sha256'],argv=actor['argv_prefix']+[config_sha],script_path=actor['script_path'],script_sha256=actor['script_sha256'],state='S',observed_monotonic=arm_time)
 receipt=dict(kind='ARMED',config_sha256=config_sha,actor_nonce=config['actor_nonce'],ready_nonce=config['ready_nonce'],pilot=config['pilot'],allocation=config['allocation'],common_end_monotonic=config['common_end_monotonic'],process=copy.deepcopy(process),loop_counter=1,loop_observed_monotonic=arm_time)
 receipt_sha=digest(receipt)
 admission=dict(kind='INDEPENDENT_PRODUCER_ADMISSION',config_path=str(config_path),config_sha256=config_sha,armed_path=config['paths']['armed'],armed_sha256=receipt_sha,expected_pid=987,process=copy.deepcopy(process),actor_nonce=config['actor_nonce'],ready_nonce=config['ready_nonce'],pilot=config['pilot'],allocation=config['allocation'],prepared_sha256=config['prepared_sha256'])
 admission_sha=digest(admission)
 actual=dict(process,observed_monotonic=now)
 heartbeat=dict(kind='HEARTBEAT',config_sha256=config_sha,actor_nonce=config['actor_nonce'],pid=987,start='98',loop_counter=2,phase='WAIT_READY',observed_monotonic=now)
 state=dict(admission=admission,admission_sha256=admission_sha,config=config,config_sha256=config_sha,receipt=receipt,receipt_sha256=receipt_sha,actual=actual,heartbeat=heartbeat,observed_monotonic=now)
 ready['phase']['producer_watcher']=dict(admission_path=str(root/'watcher-admission.json'),admission_sha256=admission_sha)
 return state
