from common import *
from concurrent.futures import ThreadPoolExecutor

def main():
 results=json.load(open(P/'c8-refine-results.json'));chosen=[x for x in results if x['name'] in ['c001','c019','c023','c024','c025']]
 def one(x):
  n=x['note'];name=x['name'];m=render(n,name,x['entry'],vel=range(28,128),root=O/'dense-screen');s=score(n,m);r=dict(note=n,name=name,entry=x['entry'],metrics=m,**s);print(n,name,s,flush=True);return r
 with ThreadPoolExecutor(5) as ex:r=list(ex.map(one,chosen))
 c.save(P/'c8-dense-screen.json',r)
if __name__=='__main__':main()
