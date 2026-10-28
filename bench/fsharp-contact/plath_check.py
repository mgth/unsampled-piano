"""Independent diagnostic, not fitted: real and model pressure/bridge-acceleration
P3/P1 at the attack. Same 60 ms power-peak analysis, each signal onset-aligned.
Retain SNR and all five takes; weak bridge partials cannot support an attribution.
"""
from pathlib import Path
import sys,json
import numpy as np,pandas as pd
from scipy.io import wavfile
from scipy.signal import resample_poly
R=Path(__file__).resolve().parents[2];sys.path.insert(0,str(R/'tools'));import contact_voicing as c
P=R/'bench/fsharp-contact';O=R/'out/fsharp-contact'
survey=pd.read_csv(R/'out/bench/refs/plath.survey.csv');s=survey[(survey.midi==102)&(survey.layer==13)];f=[float(s[s.k==k].iloc[0].freq) if len(s[s.k==k]) else float(s.iloc[0].f0*k*np.sqrt(1+s.iloc[0].B*k*k)) for k in [1,2,3]]+[float('nan')]
raw=Path('/mnt/local/SSD_B-CT4000/Piano/concert-piano-plath');rows=[]
for file in sorted(raw.glob('D1_PROD08_82_*M*.csv')):
 a=np.loadtxt(file,delimiter=';',skiprows=1,usecols=(2,3,4));a=resample_poly(a,24,25,axis=0)
 acc=a[:,0]-np.mean(a[:,0]);pressure=a[:,1:]-np.mean(a[:,1:],axis=0)
 A=c.attack(acc,f);Q=c.attack(pressure,f)
 rows.append(dict(take=file.name,acceleration=A,pressure=Q,transfer_p3_re_p1=Q['partials'][1]-A['partials'][1],eligible=A['snr'][1]>=12 and Q['snr'][1]>=12))
p=json.load(open(R/'bench/main-revalidation/note-parameters.json'))['102'];k=np.arange(1,5);mf=p['f0']*k*np.sqrt(1+p['B']*k*k)
models={}
for name in ['base','local','selected_selected']:
 sr,a=wavfile.read(O/'diagnostics'/name/'v100.wav');acc=np.r_[0,np.diff(a[:,3].astype(float))]*sr
 A=c.attack(acc,mf);Q=c.attack(a[:,:2].astype(float),mf)
 models[name]=dict(acceleration=A,pressure=Q,transfer_p3_re_p1=Q['partials'][1]-A['partials'][1],eligible=A['snr'][1]>=12 and Q['snr'][1]>=12)
c.save(P/'plath-stage-check.json',dict(reference_frequencies=f,real_takes=rows,models=models,used_for_fitting=False,limitation='Attack peak ratios are a stage diagnostic, not a calibrated transfer estimate; different instruments and microphone positions.'))
for r in rows:print(r['take'],'P3 pressure/acceleration',r['pressure']['partials'][1],r['acceleration']['partials'][1],'SNR',r['pressure']['snr'][1],r['acceleration']['snr'][1],'transfer',r['transfer_p3_re_p1'],r['eligible'])
for name,r in models.items():print(name,'P3 pressure/acceleration',r['pressure']['partials'][1],r['acceleration']['partials'][1],'transfer',r['transfer_p3_re_p1'])
