"""Out-of-position sensitivity; no fit to mismatched close-mic references."""
from support import *
import re

def main():
 a.IR=np.load(a.O/'kernel-player/responses.npz');a.CACHE.clear();a.GAIN=float(dict(line.split('=') for line in (a.O/'kernel-player/geometry.txt').read_text().splitlines())['mic_gain']);player=prev.tomllib.loads((R/'out/bridge-ear/presets/grid5.toml').read_text());extra={f'soundboard.{k}':v for k,v in player['soundboard'].items() if k in ['mics_relative','mic_gain_db','mic_pattern','grid_step','lid_angle']};extra['room.enabled']=False;choices=json.load(open(P/'selected.json'))['profiles'];out={}
 for name,profile in choices.items():
  e=deepcopy(profile['108']);e['extra'].update(extra);d=O/'player'/name;p=prev.preset(e,d/'preset.toml');p.write_text(re.sub(r'(?ms)^mics = \[.*?^\]$','mics = '+json.dumps(player['soundboard']['mics']),p.read_text()));cap=d/'capture';c.run([a.BIN,'capture',p,cap,108,'45,56,100',1.1,.1]);out[name]={}
  for v in [45,56,100]:
   dry,new,check=a.stages(cap,108,v);assert check['rms_error_db'] < -90;path=d/f'n108v{v}.wav';a.save_audio(p,new,path);out[name][v]=dict(metrics=prev.measure(path,108),reconstruction=check);(cap/f'n108v{v}.force.f32').unlink()
 c.save(P/'player-sensitivity.json',out)
if __name__=='__main__':main()
