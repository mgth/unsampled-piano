"""Global rear-radiation ablations, frozen contact and fixed reference support.
No production defaults. Control WAVs reused from the preceding validation.
"""
from pathlib import Path
import sys,json,hashlib
from concurrent.futures import ThreadPoolExecutor
import numpy as np
R=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(R/'tools'));sys.path.insert(0,str(Path(__file__).parent))
import contact_voicing as c
from preset_fields import patch
P=R/'bench/bridge-ear';O=R/'out/bridge-ear/validation';OLD=R/'out/fsharp-contact/validation'
PAR=json.load(open(R/'bench/main-revalidation/note-parameters.json'));REF=json.load(open(R/'bench/main-revalidation/references.json'))
NAMES=['balanced','back30','back0'];VEL=[45,56,100];DENSE=[97,99,102,108]
BATCH=R/'out/main-revalidation/bin/contact_batch'
def folder(stage,name):return OLD/stage/name if name=='balanced' else O/stage/name

def one(job):
 name,n=job;p=O/'presets'/name/f'n{n}.toml';p.parent.mkdir(parents=True,exist_ok=True)
 p.write_text(patch((OLD/'presets/balanced'/f'n{n}.toml').read_text(),{'soundboard.back_gain':.3 if name=='back30' else 0}))
 d=folder('attack',name);c.run([BATCH,p,d,n,','.join(map(str,VEL)),1.1,.1])
 m={f'{n}:{v}':c.measure_model(d/f'n{n}v{v}.wav',PAR[str(n)]) for v in VEL}
 if n in DENSE:
  d=folder('dense',name);c.run([BATCH,p,d,n,','.join(map(str,range(28,128))),.2,.05])
  c.save(d/f'n{n}.json',{f'{n}:{v}':c.measure_model(d/f'n{n}v{v}.wav',PAR[str(n)]) for v in range(28,128)})
 print(name,n,flush=True);return name,m

def scores(rows):
 result={}
 for name in NAMES:
  errors={s:[r['values'][name]-r['target'] for r in rows if r['source']==s] for s in {r['source'] for r in rows}}
  result[name]=dict(mae=np.mean([np.mean(np.abs(e)) for e in errors.values()]),bias=np.mean([np.mean(e) for e in errors.values()]),rows=len(rows),sources=len(errors),low_snr=sum(r['snr'][name]<12 for r in rows))
 return result

def main():
 models={name:{} for name in NAMES};models['balanced']=json.load(open(R/'bench/fsharp-contact/validation-models.json'))['balanced']
 with ThreadPoolExecutor(6) as ex:
  for name,m in ex.map(one,[(name,n) for name in NAMES[1:] for n in range(93,109)]):models[name].update(m)
 c.save(P/'validation-models.json',models)
 models['base']=json.load(open(R/'bench/main-revalidation/models.json'))['base']
 rows=c.fixed_rows(models,REF);prev=json.load(open(R/'bench/main-revalidation/summary.json'))
 ident=lambda rr:{(r['source'],r['note'],r['velocity'],r['k']) for r in rr}
 assert ident(rows)==ident(prev['attack_rows'])
 sus={name:{} for name in NAMES}
 for name in NAMES:
  for n in [96,97,99,102,105,108]:
   par=PAR[str(n)];k=np.arange(1,5);freq=par['f0']*k*np.sqrt(1+par['B']*k*k)
   for v in VEL:sus[name][f'{n}:{v}']=c.attack(c.b.load(str(folder('attack',name)/f'n{n}v{v}.wav'),stereo=True),freq,.5,.5)
 srows=[]
 for old in prev['sustain_rows']:
  r={k:old[k] for k in ['source','note','velocity','k','target']};key=f"{r['note']}:{r['velocity']}";i=r['k']-2
  r['values']={n:sus[n][key]['partials'][i] for n in NAMES};r['snr']={n:sus[n][key]['snr'][i] for n in NAMES}
  assert all(v is not None for v in r['values'].values());srows.append(r)
 dense={};dips={};levels={}
 for name in NAMES:
  dense[name]={};dips[name]={};levels[name]={}
  for n in DENSE:
   m=json.load(open(folder('dense',name)/f'n{n}.json'));dense[name].update(m);dips[name][n]=[]
   for i in range(3):
    a=[m[f'{n}:{v}']['partials'][i] for v in range(28,128)]
    dips[name][n].append(float(max(np.maximum.accumulate(a)-a)) if all(x is not None for x in a) else None)
   lv=[m[f'{n}:{v}']['level'] for v in range(28,128)];levels[name][n]=float(max(np.maximum.accumulate(lv)-lv))
 result=dict(attack=scores([r for r in rows if r['note']>=96]),transition=scores([r for r in rows if r['note']<96]),fsharp=scores([r for r in rows if r['note']==102]),c8=scores([r for r in rows if r['note']==108]),sustain=scores(srows),dips=dips,level_dips=levels,attack_rows=rows,sustain_rows=srows)
 c.save(P/'validation-summary.json',result);c.save(P/'validation-dense.json',dense)
 print(json.dumps(c.clean({k:v for k,v in result.items() if k not in ['attack_rows','sustain_rows']}),indent=1),flush=True)
if __name__=='__main__':main()
