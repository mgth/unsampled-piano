"""Cycle-aware H1 measurement with local noise checks and explicit censoring.

Blackman-Harris windows resolve weak bass H1 from much stronger H2/H3.
Zero padding prevents a narrow search band from falling between FFT bins.
All channels contribute powers; they are never summed as waveforms.
"""
import numpy as np
from scipy.signal.windows import blackmanharris
from scipy.fft import rfft,rfftfreq,next_fast_len
import keyboard_common as c
SR=c.s.b.SR
TIMES=[.25,.5,.75,1.1]
SNR_DB=10.

def extract(x,n):
 if x.ndim==1:x=x[:,None]
 f0=c.s.b.f0(n);duration=max(.12,12/f0)
 count=int(round(duration*SR));window=blackmanharris(count)
 fft_len=next_fast_len(count*8)
 freq=rfftfreq(fft_len,1/SR)
 search=(freq>f0*.94)&(freq<f0*1.06)
 flanks=((freq>f0*.4)&(freq<f0*.65))|((freq>f0*1.4)&(freq<f0*1.6))
 energy=np.sqrt(np.mean(x*x,axis=1));onset=c.s.b.onset(energy)
 pitch=f0
 if n<40:
  # Estimate tuning from strong upper harmonics over a longer span. This
  # prevents a weak H1 measurement from following nearby low-frequency noise.
  segment=x[onset+int(.1*SR):onset+int(1.3*SR)]
  if len(segment)>=int(.8*SR):
   nf=next_fast_len(len(segment)*4);frequencies=rfftfreq(nf,1/SR)
   spectrum=np.mean(abs(rfft(segment*blackmanharris(len(segment))[:,None],nf,axis=0))**2,axis=1)
   harmonic_numbers=[];ratios=[]
   for k in range(2,6):
    band=(frequencies>f0*k*.94)&(frequencies<f0*k*1.06)
    j=np.flatnonzero(band)[np.argmax(spectrum[band])]
    if spectrum[j]>spectrum.max()*1e-5:
     harmonic_numbers.append(k*k);ratios.append((frequencies[j]/k)**2)
   if len(ratios)>=3:
    slope,intercept=np.polyfit(harmonic_numbers,ratios,1)
    estimate=np.sqrt(max(0,intercept+slope))
    if .94*f0<estimate<1.06*f0:pitch=float(estimate)
 times=TIMES if n<40 else [.03]+TIMES
 rows=[]
 for centre in times:
  window_duration=.06 if centre==.03 else duration
  count=int(round(window_duration*SR));window=blackmanharris(count)
  fft_len=next_fast_len(count*8);freq=rfftfreq(fft_len,1/SR)
  search=(freq>f0*.94)&(freq<f0*1.06)
  flanks=((freq>f0*.4)&(freq<f0*.65))|((freq>f0*1.4)&(freq<f0*1.6))
  start=onset+int(round((centre-window_duration/2)*SR))
  seg=x[start:start+count]
  if start<0 or len(seg)!=count:
   rows.append(dict(level=None,limit=None,snr=None,frequency=None,reason='fenêtre absente'));continue
  seg=seg-seg.mean(axis=0)
  power=np.mean(abs(rfft(seg*window[:,None],fft_len,axis=0))**2,axis=1)*(2/window.sum())**2
  index=np.flatnonzero(search)[np.argmax(power[search])]
  peak=float(10*np.log10((np.interp(pitch,freq,power) if n<40 else power[index])+1e-30))
  # Median local spectral floor; unlike H1/H2 this tests detectability, not timbre.
  noise=float(10*np.log10(np.median(power[flanks])+1e-30))
  snr=peak-noise
  # Conservative leakage bound, tested with H1 absent and H2/H3 alone.
  limit=max(noise+SNR_DB,float(10*np.log10(power.max()+1e-30))-85,-180)
  valid=peak>=limit
  rows.append(dict(level=peak if valid else None,limit=limit,snr=snr,frequency=pitch if n<40 else float(freq[index]),reason=None if valid else 'H1 sous le seuil local'))
 anchor=rows[0]['level']
 curve=[r['level']-anchor if anchor is not None and r['level'] is not None else None for r in rows]
 return dict(pitch=pitch,times=times,duration=duration,anchor=times[0],curve=curve,windows=rows)

def measure(path,n):return extract(c.s.b.load_stereo(str(path)),n)
