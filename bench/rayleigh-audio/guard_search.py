"""Explicit voicing experiment, not a physical velocity-dependent radiator.
Only F#7, only the high band; the crossover coefficient is fixed at note-on.
Protect v<=56 and all other keys. Screen dense velocities before full scoring.
"""
from audio import *
from scipy.signal import firwin,fftconvolve
HP=firwin(129,7500,pass_zero=False,fs=FS,window=('kaiser',8.6));DELAY=64

def guarded(dry,new,v,maximum,end,start=56,shape="smooth"):
 assert shape in ["smooth","linear"]
 u=np.clip((v-start)/(end-start),0,1);a=maximum*(u if shape=="linear" else u*u*(3-2*u))
 if a==0:return dry
 delta=fftconvolve(new-dry,HP[:,None],axes=0)[:len(dry)+DELAY]
 return np.pad(dry,((DELAY,0),(0,0)))+a*delta

def main():
 folder=O/'dense-capture/n102';preset=OLD/'presets/balanced/n102.toml'
 if '--refresh' in sys.argv or not all((O/'dense-screen'/kind/f'n102v{v}.wav').exists() for kind in ['control','driven'] for v in range(28,128)):
  c.run([BIN,'capture',preset,folder,102,','.join(map(str,range(28,128))),.2,.05])
  for v in range(28,128):
   dry,new,_=stages(folder,102,v)
   for kind,y in [('control',dry),('driven',new)]:save_audio(preset,y,O/'dense-screen'/kind/f'n102v{v}.wav')
 wet={}
 for v in range(28,128):
  wet[v]=tuple(wavfile.read(O/'dense-screen'/kind/f'n102v{v}.wav')[1].astype(float) for kind in ['control','driven'])
 results=[]
 for maximum in [.05,.1,.15,.2,.25,.35,.45,.55,.65,.75]:
  for end in [64,68,76,80,88,96,104]:
   metrics={v:c.attack(guarded(d,n,v,maximum,end),np.array([1,2,3,4])*PAR['102']['f0']*np.sqrt(1+PAR['102']['B']*np.array([1,2,3,4])**2)) for v,(d,n) in wet.items()}
   dips=[]
   for k in [0,1]:
    y=np.array([metrics[v]['partials'][k] for v in range(28,128)]);dips.append(float(max(np.maximum.accumulate(y)-y)))
   delta={str(v):[metrics[v]['partials'][i]-c.measure_model(OLD/'dense/balanced'/f'n102v{v}.wav',PAR['102'])['partials'][i] for i in [0,1]] for v in [45,56,100]}
   results.append(dict(maximum=maximum,end=end,dips=dips,delta=delta));print(results[-1],flush=True)
 c.save(P/'guard-screen.json',results)
 # Choose only after recording all candidates; keep the predefined guards.
 passing=[r for r in results if r['dips'][0]<=1.5 and r['dips'][1]<=1.5 and abs(r['delta']['100'][0])<=.1 and r['delta']['100'][1]>=-8.4]
 assert passing
 best=min(passing,key=lambda r:abs(8.4+r['delta']['100'][1]));c.save(P/'guard-selected.json',dict(**best,note=102,unchanged_below_or_at=56,cutoff_hz=7500,taps=129,latency_samples=64,classification='Targeted velocity-dependent voicing crossfade; not a validated physical radiation law.',selected_on='F#7 dense model guard and previous mean reference excess; no held-out reference test.'))
 print('SELECTED',best,flush=True)
if __name__=='__main__':main()
