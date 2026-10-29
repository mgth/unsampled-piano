"""Check the port against fresh independent force captures on the merged engine."""
from common import *
from scipy.io import wavfile

def main():
 a=s.a;a.BIN=BIN/'rayleigh_capture';report={}
 for geom in GEOMS:
  folder=O/f'kernel-{geom}';a.IR=np.load(folder/'responses.npz');a.CACHE.clear();a.GAIN=10**(tomllib.loads((O/geom/'base.toml').read_text())['soundboard']['mic_gain_db']/20)
  for n in DENSE:
   d=O/'check'/geom/f'n{n}';preset=O/geom/f'n{n}.toml';run([a.BIN,'capture',preset,d,n,'45,56,100',1.1,.1])
   for v in VEL:
    dry,new,reconstruction=a.stages(d,n,v);assert reconstruction['rms_error_db'] < -90
    out=d/f'rebuilt-v{v}.wav';a.save_audio(preset,new,out);sr,x=wavfile.read(out);sr,y=wavfile.read(O/'validation'/geom/'integral/attack'/f'n{n}v{v}.wav');err=x.astype(float)-y
    error=10*np.log10(np.mean(err**2)/(np.mean(x.astype(float)**2)+1e-30)+1e-30);assert error < -130,(geom,n,v,error)
    report[f'{geom}/{n}/{v}']=dict(reconstruction=reconstruction,live_error_db=error)
 save(P/'numerical-check.json',report);print('independent prototype vs live worst',max(x['live_error_db'] for x in report.values()),flush=True)
 # The previous bundle must fail explicitly, before it can produce sound.
 bad=subprocess.run(list(map(str,[BIN/'rayleigh_live',R/'out/rayleigh-ui/player_actual/bundle.toml',O/'stale-check',108,56,1.1,.1,512,1])),capture_output=True,text=True)
 assert bad.returncode!=0 and 'physics_fingerprint' in bad.stderr
 save(P/'stale-bundle-check.json',dict(rejected=True,stderr=bad.stderr))
if __name__=='__main__':main()
