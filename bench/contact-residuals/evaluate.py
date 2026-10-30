from search import *
models=json.load(open(OUT/'validation-models.json'));rows=c.fixed_rows(models,REFS);scores={}
for split,pred in dict(all_top=lambda r:r['note']>=96,target_keys=lambda r:r['note'] in [97,99,102],held_sources=lambda r:r['note']>=96 and r['source'] in c.HOLDOUT,pp=lambda r:r['note']>=96 and r['velocity']==45,unmodified_keys=lambda r:r['note']>=96 and r['note'] not in [97,99,102]).items():
 sub=[r for r in rows if pred(r)];scores[split]={}
 for name in models:
  errs={s:[r['values'][name]-r['target'] for r in sub if r['source']==s] for s in REFS};errs={s:v for s,v in errs.items() if v}
  scores[split][name]=dict(mae=np.mean([np.mean(np.abs(e)) for e in errs.values()]),bias=np.mean([np.mean(e) for e in errs.values()]),rows=len(sub),sources=len(errs),low_snr=sum(r['snr'][name]<12 for r in sub))
dips={}
for name,a in models.items():
 dips[name]={}
 for n in [97,99,102,108]:
  vels=sorted(int(k.split(':')[1]) for k in a if k.startswith(f'{n}:') and 28<=int(k.split(':')[1])<=127)
  p=[]
  for i in range(3):
   y=[a[f'{n}:{v}']['partials'][i] for v in vels]
   if any(v is None for v in y):p.append(None);continue
   drop=np.maximum.accumulate(y)-y;j=int(np.argmax(drop));i0=int(np.argmax(y[:j+1]));p.append(dict(drop=float(drop[j]),from_velocity=vels[i0],to_velocity=vels[j],minimum=min(y)))
  lv=[a[f'{n}:{v}']['level'] for v in vels];dips[name][n]=dict(count=len(vels),partials=p,level_dip=max(np.maximum.accumulate(lv)-lv))
residuals=[]
for n,v,i in [(102,100,1),(99,45,1),(97,45,0),(108,45,0),(108,56,0),(108,100,0)]:
 key=f'{n}:{v}';ref=[r[key]['partials'][i] for r in REFS.values() if key in r and r[key]['partials'][i] is not None]
 residuals.append(dict(note=n,velocity=v,k=i+2,target_mean=np.mean(ref),target_sd=np.std(ref),reference_count=len(ref),values={name:a[key]['partials'][i] for name,a in models.items()},level_delta_vs_guarded={name:a[key]['level']-models['guarded'][key]['level'] for name,a in models.items()}))
c.save(OUT/'summary-final.json',dict(analysis_version='contact-physical-peaks-v1',reference_protocol=c.b.ANALYSIS_VERSION,scores=scores,dips=dips,residuals=residuals,coverage_rows=rows))
print('SCORES',json.dumps(c.clean(scores),indent=1));print('RESIDUALS',json.dumps(c.clean(residuals),indent=1));print('REGULAR DIPS',json.dumps(c.clean(dips['regular']),indent=1))
