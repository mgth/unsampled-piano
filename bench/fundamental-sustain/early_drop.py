"""Screen the first half-second separately; do not alter the running app."""
import study as s
from concurrent.futures import ThreadPoolExecutor
import json,math
s.notes=[60,61,62,63,64,66,69];s.vel=[45,100]
base={'polarization.h_coupling':.1,'polarization.cross':.316}
variants={
 'early_h01':base,
 'early_h02':{'polarization.h_coupling':.2,'polarization.cross':math.sqrt(.2)*.999},
 'early_h03':{'polarization.h_coupling':.3,'polarization.cross':math.sqrt(.3)*.999},
 'early_board_loss04':{**base,'soundboard.loss_factor':.04},
 'early_bridge_mass01':{**base,'soundboard.bridge_mass':.1},
 'early_bridge_mass03':{**base,'soundboard.bridge_mass':.3},
 'early_bridge_spread05':{**base,'soundboard.bridge_spread':.5},
}
with ThreadPoolExecutor(4) as pool:results=dict(pool.map(s.job,variants.items()))
(s.O/'early_drop.json').write_text(json.dumps(dict(variants=variants,notes=s.notes,velocities=s.vel,results=results),indent=1))
for name,rows in results.items():
 print(name,'H1 0.4–0.6 s at v100:',[round(rows[f'{n}|100']['audio']['decay_self'][3][0],1) for n in s.notes])
