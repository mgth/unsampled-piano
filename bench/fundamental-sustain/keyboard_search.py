"""Explore passive polarisation coupling on every alerted key, all velocities.

Use the actual experimental renderer and preserve existing per-key models.
Candidates are selected only after checking every velocity and H2–H8.
"""
import keyboard_common as c
import keyboard_diagnose as d
from concurrent.futures import ThreadPoolExecutor,as_completed
import math,json
import numpy as np
PAIRS=[(.01,.5),(.01,.999),(.03,.5),(.03,.85),(.1,.7),(.1,.999),(.3,.7),(.3,.999),(.5,.7),(.5,.85),(.7,.7),(.7,.85)]

def main():
 diagnosis=json.loads((c.O/'diagnosis.json').read_text())
 notes=[int(n) for n,a in diagnosis['notes'].items() if a['alert_velocities']]
 def job(pair):
  h,f=pair;name=f'h{h}_f{f}';out=c.O/name;metrics=out/'metrics.json'
  fields={'polarization.h_coupling':h,'polarization.cross':math.sqrt(h)*f}
  path=c.bundle(c.O/f'bundle_{name}',{n:fields for n in notes})
  c.render(path,out,notes,hold=1.4,tail=.2)
  if metrics.exists():return name,json.loads(metrics.read_text())
  rows={f'{n}|{v}':c.measure(out/f'n{n}v{v}.wav',n) for n in notes for v in c.VELOCITIES}
  c.save(metrics,rows);return name,rows
 with ThreadPoolExecutor(3) as pool:
  futures=[pool.submit(job,pair) for pair in PAIRS]
  results={}
  for future in as_completed(futures):
   name,rows=future.result();results[name]=rows;print('measured',name,flush=True)
 c.save(c.O/'search.json',dict(notes=notes,pairs=PAIRS,results=results))
 print('search complete',len(notes)*3*len(PAIRS),flush=True)

if __name__=='__main__':main()
