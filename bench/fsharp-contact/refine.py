"""Refine only the mechanisms surviving the controlled ablations.
Acceptance relative to old local at F#7: no larger P2/P3 velocity dips,
no worse source-balanced attack MAE, lower forte P3 excess.
"""
import sys,itertools
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
sys.path.insert(0,str(Path(__file__).parent))
import search as s
s.GRID={}
for q,h,d in itertools.product([.12,.16,.2,.25,.32],[.04,.08,.12,.18],[.00007,.0001,.00013,.00016]):
 s.add(dict(q=q,h=h,d=d),{'strike_ratio':.105})
for p,ratio in itertools.product([-.5,0,.5],[.095,.1,.11,.115]):s.add(dict(q=.2,h=.1,p=p),{'strike_ratio':ratio})
for spread,exponent in itertools.product([0,.1,.3,.5],[0,1]):s.add(dict(q=.2,h=.1),{'strike_ratio':.105,'unison.hit_spread':spread,'unison.soft_exponent':exponent})
s.GRID={f'r{i:03}':e for i,e in enumerate(s.GRID.values())}
s.c.save(s.P/'refine-design.json',dict(grid=s.GRID,velocities=s.VEL,acceptance='compare to old local: MAE <=9.65, P2 dip <=5.6, P3 dip <=4.9, forte P3 excess <9.49 dB; validate every integer velocity after selection',refit=True))
with ThreadPoolExecutor(6) as ex:rows=list(ex.map(s.one,s.GRID.items()))
s.c.save(s.P/'refine-results.json',rows)
print('BEST',flush=True)
for row in sorted(rows,key=lambda x:x['mae']+max(0,x['dips'][0]-5.6)*.6+max(0,x['dips'][1]-4.9)*.6)[:15]:print({k:v for k,v in row.items() if k not in ['metrics','sources']},flush=True)
