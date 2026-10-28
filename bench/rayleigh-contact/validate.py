"""Fresh long attacks/tenues for final contacts, frozen reference support.
All notes use the integral radiator. Only contacts 102 and 108 change.
"""
from common import *
from concurrent.futures import ThreadPoolExecutor
V=O/'validation';NAMES=['balanced','driven','contact','contact_pp'];ALLVEL=[45,56,100]
F={x['name']:x for x in json.load(open(P/'fsharp-dense-screen.json'))};C={x['name']:x for x in json.load(open(P/'c8-height-results.json'))}
PROFILES={'contact':{102:F['r020'],108:C['height009']},'contact_pp':{102:F['r018'],108:C['height009']}}
def file(stage,name,n,v):return (a.OLD/stage/'balanced' if name=='balanced' else a.O/'validation'/stage/'driven' if name=='driven' else V/stage/name)/f'n{n}v{v}.wav'
def scores(rows):
 result={}
 for name in NAMES:
  e={s:[r['values'][name]-r['target'] for r in rows if r['source']==s] for s in {r['source'] for r in rows}}
  result[name]=dict(mae=np.mean([np.mean(np.abs(v)) for v in e.values()]) if e else None,bias=np.mean([np.mean(v) for v in e.values()]) if e else None,rows=len(rows),sources=len(e),low_snr=sum(r['snr'][name]<12 for r in rows))
 return result

def main():
 c.save(P/'selected.json',dict(profiles={name:{n:x['entry'] for n,x in ps.items()} for name,ps in PROFILES.items()},policy='Experimental full radiator on every key; contact-only changes on 102 and 108. contact balances fixed-coverage error, absolute energy and dense velocity dips; contact_pp favors F-sharp pp and velocity regularity. C8 height009 retains energy better than rejected c025, with remaining mf and level tradeoffs. Same references used for selection and evaluation. No default change.'))
 model={k:json.load(open(R/'bench/rayleigh-audio/validation-models.json'))[k] for k in ['balanced','driven']};dense={k:json.load(open(R/'bench/rayleigh-audio/validation-dense.json'))[k] for k in ['balanced','driven']}
 def one(job):
  name,n=job;x=PROFILES[name][n];m=render(n,name,x['entry'],vel=ALLVEL,hold=1.1,tail=.1,keep_force=True,root=V/'long');
  for v in ALLVEL:
   dst=file('attack',name,n,v);dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(V/'long'/name/f'n{n}'/f'n{n}v{v}.wav',dst)
  return name,n,m
 fresh={}
 with ThreadPoolExecutor(3) as ex:
  for name,n,m in ex.map(one,[('contact',102),('contact',108),('contact_pp',102)]):fresh[name,n]=m
 fresh['contact_pp',108]=fresh['contact',108]
 for name in PROFILES:
  model[name]={};dense[name]={}
  for stage,ns,vel in [('attack',range(93,109),ALLVEL),('dense',[97,99,102,108],range(28,128))]:
   table=model if stage=='attack' else dense
   for n in ns:
    for v in vel:
     dst=file(stage,name,n,v);dst.parent.mkdir(parents=True,exist_ok=True)
     if n not in PROFILES[name]:
      src=file(stage,'driven',n,v);shutil.copy2(src,dst);table[name][f'{n}:{v}']=table['driven'][f'{n}:{v}'];assert src.read_bytes()==dst.read_bytes()
     elif stage=='attack':
      if name=='contact_pp' and n==108:shutil.copy2(file(stage,'contact',n,v),dst)
      table[name][f'{n}:{v}']=fresh[name,n][str(v)]
     else:
      chosen=PROFILES[name][n];src=O/'dense-screen'/chosen['name']/f'n{n}'/f'n{n}v{v}.wav';shutil.copy2(src,dst);table[name][f'{n}:{v}']=chosen['metrics'][str(v)]
 c.save(P/'validation-models.json',model);c.save(P/'validation-dense.json',dense)
 models={**model,'base':json.load(open(R/'bench/main-revalidation/models.json'))['base']};refs=json.load(open(R/'bench/main-revalidation/references.json'));rows=c.fixed_rows(models,refs)
 ident=lambda rr:{(r['source'],r['note'],r['velocity'],r['k']) for r in rr};prev=json.load(open(R/'bench/main-revalidation/summary.json'));assert ident(rows)==ident(prev['attack_rows'])
 sus={name:{} for name in NAMES}
 for name in NAMES:
  for n in [96,97,99,102,105,108]:
   par=PAR[str(n)];k=np.arange(1,5);freq=par['f0']*k*np.sqrt(1+par['B']*k*k)
   for v in ALLVEL:sus[name][f'{n}:{v}']=c.attack(c.b.load(str(file('attack',name,n,v)),stereo=True),freq,.5,.5)
 srows=[]
 for old in prev['sustain_rows']:
  r={k:old[k] for k in ['source','note','velocity','k','target']};key=f"{r['note']}:{r['velocity']}";i=r['k']-2;r['values']={n:sus[n][key]['partials'][i] for n in NAMES};r['snr']={n:sus[n][key]['snr'][i] for n in NAMES};assert all(v is not None for v in r['values'].values());srows.append(r)
 assert ident(srows)==ident(prev['sustain_rows'])
 dips={};levels={};level_dips={};band_changes={}
 for name in NAMES:
  dips[name]={};levels[name]={};level_dips[name]={};band_changes[name]={}
  for n in [97,99,102,108]:
   m=dense[name];dips[name][n]=[]
   for i in range(3):
    y=[m[f'{n}:{v}']['partials'][i] for v in range(28,128)];dips[name][n].append(float(max(np.maximum.accumulate(y)-y)) if all(x is not None for x in y) else None)
   levels[name][n]=[]
   for v in range(28,128):
    _,x=wavfile.read(file('dense',name,n,v));levels[name][n].append(float(c.b.onset_level(np.pad(x,((1024,0),(0,0))))))
   level_dips[name][n]=float(max(np.maximum.accumulate(levels[name][n])-levels[name][n]))
   band_changes[name][n]={}
   for v in ALLVEL:
    a1=measure(file('attack',name,n,v),n);b1=measure(file('attack','balanced',n,v),n)
    band_changes[name][n][v]=dict(bands={k:a1['bands'][k]-b1['bands'][k] for k in a1['bands']},p1=a1['p1']-b1['p1'],level=a1['level_preroll']-b1['level_preroll'])
 out=dict(attack=scores([r for r in rows if r['note']>=96]),transition=scores([r for r in rows if r['note']<96]),fsharp=scores([r for r in rows if r['note']==102]),c8=scores([r for r in rows if r['note']==108]),sustain=scores(srows),fsharp_sustain=scores([r for r in srows if r['note']==102]),c8_sustain=scores([r for r in srows if r['note']==108]),dips=dips,level_dips=level_dips,level_curves=levels,band_changes=band_changes,unchanged_vs_full=dict(attack_per_candidate=42,dense_per_candidate=200),attack_rows=rows,sustain_rows=srows)
 c.save(P/'validation-summary.json',out);print(json.dumps(c.clean({k:v for k,v in out.items() if k not in ['attack_rows','sustain_rows','level_curves','band_changes']}),indent=1))
if __name__=='__main__':main()
