from common import *
from concurrent.futures import ThreadPoolExecutor

def main():
 results=json.load(open(P/'refine-results.json'));chosen=[x for x in results if x['note']==102 and x['name'] in ['r012','r018','r020','r030','r032']]
 def one(x):
  n=x['note'];name=x['name'];m=render(n,name,x['entry'],vel=range(28,128),root=O/'dense-screen');s=score(n,m);r=dict(note=n,name=name,entry=x['entry'],metrics=m,**s);print(n,name,s,flush=True);return r
 with ThreadPoolExecutor(5) as ex:r=list(ex.map(one,chosen))
 c.save(P/'fsharp-dense-screen.json',r)
if __name__=='__main__':main()
