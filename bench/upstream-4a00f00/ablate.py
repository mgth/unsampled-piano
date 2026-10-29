"""Diagnose upstream changes only; never select or promote another contact."""
from common import *
from scipy.io import wavfile

def main():
 a=s.a;a.BIN=BIN/'rayleigh_capture';a.IR=np.load(O/'kernel-close/responses.npz');a.CACHE.clear();a.GAIN=10**(tomllib.loads((O/'close/base.toml').read_text())['soundboard']['mic_gain_db']/20)
 variants={'current':{},'width2':{'unison.width_curve':2.0},'frame_oneway':{'frame.coupling':0.0},'frame_loss1':{'frame.loss_factor':.01},'frame_length175':{'frame.a':1.75},'previous_numeric':{'unison.width_curve':2.0,'frame.coupling':0.0,'frame.loss_factor':.01,'frame.a':1.75}}
 par=json.load(open(P/'note-parameters.json'));result={}
 for name,fields in variants.items():
  result[name]={}
  for n in [102,108]:
   d=O/'ablation'/name/f'n{n}';d.mkdir(parents=True,exist_ok=True);preset=d/'preset.toml';preset.write_text(s.prev.patch((O/'close'/f'n{n}.toml').read_text(),fields));run([a.BIN,'capture',preset,d,n,'45,56,100',1.1,.1])
   for v in VEL:
    _,new,check=a.stages(d,n,v);assert check['rms_error_db'] < -90
    path=d/f'output-v{v}.wav';a.save_audio(preset,new,path);result[name][f'{n}:{v}']=s.prev.measure(path,n)
  print(name,{k:np.round(v['partials'][:2],3).tolist() for k,v in result[name].items() if k.startswith('102')},flush=True)
 save(P/'ablation.json',dict(variants=variants,metrics=result,policy='Diagnostic single-field changes and old numerical settings; no re-tuning, no promotion. Old frame outline implementation is not restored by these ablations.'))
if __name__=='__main__':main()
