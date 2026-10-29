"""Refine E4 after a fixed coarse search; keep the half-second guard."""
import study as s
from concurrent.futures import ThreadPoolExecutor
import json,math
s.notes=[64];s.vel=[45,56,100]
variants={f'plateau_ref_h{h}_x{x}':{'polarization.h_coupling':h,'polarization.cross':math.sqrt(h)*x} for h,x in [(.6,.78),(.65,.78),(.65,.75),(.7,.75),(.7,.78),(.75,.75),(.8,.75),(.8,.7)]}
with ThreadPoolExecutor(4) as pool:results=dict(pool.map(s.job,variants.items()))
(s.O/'plateau_refine.json').write_text(json.dumps(dict(variants=variants,notes=s.notes,velocities=s.vel,results=results),indent=1))
for name,rows in results.items():
 print(name,[(v,round(rows[f'64|{v}']['audio']['decay_self'][3][0],2),round(rows[f'64|{v}']['audio']['decay_self'][6][0],2)) for v in s.vel])
