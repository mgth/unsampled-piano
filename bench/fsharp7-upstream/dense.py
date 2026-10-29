from study import *

def dip(m,n=102):
 result={}
 for k in [0,1]:
  values=[m[f'{n}:{v}']['partials'][k] for v in range(28,128)]
  if all(v is None for v in values):
   result[f'p{k+2}']=None;continue
  y=np.array(values,float);assert np.isfinite(y).all()
  drop=np.maximum.accumulate(y)-y;j=int(np.argmax(drop));i=int(np.argmax(y[:j+1]));result[f'p{k+2}']=dict(db=float(drop[j]),from_velocity=28+i,to_velocity=28+j)
 y=np.array([m[f'{n}:{v}']['level_preroll'] for v in range(28,128)]);drop=np.maximum.accumulate(y)-y;j=int(np.argmax(drop));i=int(np.argmax(y[:j+1]));result['level']=dict(db=float(drop[j]),from_velocity=28+i,to_velocity=28+j)
 return result

if __name__=='__main__':
 ss=json.load(open(P/'refine-summary.json'));base=json.load(open(P/'screen-summary.json'))['control'];variants=json.load(open(P/'refine-variants.json'));selected={'control':{}}
 for n,r in ss.items():
  if all(r['scores'][v]['mae']<=base['scores'][v]['mae']+.05 and r['delta'][v]['level']>=-.25 and r['delta'][v]['band_1_2k']>=-.25 for v in ['45','56']) and r['scores']['None']['mae']<=base['scores']['None']['mae']+.05 and abs(r['ff_p3_bias'])<abs(base['ff_p3_bias'])-.5:selected[n]=variants[n]
 save(P/'dense-variants.json',selected)
 def job(item):
  n,f=item;m=render(n,f,'dense',vel=list(range(28,128)),hold=.2,tail=.05);print('dense',n,flush=True);return n,m
 with ThreadPoolExecutor(4) as ex:models=dict(ex.map(job,selected.items()))
 ds={n:dip(m) for n,m in models.items()};passed=[n for n,r in ds.items() if n!='control' and all(r[k]['db']<=ds['control'][k]['db']+.1 for k in ['p2','p3','level'])]
 save(P/'dense-models.json',models);save(P/'dense-summary.json',dict(dips=ds,passed=passed))
 print(json.dumps(dict(dips=ds,passed=passed),indent=1),flush=True)
