"""Test the felt exponent after suppressing the shank/velocity dips.
A lower exponent is expected to reduce the pp-to-ff spectral growth;
force is still pivoted at 0.2 mm. No piecewise velocity compensation.
"""
import sys,itertools
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0,str(Path(__file__).parent));import search as s
s.GRID={}
for q,p,h in itertools.product([.08,.12,.18,.25],[-2,-1,0],[.05,.15,.3]):s.add(dict(q=q,p=p,h=h),{'strike_ratio':.105,'shank_share':.45,'shank_freq':150,'shank_loss':2.})
s.GRID={f'e{i:03}':e for i,e in enumerate(s.GRID.values())}
s.c.save(s.P/'damped-law-design.json',dict(grid=s.GRID,velocities=s.VEL,refit=True,hypothesis='lower exponent reduces pp-to-ff spectral growth after damping-induced smoothing'))
with ThreadPoolExecutor(6) as ex:r=list(ex.map(s.one,s.GRID.items()))
s.c.save(s.P/'damped-law-results.json',r)
for x in sorted(r,key=lambda x:x['mae']+max(0,x['dips'][0]-5.6)*.6+max(0,x['dips'][1]-4.9)*.6)[:8]:print({k:v for k,v in x.items() if k not in ['metrics','sources']},flush=True)
