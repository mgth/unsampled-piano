"""Frequency dependence and the action path's invariance to position/gain."""
from support import *
from scipy.fft import rfft

def main():
 response={};f=np.fft.rfftfreq(65536,1/48000);frequencies=list(range(1000,2001,50))
 for geom in ['close_dry','player35','player0','player_actual_dry']:
  kernel(geom);H=rfft(a.IR['driven'][15],axis=-1)*a.GAIN;ix=[int(np.argmin(abs(f-F))) for F in frequencies];response[geom]=dict(frequency=f[ix],gain_db=10*np.log10(np.mean(abs(H[:,ix])**2,axis=0)+1e-30),ear_gain_db=20*np.log10(abs(H[:,ix])+1e-30),ear_phase=np.angle(H[:,ix]))
 invariant={}
 for v in [45,56,100]:
  read=lambda geom,name:wavfile.read(O/'decomposition'/geom/name/f'action-v{v}.wav')[1].astype(float)
  ref=read('close_dry','recovered');invariant[v]={}
  for geom,name in [('player_actual_dry','recovered'),('close_dry','previous')]:
   y=read(geom,name);err=y-ref;invariant[v][f'{geom}:{name}']=dict(max_error=float(np.max(abs(err))),rms_error_db=float(10*np.log10(np.mean(err**2)/(np.mean(ref**2)+1e-30)+1e-30)))
 c.save(P/'path-checks.json',dict(action_invariance=invariant,unit_region15_response=response,policy='Transfer of unit bridge-region force, not a reference-piano fit. Both ears included. Dry action isolated by no-action subtraction; its MIDI input and fixed pan do not follow soundboard position/gain or contact.'))
 print(json.dumps(c.clean(invariant),indent=1))
if __name__=='__main__':main()
