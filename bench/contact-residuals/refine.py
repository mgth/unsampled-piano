from search import *
from rank import cost
G=json.load(open(OUT/'grid.json'));grid={}
for name in ['n97a058','n97a044','n97a056','n97a032','n97a077','n97a039','n97a063','n97a079']:grid[name+'dense']=G[name]
for i,(q,h,d) in enumerate(itertools.product([.15,.2,.25],[.1,.2,.3],[.00005,.000075,.0001])):
 grid[f'fine102-{i:02}']=dict(note=102,params=dict(q=q,h=h,p=0,w=1,d=d),extra=dict(strike_ratio=.105))
c.save(OUT/'refine-grid.json',grid)
vels=sorted(set(range(28,128,4))|{45,56,100,127})
def one(j):
 name,e=j;a=render(name,e['note'],e['params'],e.get('extra',{}),vels);return name,a
with ThreadPoolExecutor(6) as pool:
 results={}
 for name,a in pool.map(one,grid.items()):
  results[name]=a;print(name,flush=True)
c.save(OUT/'refine-results.json',results)
rank={}
for n in [97,102]:
 arr=[]
 for name,e in grid.items():
  if e['note']!=n:continue
  a=results[name];dips=[]
  for i in range(2):
   y=[a[f'{n}:{v}']['partials'][i] for v in vels];dips.append(float(max(np.maximum.accumulate(y)-y)))
  pp=a[f'{n}:45']['partials'];ff=a[f'{n}:100']['partials'];level=max(abs(a[f'{n}:{v}']['level']-MODELS['guarded'][f'{n}:{v}']['level']) for v in [45,56,100])
  penalty=sum(max(0,d-5)**2 for d in dips)
  if n==97:penalty+=max(0,abs(pp[0]+34.9477)-3)**2
  if n==102:penalty+=max(0,ff[1]+36)**2+max(0,-60-pp[1])**2
  arr.append(dict(name=name,cost=cost(a,n),penalty=penalty,dips=dips,level=level,spec=e,pp=pp,ff=ff))
 arr.sort(key=lambda r:r['cost']+r['penalty']);rank[n]=arr
 print(n,json.dumps(arr[:5],indent=1))
c.save(OUT/'refine-ranking.json',rank)
