"""Frozen support: 459 attacks, 71 sustains, four keys x 100 velocities.
Global driven radiation and explicitly local/dynamic guarded crossfade.
"""
from audio import *
from guard_search import guarded
from concurrent.futures import ThreadPoolExecutor
import shutil,hashlib,threading
REF=json.load(open(R/'bench/main-revalidation/references.json'));NAMES=['balanced','driven','guarded'];VEL=[45,56,100];DENSE=[97,99,102,108]
V=O/'validation';LOCK=threading.Lock()
def path(stage,name,n,v):return (OLD/stage/'balanced' if name=='balanced' else V/stage/name)/f'n{n}v{v}.wav'
def job(item):
 stage,n=item;vel=VEL if stage=='attack' else range(28,128);hold,tail=(1.1,.1) if stage=='attack' else (.2,.05)
 preset=OLD/'presets/balanced'/f'n{n}.toml';folder=V/'capture'/stage/f'n{n}'
 c.run([BIN,'capture',preset,folder,n,','.join(map(str,vel)),hold,tail]);models={};checks={}
 for v in vel:
  dry,new,check=stages(folder,n,v);out=path(stage,'driven',n,v);save_audio(preset,new,out);models[f'{n}:{v}']=c.measure_model(out,PAR[str(n)])
  control=folder/f'control-v{v}.wav';save_audio(preset,dry,control);_,x=wavfile.read(control);_,base=wavfile.read(path(stage,'balanced',n,v));assert x.shape==base.shape
  check['control_max_error']=float(np.max(abs(x-base)));assert check['control_max_error']==0
  check['reconstruction_pass']=check['rms_error_db'] < -90;assert check['reconstruction_pass'];checks[f'{n}:{v}']=check;control.unlink()
 print('finished',stage,n,flush=True);return stage,models,checks

def scores(rows):
 out={}
 for name in NAMES:
  errors={s:[r['values'][name]-r['target'] for r in rows if r['source']==s] for s in {r['source'] for r in rows}}
  out[name]=dict(mae=np.mean([np.mean(np.abs(e)) for e in errors.values()]),bias=np.mean([np.mean(e) for e in errors.values()]),rows=len(rows),sources=len(errors),low_snr=sum(r['snr'][name]<12 for r in rows))
 return out

def main():
 models={name:{} for name in NAMES};dense={name:{} for name in NAMES};models['balanced']=json.load(open(R/'bench/fsharp-contact/validation-models.json'))['balanced'];dense['balanced']=json.load(open(R/'bench/fsharp-contact/validation-dense.json'))['balanced'];checks={'attack':{},'dense':{}}
 if '--reuse' in sys.argv:
  models=json.load(open(P/'validation-models.json'));dense=json.load(open(P/'validation-dense.json'));checks=json.load(open(P/'reconstruction-checks.json'))
 else:
  with ThreadPoolExecutor(4) as ex:
   for stage,m,check in ex.map(job,[('attack',n) for n in range(93,109)]+[('dense',n) for n in DENSE]):
    (models if stage=='attack' else dense)['driven'].update(m);checks[stage].update(check)
 c.save(P/'reconstruction-checks.json',checks)
 guard=json.load(open(P/'guard-selected.json'));equal={};onsets={}
 for stage,table in [('attack',models),('dense',dense)]:
  equal[stage]=0;onsets[stage]={}
  for key in table['driven']:
   n,v=map(int,key.split(':'));dst=path(stage,'guarded',n,v);dst.parent.mkdir(parents=True,exist_ok=True);base=path(stage,'balanced',n,v)
   if n!=102 or v<=guard['unchanged_below_or_at']:
    shutil.copy2(base,dst);assert base.read_bytes()==dst.read_bytes();equal[stage]+=1
   else:
    _,x=wavfile.read(base);_,y=wavfile.read(path(stage,'driven',n,v));z=guarded(x.astype(float),y.astype(float),v,guard['maximum'],guard['end'],start=guard['start'],shape=guard.get('shape','smooth'));wavfile.write(dst,FS,z.astype(np.float32))
    onsets[stage][key]=int(c.b.onset(z)-c.b.onset(x)-64)
   table['guarded'][key]=c.measure_model(dst,PAR[str(n)])
 c.save(P/'validation-models.json',models);c.save(P/'validation-dense.json',dense)
 models['base']=json.load(open(R/'bench/main-revalidation/models.json'))['base'];rows=c.fixed_rows(models,REF);prev=json.load(open(R/'bench/main-revalidation/summary.json'))
 ident=lambda rr:{(r['source'],r['note'],r['velocity'],r['k']) for r in rr};assert ident(rows)==ident(prev['attack_rows'])
 sus={name:{} for name in NAMES}
 for name in NAMES:
  for n in [96,97,99,102,105,108]:
   par=PAR[str(n)];k=np.arange(1,5);freq=par['f0']*k*np.sqrt(1+par['B']*k*k)
   for v in VEL:sus[name][f'{n}:{v}']=c.attack(c.b.load(str(path('attack',name,n,v)),stereo=True),freq,.5,.5)
 srows=[]
 for old in prev['sustain_rows']:
  r={k:old[k] for k in ['source','note','velocity','k','target']};key=f"{r['note']}:{r['velocity']}";i=r['k']-2
  r['values']={n:sus[n][key]['partials'][i] for n in NAMES};r['snr']={n:sus[n][key]['snr'][i] for n in NAMES};assert all(v is not None for v in r['values'].values());srows.append(r)
 assert ident(srows)==ident(prev['sustain_rows'])
 dips={};levels={}
 for name in NAMES:
  dips[name]={};levels[name]={}
  for n in DENSE:
   m=dense[name];dips[name][n]=[]
   for i in range(3):
    a=[m[f'{n}:{v}']['partials'][i] for v in range(28,128)];dips[name][n].append(float(max(np.maximum.accumulate(a)-a)) if all(x is not None for x in a) else None)
   lv=[m[f'{n}:{v}']['level'] for v in range(28,128)];levels[name][n]=float(max(np.maximum.accumulate(lv)-lv))
 result=dict(attack=scores([r for r in rows if r['note']>=96]),transition=scores([r for r in rows if r['note']<96]),fsharp=scores([r for r in rows if r['note']==102]),c8=scores([r for r in rows if r['note']==108]),sustain=scores(srows),dips=dips,level_dips=levels,guard_equal_files=equal,guard_onset_offset_after_latency=onsets,attack_rows=rows,sustain_rows=srows)
 c.save(P/'validation-summary.json',result)
 print(json.dumps(c.clean({k:v for k,v in result.items() if k not in ['attack_rows','sustain_rows','guard_onset_offset_after_latency']}),indent=1),flush=True)
if __name__=='__main__':main()
