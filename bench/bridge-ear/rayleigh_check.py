"""Run the offline driven-surface diagnostic, excluding room, lid and wall."""
from pathlib import Path
import sys,json,subprocess,shutil
from concurrent.futures import ThreadPoolExecutor
from preset_fields import patch
R=Path(__file__).resolve().parents[2];O=R/'out/bridge-ear';P=R/'bench/bridge-ear'
def main():
 shutil.copy2(R/'target/release/examples/driven_rayleigh',O/'bin/driven_rayleigh')
 d=O/'presets';(d/'close5.toml').write_text(patch((d/'close.toml').read_text(),{'soundboard.grid_step':.005}))
 f=','.join(map(str,json.load(open(P/'diagnostic-design.json'))['model_frequencies']))
 def run(case):
  with open(O/f'{case}-driven.csv','w') as out,open(O/f'{case}-driven.log','w') as err:
   subprocess.run([O/'bin/driven_rayleigh',d/f'{case}.toml','102',f],stdout=out,stderr=err,check=True)
  if case=='close5':
   with open(O/f'{case}-transfer.csv','w') as out:subprocess.run([O/'bin/ear_transfer',d/f'{case}.toml','102',f],stdout=out,check=True)
  print(case,flush=True)
 with ThreadPoolExecutor(3) as ex:list(ex.map(run,['player','grid75','grid5','front_only','close','close5']))
if __name__=='__main__':main()
