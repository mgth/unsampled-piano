import study as s
from concurrent.futures import ThreadPoolExecutor
import json
variants={
'unison3':{'unison.width_mid':3},'unison4':{'unison.width_mid':4},
'radiation05':{'polarization.radiation':.5},
'unison2_rad03':{'unison.width_mid':2,'polarization.radiation':.3},
'unison3_rad03':{'unison.width_mid':3,'polarization.radiation':.3},
'unison2_rad05':{'unison.width_mid':2,'polarization.radiation':.5},
'pol_h01':{'polarization.h_coupling':.1,'polarization.cross':.316},
'pol_detune1':{'polarization.detune_cents':1},
}
with ThreadPoolExecutor(4) as pool:results=dict(pool.map(s.job,variants.items()))
(s.O/'refine.json').write_text(json.dumps(dict(variants=variants,results=results),indent=1))
for name,rows in results.items():print(name,[round(rows[f'{n}|100']['audio']['decay_self'][6][0],1) for n in s.notes])
