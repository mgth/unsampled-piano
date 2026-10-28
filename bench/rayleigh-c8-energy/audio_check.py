"""Audio sensitivity at 10 contact steps, same Full instrument and frozen support.
The audit also disables silence shortcuts; it is not a pure integrator change.
Default-mode captures are first compared bitwise to the frozen previous binary.
"""
from support import *

def main():
 binary=O/'bin/rayleigh_capture';choices=json.load(open(P/'selected.json'))['profiles'];entries={'balanced':prev.entry(108),'contact':SEED,**{name:ps['108'] for name,ps in choices.items()}};out={};controls={}
 for name,e in entries.items():
  p=prev.preset(e,O/'audio-audit'/name/'preset.toml');cap=p.parent/'full-control';c.run([binary,'capture',p,cap,108,'45,56,100',1.1,.1]);old=(a.O/'validation/capture/attack/n108' if name=='balanced' else prev.O/'validation/long/contact/n108/capture' if name=='contact' else O/'validation/long'/name/'n108/capture')
  for v in [45,56,100]:
   assert (cap/f'n108v{v}.wav').read_bytes()==(old/f'n108v{v}.wav').read_bytes()
   assert (cap/f'n108v{v}.force.f32').read_bytes()==(old/f'n108v{v}.force.f32').read_bytes()
   (cap/f'n108v{v}.force.f32').unlink()
  controls[name]=dict(wavs_identical=3,force_captures_identical=3)
 def one(item):
  name,e=item;p=O/'audio-audit'/name/'preset.toml';cap=p.parent/'audit10';c.run([binary,'capture',p,cap,108,','.join(map(str,range(28,128))),.2,.05,'audit10']);m={};checks={}
  for v in range(28,128):
   dry,new,check=a.stages(cap,108,v);assert check['rms_error_db'] < -90;path=p.parent/f'n108v{v}.wav';a.save_audio(p,new,path);m[str(v)]=prev.measure(path,108);checks[v]=check;(cap/f'n108v{v}.force.f32').unlink()
  return name,dict(metrics=m,score=evaluate(m),reconstruction=checks)
 with ThreadPoolExecutor(4) as ex:out=dict(ex.map(one,entries.items()))
 legacy={}
 for v in range(28,128):
  _,x=wavfile.read(O/'audio-audit/balanced/audit10'/f'n108v{v}.wav');path=O/'audio-audit/legacy10'/f'n108v{v}.wav';a.save_audio(O/'audio-audit/balanced/preset.toml',x[:,:2],path);legacy[str(v)]=prev.measure(path,108)
 out['legacy10']=dict(metrics=legacy,score=evaluate(legacy))
 c.save(P/'audio-convergence.json',dict(controls=controls,models=out,labels={'balanced':'integral radiator with old balanced contact','legacy10':'historical radiator with old balanced contact'},policy='10 contact steps with engine silence shortcuts disabled, Full instrument unchanged. Same 15 C8 reference identities and 100 MIDI velocities. No production/default change.'))
 print(json.dumps(c.clean({name:r['score'] for name,r in out.items()}),indent=1))
if __name__=='__main__':main()
