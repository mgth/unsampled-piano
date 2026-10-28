"""Separate mechanical hypotheses: contact width, shank loss and effective mass.
These are isolated-note experiments only, not a new global calibration.
"""
import sys,itertools
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0,str(Path(__file__).parent));import search as s
s.GRID={}
for share,freq,loss in itertools.product([.15,.3,.45],[150,500,1000],[.1,.5,2.]):s.add(dict(q=.2,h=.1),{'strike_ratio':.105,'shank_share':share,'shank_freq':freq,'shank_loss':loss})
for w,q,h in itertools.product([.5,1.5,2.],[.12,.2,.32],[.05,.15]):s.add(dict(q=q,h=h,w=w),{'strike_ratio':.105})
for mass,q in itertools.product([.6,.8,1.2,1.5],[.12,.2,.32]):s.add(dict(q=q,h=.1),{'strike_ratio':.105,'hammer_mass_scale':mass})
s.GRID={f'm{i:03}':e for i,e in enumerate(s.GRID.values())}
s.c.save(s.P/'mechanics-design.json',dict(grid=s.GRID,velocities=s.VEL,refit=True))
with ThreadPoolExecutor(6) as ex:r=list(ex.map(s.one,s.GRID.items()))
s.c.save(s.P/'mechanics-results.json',r)
for x in sorted(r,key=lambda x:x['mae']+max(0,x['dips'][0]-5.6)*.6+max(0,x['dips'][1]-4.9)*.6)[:12]:print({k:v for k,v in x.items() if k not in ['metrics','sources']},flush=True)
