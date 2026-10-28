"""Out-of-position sensitivity only, not a close-mic reference fit."""
from common import *
import re

def main():
 a.IR=np.load(a.O/'kernel-player/responses.npz');a.CACHE.clear();a.GAIN=float(dict(line.split('=') for line in (a.O/'kernel-player/geometry.txt').read_text().splitlines())['mic_gain'])
 player=tomllib.loads((R/'out/bridge-ear/presets/grid5.toml').read_text());extra={f'soundboard.{k}':v for k,v in player['soundboard'].items() if k in ['mics_relative','mic_gain_db','mic_pattern','grid_step','lid_angle']};extra['room.enabled']=False
 sel=json.load(open(P/'selected.json'))['profiles'];out={}
 for name in ['balanced','contact','contact_pp']:
  for n in [102,108]:
   if name=='contact_pp' and n==108:continue
   e=json.loads(json.dumps(entry(n) if name=='balanced' else sel[name][str(n)]));e['extra'].update(extra);d=O/'player'/name/f'n{n}';p=preset(e,d/'preset.toml');p.write_text(re.sub(r'(?ms)^mics = \[.*?^\]$', 'mics = '+json.dumps(player['soundboard']['mics']),p.read_text()));cap=d/'capture';c.run([a.BIN,'capture',p,cap,n,'45,56,100',1.1,.1]);out[f'{name}:{n}']={}
   for v in [45,56,100]:
    dry,new,check=a.stages(cap,n,v);assert check['rms_error_db'] < -90
    out[f'{name}:{n}'][v]={'reconstruction':check,'models':{}}
    for label,y in [('current',dry),('integral',new)]:
     path=d/f'{label}-v{v}.wav';a.save_audio(p,y,path);out[f'{name}:{n}'][v]['models'][label]=measure(path,n)
    (cap/f'n{n}v{v}.force.f32').unlink()
 c.save(P/'player-sensitivity.json',out)
if __name__=='__main__':main()
