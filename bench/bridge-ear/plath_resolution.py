"""Resolution and peak-identity checks, diagnostic only; no score support changes."""
from pathlib import Path
import sys,json
import numpy as np
from scipy.signal import resample_poly,find_peaks
R=Path(__file__).resolve().parents[2];sys.path.insert(0,str(R/'tools'));import contact_voicing as c
P=R/'bench/bridge-ear';RAW=Path('/mnt/local/SSD_B-CT4000/Piano/concert-piano-plath');SR=48000

def check(x,W,t0,t1,targets):
 on=c.b.onset(x[:,1:]);seg=x[on+round(t0*SR):on+round(t1*SR)];hop=W//4
 ix=np.arange(W)[None,:]+hop*np.arange((len(seg)-W)//hop+1)[:,None]
 X=np.fft.rfft(seg[ix]*np.hanning(W)[None,:,None],axis=1)
 pa=np.mean(abs(X[:,:,0])**2,axis=0);pp=np.mean(abs(X[:,:,1:])**2,axis=0);sap=np.mean(np.conj(X[:,:,0,None])*X[:,:,1:],axis=0)
 coh=abs(sap)**2/(pa[:,None]*pp+1e-30);H=sap/(pa[:,None]+1e-30);f=np.fft.rfftfreq(W,1/SR)
 rows=[]
 for target,width in targets:
  sel=np.flatnonzero(abs(f-target)<width);j=sel[np.argmax(pp[sel].mean(axis=1))]
  noise=(abs(f-target)>200)&(abs(f-target)<450)
  rows.append(dict(target=target,peak_hz=f[j],frames=len(ix),coherence=coh[j],H1_db=10*np.log10(np.mean(abs(H[j])**2)+1e-30),pressure_snr=10*np.log10(pp[j].mean()/(np.median(pp[noise].mean(axis=1))+1e-30)),acc_snr=10*np.log10(pa[j]/(np.median(pa[noise])+1e-30))))
 return rows

def spectra(x):
 on=c.b.onset(x[:,1:]);seg=x[on:on+round(.12*SR)];N=131072
 X=np.fft.rfft(seg*np.hanning(len(seg))[:,None],n=N,axis=0);f=np.fft.rfftfreq(N,1/SR)
 powers=[abs(X[:,0])**2,np.mean(abs(X[:,1:])**2,axis=1)];out={}
 for label,power in zip(['acceleration','ears'],powers):
  peaks=find_peaks(power)[0];peaks=peaks[(f[peaks]>8500)&(f[peaks]<10200)];peaks=sorted(peaks,key=lambda j:power[j],reverse=True)[:6]
  out[label]=[dict(hz=f[j],db=10*np.log10(power[j])) for j in peaks]
 return out

def main():
 result={};near={};targets=[(3015,80),(6033,100),(9055,140),(9093,25),(9670,30)]
 for file in sorted(RAW.glob('D1_PROD08_82_*M*.csv')):
  x=resample_poly(np.loadtxt(file,delimiter=';',skiprows=1,usecols=(2,3,4)),24,25,axis=0);x-=x.mean(axis=0)
  result[file.stem]={f'{W}:{t0}:{t1}':check(x,W,t0,t1,targets) for W in [1024,2048,4096] for t0,t1 in [(0,.12),(.03,.2),(0,.3)]}
  near[file.stem]=spectra(x)
 for note in [100,101,103,104]:
  file=next(RAW.glob(f'D1_PROD08_{note-20:02d}_*M0002.csv'));x=resample_poly(np.loadtxt(file,delimiter=';',skiprows=1,usecols=(2,3,4)),24,25,axis=0);near[file.stem]=spectra(x)
 c.save(P/'plath-resolution.json',dict(transfers=result,spectral_peaks=near,policy='Diagnostic only. Same pressure-selected frequency bin in both sensors, no independent onset or peak matching. Coherence with few overlapping frames is optimistic, not independent-replicate confidence. Third survey band is a proxy, not an identified modal family.'))
 for W in [1024,2048,4096]:
  print('W',W)
  for i in [2,3,4]:
   a=[r[f'{W}:0.03:0.2'][i] for r in result.values()];print(i,[(round(r['peak_hz']),np.round(r['coherence'],2).tolist(),round(r['pressure_snr'],1)) for r in a])
 print('peaks',json.dumps(c.clean(near),indent=1))
if __name__=='__main__':main()
