import study as s
from concurrent.futures import ThreadPoolExecutor
import json
s.notes=[48,57,60,61,62,63,64,65,66,67,68,69,72,81,90,96,102,108]
s.vel=[45,56,100]
fields={'base':{},'unison4':{'unison.width_mid':4},'radiation05':{'polarization.radiation':.5},'pol_h01':{'polarization.h_coupling':.1,'polarization.cross':.316}}
variants={'validation_'+k:v for k,v in fields.items()}
with ThreadPoolExecutor(4) as pool:results=dict(pool.map(s.job,variants.items()))
(s.O/'validation.json').write_text(json.dumps(dict(variants=variants,notes=s.notes,velocities=s.vel,results=results),indent=1))
