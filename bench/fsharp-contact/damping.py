"""Final bounded cross: shape the spectrum after damping the shank-induced dips."""
import sys,itertools
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0,str(Path(__file__).parent));import search as s
s.GRID={}
for freq,loss in [(150,2.),(500,.5)]:
 for w,q,h in itertools.product([1.,1.5,2.],[.1,.16,.24],[.05,.15]):s.add(dict(q=q,h=h,w=w),{'strike_ratio':.105,'shank_share':.45,'shank_freq':freq,'shank_loss':loss})
s.GRID={f'd{i:03}':e for i,e in enumerate(s.GRID.values())}
s.c.save(s.P/'damping-design.json',dict(grid=s.GRID,velocities=s.VEL,refit=True))
with ThreadPoolExecutor(6) as ex:r=list(ex.map(s.one,s.GRID.items()))
s.c.save(s.P/'damping-results.json',r)
for x in sorted(r,key=lambda x:x['mae']+max(0,x['dips'][0]-5.6)*.6+max(0,x['dips'][1]-4.9)*.6)[:8]:print({k:v for k,v in x.items() if k not in ['metrics','sources']},flush=True)
