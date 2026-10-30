import sys,json,re,subprocess,itertools,hashlib
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import numpy as np
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools'))
import contact_voicing as c
OUT=ROOT/'out/contact-residuals'; OUT.mkdir(parents=True,exist_ok=True); BASE=c.BASE_PRESET.read_text()
PROFILES=json.load(open(ROOT/'bench/contact-voicing/profiles.json'));GUARD=PROFILES['guarded']
PARAMS=json.load(open(ROOT/'bench/contact-voicing/note-parameters.json'))
REFS=json.load(open(ROOT/'bench/contact-voicing/references.json'))
MODELS=json.load(open(ROOT/'bench/contact-voicing/validation-models.json'))
def preset(path,spec,extra={}):
 c.write_preset(path,BASE,**spec)
 t=path.read_text()
 for key,v in extra.items():
  t,n=re.subn(rf'^{key} = .*$',f'{key} = {v}',t,flags=re.M)
  if n!=1:raise ValueError((key,n))
 path.write_text(t)
def render(name,n,spec,extra={},vels=[45,56,100],hold=.2):
 d=OUT/'grid'/name;d.mkdir(parents=True,exist_ok=True);p=d/'preset.toml';preset(p,spec,extra)
 c.run([c.BATCH,p,d,n,','.join(map(str,vels)),hold,.05])
 a={f'{n}:{v}':c.measure_model(d/f'n{n}v{v}.wav',PARAMS[str(n)]) for v in vels}
 c.save(d/'metrics.json',a);return a

def screen():
 grid={}
 for n in [97,99]:
  for i,(q,p,h,d) in enumerate(itertools.product([.25,.5,.8,1.2],[-1,0,1],[0,.2,.4,.6],[.000025,.0001])):
   grid[f'n{n}a{i:03}']=dict(note=n,params=dict(q=q,p=p,h=h,d=d,w=1))
 for i,(q,p,h,d) in enumerate(itertools.product([.025,.05,.1,.2,.4,.8],[-2,-1,0,1,2],[0,.2,.6],[0,.000025,.000075])):
  grid[f'n102a{i:03}']=dict(note=102,params=dict(q=q,p=p,h=h,d=d,w=1))
 c.save(OUT/'grid.json',grid)
 def one(job):
  name,e=job;a=render(name,e['note'],e['params']);return name,a
 with ThreadPoolExecutor(6) as pool:
  results={}
  for name,a in pool.map(one,grid.items()):
   results[name]=a
   if len(results)%30==0:print(len(results),'/',len(grid),flush=True)
 c.save(OUT/'grid-results.json',results)

def ablate():
 grid={}
 for n in [97,99,102]:
  s=c.interpolate(GUARD,n)
  grid[f'n{n}-guard']=dict(note=n,params=s,extra={})
  for k,values in dict(shank_share=[0],core_hardness=[0],strike_ratio=[.09,.105,.135,.15,.18],contact_width_scale=[.5,2,4],height_spread=[0,.00005,.0001]).items():
   for v in values:
    sp=s.copy();extra={}
    if k=='contact_width_scale':sp['w']=v
    elif k=='height_spread':sp['d']=v
    else:extra[k]=v
    grid[f'n{n}-{k}-{v}']=dict(note=n,params=sp,extra=extra)
 c.save(OUT/'ablation-grid.json',grid)
 def one(job):
  name,e=job;a=render(name,e['note'],e['params'],e['extra']);return name,a
 with ThreadPoolExecutor(6) as pool:c.save(OUT/'ablation-results.json',dict(pool.map(one,grid.items())))
 for n in [97,99,102]:
  for label,s in [('base',c.DEFAULT),('guard',c.interpolate(GUARD,n))]:
   p=OUT/f'{n}-{label}.toml';preset(p,s)
   for v in [45,56,100]:
    result=c.run([c.PROBE,p,n,v,'contact','unused']);(OUT/f'{n}-{v}-{label}-forces.csv').write_text(result.stdout);(OUT/f'{n}-{v}-{label}-contact.txt').write_text(result.stderr)
    c.run([c.PROBE,p,n,v,'force',OUT/f'{n}-{v}-{label}-bridge.wav',.2,.05])
if __name__=='__main__':ablate() if sys.argv[-1]=='ablate' else screen()
