"""Fresh long attacks and dense curves for comparison candidates, not promotion."""
from support import *
from concurrent.futures import ThreadPoolExecutor
import shutil

def main():
 geom=sys.argv[1];kernel(geom);one={x['name']:x for x in json.load(open(P/'close-screen.json'))};combo={x['name']:x for x in json.load(open(P/'close-coupled.json'))};candidates={'soft':one['q0.25'],'weighted':combo['c01'],'energy':combo['c07']};c.save(P/f'{geom}-candidates.json',{k:v['entry'] for k,v in candidates.items()})
 def job(arg):
  name,stage=arg;vel=[45,56,100] if stage=='attack' else list(range(28,128));root=O/'validation/raw'/stage;m=capture(candidates[name]['entry'],name,geom,vel=vel,hold=1.1 if stage=='attack' else .2,tail=.1 if stage=='attack' else .05,root=root,keep=stage=='attack')
  for v in vel:
   path=O/'validation'/stage/geom/name/f'n108v{v}.wav';path.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(root/geom/name/path.name,path)
  return name,stage,m
 out={name:{} for name in candidates}
 with ThreadPoolExecutor(3) as ex:
  for name,stage,m in ex.map(job,[(name,stage) for name in candidates for stage in ['attack','dense']]):out[name][stage]=m
 c.save(P/f'{geom}-final-models.json',out)
if __name__=='__main__':main()
