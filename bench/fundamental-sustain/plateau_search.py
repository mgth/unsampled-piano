"""Reduce the H1 plateau with passive, less correlated polarisation coupling.

Screen on main; selected candidates must then be rendered by the prototype.
No temporal gain envelope or production/default change is introduced.
"""
import study as s
from concurrent.futures import ThreadPoolExecutor
import json,math
s.notes=[62,64,69];s.vel=[56,100]
variants={}
for h in [.3,.4,.5,.6]:
 for fraction in [.999,.95,.85,.7,.5]:
  variants[f'plateau_h{h}_x{fraction}']={'polarization.h_coupling':h,'polarization.cross':math.sqrt(h)*fraction}
with ThreadPoolExecutor(4) as pool:results=dict(pool.map(s.job,variants.items()))
(s.O/'plateau_screen.json').write_text(json.dumps(dict(variants=variants,notes=s.notes,velocities=s.vel,results=results),indent=1))
for n in s.notes:
 print('NOTE',n)
 for name,rows in results.items():
  a=rows[f'{n}|100']['audio'];print(name,'half',round(a['decay_self'][3][0],2),'late',round(a['decay_self'][6][0],2))
