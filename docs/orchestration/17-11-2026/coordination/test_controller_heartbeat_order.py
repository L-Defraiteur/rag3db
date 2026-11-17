"""Actual supervisor; heartbeat publication interleaved with its read."""
from pathlib import Path
from unittest.mock import patch
import json,unittest
import grant_watch_controller as ctrl
from test_grant_watch_controller import FakeWorld

class ControllerHeartbeatOrder(unittest.TestCase):
 def test_healthy_heartbeat_published_after_scope_sample_is_not_future(self):
  world=FakeWorld('valid');self.addCleanup(world.tmp.cleanup)
  read=ctrl.read_owned;injected=[]
  def interleaved(path):
   if Path(path)==world.path('heartbeat') and world.driver is not None and not injected:
    before=world.now;world.now+=.01
    heartbeat=json.loads(world.path('heartbeat').read_text())
    heartbeat.update(observed_monotonic=world.now,loop_counter=2)
    world.write('heartbeat',heartbeat);injected.append((before,world.now))
   return read(path)
  with patch.object(ctrl,'read_owned',side_effect=interleaved):rc=world.run()
  self.assertEqual(len(injected),1,'must exercise the driver-loop interleaving')
  self.assertEqual(rc,0)
  self.assertFalse(world.path('first_red').exists())
  self.assertTrue(world.path('release').exists())

if __name__=='__main__':unittest.main()
