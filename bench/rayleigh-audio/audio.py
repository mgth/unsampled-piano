"""Offline board-only substitution, retaining the engine's structural/action sound.
Room is applied with the original Rust processor after substitution. No limiter
or gain matching added: the preceding contact_batch benchmark also uses Engine.
"""
from pathlib import Path
import sys,json
import numpy as np
from scipy.io import wavfile
from scipy.fft import rfft,irfft,next_fast_len
R=Path(__file__).resolve().parents[2];sys.path.insert(0,str(R/'tools'));import contact_voicing as c
P=R/'bench/rayleigh-audio';O=R/'out/rayleigh-audio';BIN=O/'bin/rayleigh_capture';OLD=R/'out/fsharp-contact/validation'
PAR=json.load(open(R/'bench/main-revalidation/note-parameters.json'));FS=48000
IR=np.load(O/'kernel5/responses.npz');CACHE={}
GAIN=float(dict(line.split('=') for line in (O/'kernel5/geometry.txt').read_text().splitlines())['mic_gain'])
def conv(force,kind):
 h=IR[kind];n=next_fast_len(len(force)+h.shape[-1]-1)
 key=kind,n
 if key not in CACHE:CACHE[key]=rfft(h,n=n,axis=-1)
 x=rfft(force.T,n=n,axis=-1)
 y=np.einsum('rf,ref->ef',x,CACHE[key],optimize=True)
 return irfft(y,n=n,axis=-1).T[:len(force)]
def stages(folder,n,v):
 sr,x=wavfile.read(folder/f'n{n}v{v}.wav');assert sr==FS and x.shape[1]==4
 force=np.fromfile(folder/f'n{n}v{v}.force.f32',dtype='<f4').reshape(-1,16).astype(float);assert len(force)==len(x)
 scale=float((folder/'scale.txt').read_text())*GAIN
 new=conv(force,'driven')*scale
 reconstructed=conv(force,'current')*scale
 old=x[:,2:].astype(float);dry=x[:,:2].astype(float)
 error=reconstructed-old
 checks=dict(rms_error_db=float(10*np.log10(np.mean(error**2)/(np.mean(old**2)+1e-30)+1e-30)),peak_error=float(np.max(abs(error))),peak_old=float(np.max(abs(old))))
 return dry,dry-old+new,checks

def save_audio(preset,dry,path):
 path.parent.mkdir(parents=True,exist_ok=True);tmp=path.with_suffix('.dry.wav');wavfile.write(tmp,FS,dry.astype(np.float32));c.run([BIN,'room',preset,tmp,path]);tmp.unlink()

def screen():
 result={}
 for n in [97,99,102,108]:
  preset=OLD/'presets/balanced'/f'n{n}.toml';folder=O/'screen-capture'/f'n{n}'
  c.run([BIN,'capture',preset,folder,n,'45,56,100',1.1,.1])
  for v in [45,56,100]:
   dry,new,check=stages(folder,n,v);result[f'{n}:{v}']=dict(reconstruction=check,models={})
   for name,a in [('control',0),('blend25',.25),('blend50',.5),('driven',1)]:
    path=O/'screen'/name/f'n{n}v{v}.wav';save_audio(preset,(1-a)*dry+a*new,path);result[f'{n}:{v}']['models'][name]=c.measure_model(path,PAR[str(n)])
   sr,base=wavfile.read(OLD/'attack/balanced'/f'n{n}v{v}.wav');sr,x=wavfile.read(O/'screen/control'/f'n{n}v{v}.wav');err=x.astype(float)-base
   result[f'{n}:{v}']['engine_control']=dict(max_error=float(np.max(abs(err))),rms_error_db=float(10*np.log10(np.mean(err**2)/(np.mean(base**2)+1e-30)+1e-30)))
   print(n,v,{name:np.round(x['partials'][:2],2).tolist() if n!=108 else x['partials'][0] for name,x in result[f'{n}:{v}']['models'].items()},check,flush=True)
 c.save(P/'screen.json',result)
if __name__=='__main__':screen()
