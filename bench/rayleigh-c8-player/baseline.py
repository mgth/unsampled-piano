"""Actual player setup over the same notes/velocities as the frozen close bench."""
from support import *
from concurrent.futures import ThreadPoolExecutor
import shutil

def main():
 kernel('player_actual');jobs=[('attack',n,[45,56,100]) for n in range(93,109)]+[('dense',n,list(range(28,128))) for n in [97,99,102,108]]
 def one(job):
  stage,n,vel=job;e=CONTACT.get(str(n),prev.entry(n));name=f'baseline-n{n}';root=O/'validation/raw'/stage;m=capture(e,name,'player_actual',vel=vel,hold=1.1 if stage=='attack' else .2,tail=.1 if stage=='attack' else .05,root=root,keep=False,note=n)
  for v in vel:
   path=O/'validation'/stage/'player_actual/baseline'/f'n{n}v{v}.wav';path.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(root/'player_actual'/name/path.name,path)
  return stage,n,m
 result={'attack':{},'dense':{}}
 with ThreadPoolExecutor(4) as ex:
  for stage,n,m in ex.map(one,jobs):result[stage].update({f'{n}:{v}':x for v,x in m.items()})
 c.save(P/'player-baseline.json',result)
if __name__=='__main__':main()
