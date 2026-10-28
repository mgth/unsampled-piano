"""Separate absolute levels, relative spectra, residual paths and windows."""
from common import *
from scipy.fft import rfft

def attack_at(x,n,start,duration):
 par=PAR[str(n)];kk=np.arange(1,5);freq=par['f0']*kk*np.sqrt(1+par['B']*kk*kk);seg=x[start:start+round(duration*48000)];N=32768;f=np.fft.rfftfreq(N,1/48000);power=c.b.spectrum_power(seg,N);levels=[];peaks=[]
 for F in freq:
  if F>=12000:levels.append(None);peaks.append(None);continue
  width=max(2*48000/len(seg),.015*F);ix=np.flatnonzero(abs(f-F)<width);j=ix[np.argmax(power[ix])];levels.append(float(10*np.log10(power[j]+1e-30)));peaks.append(float(f[j]))
 return dict(partials=[None if q is None else q-levels[0] for q in levels[1:]],p1=levels[0],peak_hz=peaks)

def main():
 old=json.load(open(R/'bench/rayleigh-audio/validation-dense.json'));d={};windows={};stages={}
 for n in [102,108]:
  d[n]={}
  for v in [45,56,100]:
   d[n][v]={}
   for name in ['balanced','driven']:
    path=(a.OLD/'attack/balanced' if name=='balanced' else a.O/'validation/attack/driven')/f'n{n}v{v}.wav';d[n][v][name]=measure(path,n)
  selected=sorted(set([45,56,100]+[28+int(np.argmax(np.maximum.accumulate([old[name][f'{n}:{v}']['partials'][0] for v in range(28,128)])-[old[name][f'{n}:{v}']['partials'][0] for v in range(28,128)])) for name in ['balanced','driven']]))
  windows[n]={};stages[n]={}
  for v in selected:
   windows[n][v]={}
   for name in ['balanced','driven']:
    path=(a.OLD/'dense/balanced' if name=='balanced' else a.O/'validation/dense/driven')/f'n{n}v{v}.wav';x=c.b.load(str(path),stereo=True);on=c.b.onset(x)
    windows[n][v][name]={tag:attack_at(x,n,on+round(t*48000),dur) for tag,t,dur in [('attack60',0,.06),('attack120',0,.12),('body',.08,.12)]}
   if v in [45,56,100]:
    folder=a.O/'validation/capture/attack'/f'n{n}';sr,x=wavfile.read(folder/f'n{n}v{v}.wav');force=np.fromfile(folder/f'n{n}v{v}.force.f32',dtype='<f4').reshape(-1,16).astype(float);scale=float((folder/'scale.txt').read_text())*a.GAIN
    new=a.conv(force,'driven')*scale;res=x[:,:2].astype(float)-x[:,2:];oldboard=x[:,2:].astype(float)
    # Room-free endpoint comparison with one onset for all paths.
    onset=c.b.onset(x[:,:2]);stages[n][v]={tag:attack_at(y,n,onset,.06) for tag,y in [('old_board',oldboard),('new_board',new),('residual',res),('old_total',x[:,:2]),('new_total',new+res)]}
 c.save(P/'diagnostic.json',dict(absolute_and_relative=d,window_sensitivity=windows,stage_components=stages,interpretation='Linear propagation affects every dynamic. F# pp combines a pre-existing weak P3 with increased P1 and reduced P3; C8 P2 actually gains in absolute level. Component comparisons share one onset; separate components are coherent, not additive powers.'))
 for n,vv in d.items():
  for v,m in vv.items():
   x=m['balanced'];y=m['driven'];print(n,v,'P1 delta',y['p1']-x['p1'],'bands old/new',x['bands'],y['bands'],flush=True)
if __name__=='__main__':main()
