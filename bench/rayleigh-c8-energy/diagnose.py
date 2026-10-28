"""Separate real low-band loss from onset-window effects and radiation paths."""
from support import *
from scipy.signal import butter,sosfilt

def window(x,start,size):
 seg=x[start:start+size];w=np.hanning(len(seg));X=np.fft.rfft(seg*w[:,None],n=32768,axis=0);f=np.fft.rfftfreq(32768,1/48000);power=2*abs(X)**2/(48000*np.sum(w*w))*48000/32768
 return float(10*np.log10(power[(f>=1000)&(f<2000)].mean(axis=1).sum()+1e-30))
def measure(x,on):
 filt=sosfilt(butter(4,[1000,2000],btype='bandpass',fs=48000,output='sos'),x,axis=0)
 return dict(onset=int(c.b.onset(x)),own60=window(x,c.b.onset(x),2880),fixed60=window(x,on,2880),trigger60=window(x,0,2880),trigger200=window(x,0,9600),filtered_energy200=float(10*np.log10(np.mean(filt[:9600]**2)+1e-30)))
def main():
 out={}
 for name,e,folder in [('old_contact',prev.entry(108),a.O/'validation/capture/attack/n108'),('contact',SEED,prev.O/'validation/long/contact/n108/capture')]:
  p=prev.preset(e,O/'diagnose'/name/'preset.toml');out[name]={}
  for v in [45,56,100]:
   dry,new,check=a.stages(folder,108,v);_,four=wavfile.read(folder/f'n108v{v}.wav');res=dry-four[:,2:];board=new-res;on=80;out[name][v]={}
   for label,y in [('total',new),('residual',res),('board',board)]:
    path=O/'diagnose'/name/f'{label}-v{v}.wav';a.save_audio(p,y,path);x=c.b.load(str(path),stereo=True);out[name][v][label]=measure(x,on)
  for label,extra in [('no_action',{'noise.action':0}),('no_keybed',{'noise.keybed':0}),('no_frame',{'frame.enabled':False})]:
   ee=deepcopy(e);ee['extra'].update(extra);
   if not (O/'ablation'/f'{name}_{label}/n108/n108v100.wav').exists():prev.render(108,name+'_'+label,ee,vel=[45,56,100],root=O/'ablation',keep_force=False)
   for v in [45,56,100]:
    x=c.b.load(str(O/'ablation'/f'{name}_{label}/n108/n108v{v}.wav'),stereo=True);out[name][v][label]=measure(x,80)
 c.save(P/'diagnostic.json',out);print(json.dumps(c.clean(out),indent=1))
if __name__=='__main__':main()
