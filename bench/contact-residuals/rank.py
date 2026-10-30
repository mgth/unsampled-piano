from search import *
TARGETS={}
for n in [97,99,102]:
 for v in [45,56,100]:
  k=f'{n}:{v}';TARGETS[k]=[]
  for i in range(3):
   vals=[r[k]['partials'][i] for s,r in REFS.items() if s not in c.HOLDOUT and k in r and r[k]['partials'][i] is not None]
   TARGETS[k].append(dict(mean=float(np.mean(vals)),sd=max(4,float(np.std(vals))),count=len(vals)) if vals else None)
def cost(a,n):
 err=[];pen=0
 for v in [45,56,100]:
  k=f'{n}:{v}';r=a[k]
  for i,t in enumerate(TARGETS[k]):
   if t and t['count']>=2:err.append(((r['partials'][i]-t['mean'])/t['sd'])**2)
  pen+=.2*max(0,abs(r['level']-MODELS['base'][k]['level'])-3)**2
 return float(np.mean(err)+pen)
def rank():
 grid={};data={}
 for f in ['grid','height','ablation']:
  g=json.load(open(OUT/('grid.json' if f=='grid' else f+'-grid.json')));grid.update(g)
  for name,e in g.items():
   p=OUT/'grid'/name/'metrics.json'
   if p.exists():data[name]=json.load(open(p))
 result={}
 for n in [97,99,102]:
  arr=[]
  for name,e in grid.items():
   if e['note']!=n or name not in data or e.get('extra'):continue
   a=data[name];r=a[f'{n}:45']['partials']
   # Explicit residual guard: correct the specified weak partial, retain pp brightness at F#7.
   if n==97 and abs(r[0]-TARGETS['97:45'][0]['mean'])>3:continue
   if n==99 and not(-51<=r[1]<=-42):continue
   if n==102 and (r[1]<-62 or a['102:100']['partials'][1]>-35):continue
   arr.append((cost(a,n),name,a))
  arr.sort(key=lambda x:x[0]);print('\nNOTE',n,'N',len(arr),'guard cost',cost(MODELS['guarded'],n))
  for sc,name,a in arr[:8]:print(round(sc,3),name,grid[name]['params'],[[round(x,1) if x is not None else None for x in a[f'{n}:{v}']['partials']] for v in [45,56,100]],'level',[round(a[f'{n}:{v}']['level']-MODELS['base'][f'{n}:{v}']['level'],1) for v in [45,56,100]])
  result[n]=[dict(cost=sc,name=name,spec=grid[name],metrics=a) for sc,name,a in arr]
 c.save(OUT/'ranking.json',result);c.save(OUT/'training-targets.json',TARGETS)
if __name__=='__main__':rank()
