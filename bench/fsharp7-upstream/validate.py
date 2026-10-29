from study import *
from dense import dip
NAMES=['control','freq375_h0.2','strike0.108_h0.15','hysteresis_0.15','s0.098_h0.2_m1.05']

def main():
 variants={**json.load(open(P/'dense-variants.json')),**json.load(open(P/'held-variants.json'))};models={n:{} for n in NAMES};dense={n:{} for n in NAMES}
 for name in NAMES:bundle(name,variants[name])
 def job(item):
  name,stage=item
  if '--reuse' in sys.argv and (O/name/stage/'metrics.json').exists():m=json.load(open(O/name/stage/'metrics.json'))
  else:m=render(name,variants[name],stage,NOTES if stage=='attack' else DENSE,VEL if stage=='attack' else list(range(28,128)),1.1 if stage=='attack' else .2,.1 if stage=='attack' else .05,prepared=True)
  print('validation',name,stage,flush=True);return name,stage,m
 with ThreadPoolExecutor(4) as ex:
  for name,stage,m in ex.map(job,[(n,s) for n in NAMES for s in ['attack','validation-dense']]):(models if stage=='attack' else dense)[name]=m
 rows=[];srows=[]
 for r in OLD['attack_rows']:
  x={k:r[k] for k in ['source','note','velocity','k','target']};key=f"{r['note']}:{r['velocity']}";i=r['k']-2
  x['values']={n:models[n][key]['partials'][i] for n in NAMES};x['snr']={n:models[n][key]['snr'][i] for n in NAMES};assert all(v is not None for v in x['values'].values());rows.append(x)
 sus={n:{} for n in NAMES}
 for n in NAMES:
  for note in [96,97,99,102,105,108]:
   p=PAR[str(note)];k=np.arange(1,5);f=p['f0']*k*np.sqrt(1+p['B']*k*k)
   for v in VEL:sus[n][f'{note}:{v}']=c.attack(b.load(str(O/n/'attack'/f'n{note}v{v}.wav'),stereo=True),f,.5,.5)
 for r in OLD['sustain_rows']:
  x={k:r[k] for k in ['source','note','velocity','k','target']};key=f"{r['note']}:{r['velocity']}";i=r['k']-2
  x['values']={n:sus[n][key]['partials'][i] for n in NAMES};x['snr']={n:sus[n][key]['snr'][i] for n in NAMES};assert all(v is not None for v in x['values'].values());srows.append(x)
 def score(rr):
  result={}
  for n in NAMES:
   by={src:[r['values'][n]-r['target'] for r in rr if r['source']==src] for src in {r['source'] for r in rr}}
   result[n]=dict(mae=np.mean([np.mean(np.abs(e)) for e in by.values()]),bias=np.mean([np.mean(e) for e in by.values()]),rows=len(rr),sources=len(by),per_source={src:dict(mae=np.mean(np.abs(e)),bias=np.mean(e)) for src,e in by.items()})
  return result
 subsets={'attack_all':rows,'top':[r for r in rows if r['note']>=96],'fsharp':[r for r in rows if r['note']==102],'c8':[r for r in rows if r['note']==108],'sustain_all':srows,'sustain_fsharp':[r for r in srows if r['note']==102]}
 for v in VEL:
  subsets[f'fsharp_v{v}']=[r for r in rows if r['note']==102 and r['velocity']==v];subsets[f'sustain_fsharp_v{v}']=[r for r in srows if r['note']==102 and r['velocity']==v]
 checks={}
 for n in NAMES[1:]:
  checks[n]={}
  for stage,ns,vs in [('attack',NOTES,VEL),('validation-dense',DENSE,range(28,128))]:
   equal=[];different=[]
   for note in ns:
    if note==102:continue
    for v in vs:
     f=f'n{note}v{v}.wav';a=O/n/stage/f;z=O/'control'/stage/f
     (equal if a.read_bytes()==z.read_bytes() else different).append(f)
   checks[n][stage]=dict(identical=len(equal),different=different);assert not different,(n,stage,different)
 ids=lambda rr:{tuple(r[k] for k in ['source','note','velocity','k']) for r in rr}
 assert len(rows)==459 and ids(rows)==ids(OLD['attack_rows']);assert len(srows)==71 and ids(srows)==ids(OLD['sustain_rows'])
 result=dict(variants={n:variants[n] for n in NAMES},scores={k:score(rr) for k,rr in subsets.items()},dips={n:{str(note):dip(dense[n],note) for note in DENSE} for n in NAMES},unchanged=checks,targets={n:summary(models[n],models['control']) for n in NAMES},attack_rows=rows,sustain_rows=srows,coverage=dict(attack=459,sustain=71,attack_wavs_per_profile=48,dense_wavs_per_profile=400))
 save(P/'validation-models.json',models);save(P/'validation-dense.json',dense);save(P/'validation-summary.json',result)
 print('fixed coverage and unchanged-note hashes PASS',flush=True)
if __name__=='__main__':main()
