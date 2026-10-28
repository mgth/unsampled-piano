"""Causal FIR prototype: delayed Rayleigh kernels after mechanical modal IIRs.
All 16 bridge regions; no spectral inversion or magnitude-only replacement.
"""
from pathlib import Path
import sys,json
import numpy as np,pandas as pd
from scipy.signal import resample_poly,firwin
from scipy.fft import rfft,irfft
R=Path(__file__).resolve().parents[2];sys.path.insert(0,str(R/'tools'));import contact_voicing as c
P=R/'bench/rayleigh-audio';O=R/'out/rayleigh-audio';SR=48000

def make(folder):
 nm,ears,nh,os=map(int,(folder/'shape.txt').read_text().split());m=pd.read_csv(folder/'modes.csv');assert len(m)==nm
 h=np.fromfile(folder/'kernels.f64',dtype='<f8').reshape(nm,ears,nh)
 fir=firwin(2*32*os+1,1/os,window=('kaiser',8.6));kernel=resample_poly(h,1,os,axis=-1,window=fir)*os
 # Physical travel time exceeds interpolation support: no anticipatory taps.
 assert np.max(abs(kernel[...,:20]))<1e-15
 N=1<<int(np.ceil(np.log2(SR*np.log(1e14)/m.sigma.min()+kernel.shape[-1])))
 w=2*np.pi*np.arange(N//2+1)/N;z=np.exp(-1j*w);D=(1-z)*SR
 inputs=m[[f'input{r}' for r in range(16)]].to_numpy()
 transfers={n:np.zeros((16,2,len(w)),complex) for n in ['current','driven']}
 for lo in range(0,nm,64):
  hi=min(nm,lo+64);s=m.iloc[lo:hi];pole=np.exp((-s.sigma.to_numpy()+2j*np.pi*s.frequency.to_numpy())/SR)[:,None]
  pos=.5/(1-pole*z);neg=.5/(1-pole.conj()*z)
  weight=(1+1j*s.sigma.to_numpy()/(2*np.pi*s.frequency.to_numpy()))[:,None]
  velocity=weight*pos+weight.conj()*neg-.5*weight.real
  for ear in range(2):
   rad=rfft(kernel[lo:hi,ear],n=N,axis=-1)
   correct=velocity*D*rad
   wl=(s[['pl_re','pr_re'][ear]].to_numpy()+1j*s[['pl_im','pr_im'][ear]].to_numpy())[:,None]
   old=(wl*pos+wl.conj()*neg-.5*wl.real)*D
   transfers['driven'][:,ear]+=inputs[lo:hi].T@correct
   transfers['current'][:,ear]+=inputs[lo:hi].T@old
  print(folder.name,'modes',hi,'/',nm,flush=True)
 impulses={k:irfft(v,n=N,axis=-1) for k,v in transfers.items()}
 # With min damping >36/s, response tails at N are negligible. Report,
 # rather than window away, any residual periodic or truncation error.
 report=dict(samples=N,duration=N/SR,minimum_sigma=float(m.sigma.min()),pole_tail_bound=float(np.exp(-m.sigma.min()*(N-kernel.shape[-1])/SR)),delay_oversampling=os,spatial_step=folder.name,interpolation_halfwidth=32,interpolation_kaiser_beta=8.6,tail_peak_relative={k:float(np.max(abs(v[...,-1024:]))/np.max(abs(v))) for k,v in impulses.items()},kernel_sum_error=float(np.max(abs(kernel.sum(axis=-1)-h.sum(axis=-1)))))
 np.savez(folder/'responses.npz',**impulses,kernel=kernel)
 c.save(P/f'{folder.name}-filter-check.json',report)
 print(report,flush=True)
if __name__=='__main__':make(Path(sys.argv[1]) if len(sys.argv)>1 else O/'kernel5')
