"""Dense final height sweep; assess energy as well as partial ratios."""
from common import *
from concurrent.futures import ThreadPoolExecutor
from copy import deepcopy

def main():
 seed=next(x for x in json.load(open(P/'c8-refine-results.json')) if x['name']=='c032');jobs=[]
 for q in [.3,.35,.4]:
  for d in [10e-6,15e-6,20e-6,25e-6]:
   e=deepcopy(seed['entry']);e['params'].update(q=q,d=d);jobs.append(dict(note=108,name=f'height{len(jobs):03}',entry=e))
 c.save(P/'c8-height-design.json',jobs)
 def one(x):
  m=render(108,x['name'],x['entry'],vel=range(28,128),root=O/'dense-screen');s=score(108,m);print(x['name'],s,flush=True);return dict(**x,metrics=m,**s)
 with ThreadPoolExecutor(4) as ex:r=list(ex.map(one,jobs))
 c.save(P/'c8-height-results.json',r)
if __name__=='__main__':main()
