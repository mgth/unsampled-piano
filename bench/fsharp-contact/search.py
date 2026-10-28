"""Bounded screen on F#7 only; no production preset or DSP edits.
Attack reference eligibility is fixed before evaluating any candidate.
"""
from pathlib import Path
import sys,json,itertools,shutil
from concurrent.futures import ThreadPoolExecutor
import numpy as np
R=Path(__file__).resolve().parents[2];sys.path.insert(0,str(R/'tools'));sys.path.insert(0,str(Path(__file__).parent))
import contact_voicing as c
from diagnose import patch
O=R/'out/fsharp-contact';P=R/'bench/fsharp-contact';BASE=(R/'bench/main-revalidation/effective.toml').read_text()
PAR=json.load(open(R/'bench/main-revalidation/note-parameters.json'))['102'];REF=json.load(open(R/'bench/main-revalidation/references.json'))
BATCH=O/'contact_batch';shutil.copy2(R/'out/main-revalidation/bin/contact_batch',BATCH)
VEL=sorted(set(range(28,128,4))|{45,56,100,127})
GRID={}
def add(s,extra=None):
 entry={'params':{**c.DEFAULT,**s},'extra':extra or {}}
 if entry not in GRID.values():GRID[f'g{len(GRID):03}']=entry
# Exponent/force pivot/hysteresis cross: the force scale is held at u=0.2 mm.
for q,p,h in itertools.product([.08,.15,.3],[-2,-1,0],[.1,.3,.6]):add(dict(q=q,p=p,h=h))
# Height and geometry ablations around the previous local contact.
for d in [0,.000025,.00005,.0001,.00015,.0002]:
 for p in [-1,0]:add(dict(q=.2,p=p,h=.3,d=d),{'strike_ratio':.105})
for ratio in [.085,.095,.105,.12,.14]:add(dict(q=.2,h=.3),{'strike_ratio':ratio})
for share in [.15,.45,.6]:add(dict(q=.2,h=.3),{'shank_share':share})
# Include defaults and old profiles as positive controls in exactly this run.
add({});old=json.load(open(R/'bench/contact-residuals/profile-local.json'))['notes']['102'];add(old['params'],old['extra'])
add(c.interpolate(json.load(open(R/'bench/contact-voicing/profiles.json'))['guarded'],102))

def one(job):
 name,e=job;d=O/'screen'/name;p=d/'preset.toml';c.write_preset(p,BASE,**e['params']);p.write_text(patch(p.read_text(),e['extra']))
 c.run([BATCH,p,d,102,','.join(map(str,VEL)),.2,.05])
 m={str(v):c.measure_model(d/f'n102v{v}.wav',PAR) for v in VEL};c.save(d/'metrics.json',m)
 per_source={};rowerrors=[];signed={};levels={}
 for source,ref in REF.items():
  errors=[]
  for v in [45,56,100]:
   key=f'102:{v}'
   if key not in ref:continue
   for i in [0,1]:
    target=ref[key]['partials'][i]
    if target is not None:errors.append(m[str(v)]['partials'][i]-target);rowerrors.append([source,v,i+2,m[str(v)]['partials'][i]-target])
  if errors:per_source[source]=np.mean(np.abs(errors))
 for v in [45,56,100]:
  signed[v]=[]
  for i in [0,1]:
   targets=[r[f'102:{v}']['partials'][i] for r in REF.values() if f'102:{v}' in r and r[f'102:{v}']['partials'][i] is not None]
   signed[v].append(m[str(v)]['partials'][i]-np.mean(targets))
 dips=[max(np.maximum.accumulate(a)-a) for a in [[m[str(v)]['partials'][i] for v in VEL] for i in [0,1]]]
 lv=[m[str(v)]['level'] for v in VEL]
 row=dict(name=name,mae=np.mean(list(per_source.values())),sources=per_source,rows=len(rowerrors),dips=dips,level_dip=max(np.maximum.accumulate(lv)-lv),signed=signed,params=e,metrics=m)
 print(name,round(row['mae'],2),[round(x,2) for x in dips],flush=True);return row
if __name__=='__main__':
 c.save(P/'screen-design.json',dict(grid=GRID,velocities=VEL,note=102,refit=True,cutoff=12000,reference_rows=34))
 with ThreadPoolExecutor(6) as ex:rows=list(ex.map(one,GRID.items()))
 c.save(P/'screen-results.json',rows)
 print('BEST',flush=True)
 for row in sorted(rows,key=lambda x:x['mae']+max(0,max(x['dips'])-5)*.6)[:12]:print({k:v for k,v in row.items() if k not in ['metrics','sources']},flush=True)
