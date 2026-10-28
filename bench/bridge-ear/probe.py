"""Synchronous, stereo-power H1 stage diagnostics. No note normalization.
Same bins for bridge acceleration and ear pressure; raw SNR/coherence retained.
"""
from pathlib import Path
import sys,json,shutil,subprocess,io
from concurrent.futures import ThreadPoolExecutor
import numpy as np,pandas as pd
from scipy.signal import resample_poly
R=Path(__file__).resolve().parents[2];sys.path.insert(0,str(R/'tools'));sys.path.insert(0,str(Path(__file__).parent))
import contact_voicing as c
from preset_fields import patch
P=R/'bench/bridge-ear';O=R/'out/bridge-ear';O.mkdir(exist_ok=True);(O/'bin').mkdir(exist_ok=True)
RENDER=R/'out/main-revalidation/bin/unsampled-piano-render';BASE=R/'out/fsharp-contact/validation/presets/balanced/n102.toml'
PAR=json.load(open(R/'bench/main-revalidation/note-parameters.json'))['102'];k=np.arange(1,4);FM=PAR['f0']*k*np.sqrt(1+PAR['B']*k*k)
survey=pd.read_csv(R/'out/bench/refs/plath.survey.csv');s=survey[(survey.midi==102)&(survey.layer==13)];FR=np.array([float(s[s.k==k].iloc[0].freq) if len(s[s.k==k]) else s.iloc[0].f0*k*np.sqrt(1+s.iloc[0].B*k*k) for k in [1,2,3]])
WINDOWS={'attack':[0,.06],'early':[.03,.12],'body':[.08,.2],'late':[.2,.5]}
CASES={'player':{},'front_only':{'soundboard.back_gain':0},'grid10':{'soundboard.grid_step':.01},'grid75':{'soundboard.grid_step':.0075},'grid5':{'soundboard.grid_step':.005},'bare_board':{'frame.enabled':False,'rim.enabled':False,'sympathetic.enabled':False,'noise.action':0,'noise.keybed':0},'long_outline':{'soundboard.outline_below':3000},'close':{}}

def analyse(acc,pressure,freqs):
 x=np.column_stack([acc,pressure]);x=x-x.mean(axis=0);on=c.b.onset(pressure);out={}
 for window,(t0,t1) in WINDOWS.items():
  seg=x[on+round(t0*48000):on+round(t1*48000)];W=1024;hop=128;idx=np.arange(W)[None,:]+hop*np.arange((len(seg)-W)//hop+1)[:,None]
  X=np.fft.rfft(seg[idx]*np.hanning(W)[None,:,None],axis=1);Pa=np.mean(abs(X[:,:,0])**2,axis=0);Pp=np.mean(abs(X[:,:,1:])**2,axis=0);Sap=np.mean(np.conj(X[:,:,0,None])*X[:,:,1:],axis=0)
  f=np.fft.rfftfreq(W,1/48000);H=Sap/(Pa[:,None]+1e-30);coh=abs(Sap)**2/(Pa[:,None]*Pp+1e-30)
  rows=[]
  for freq in freqs:
   width=max(2*48000/W,.015*freq);sel=abs(f-freq)<width;j=np.flatnonzero(sel)[np.argmax(Pp[sel].mean(axis=1))]
   ix=np.arange(max(1,j-1),min(len(f),j+2));noise=(abs(f-freq)>2*width)&(abs(f-freq)<4*width)
   # Peak SNR uses the same common peak; H1 weights each bin by input power.
   weights=Pa[ix]/Pa[ix].sum();hp=np.sum(weights[:,None]*abs(H[ix])**2,axis=0)
   snra=10*np.log10(Pa[j]/(np.median(Pa[noise])+1e-30));snrp=10*np.log10(Pp[j].mean()/(np.median(Pp[noise].mean(axis=1))+1e-30))
   coherence=(np.sum(Pa[ix,None]*coh[ix],axis=0)/Pa[ix].sum()).tolist()
   rows.append(dict(frequency=float(f[j]),H1_db=float(10*np.log10(hp.mean()+1e-30)),H1_ears_db=(10*np.log10(hp+1e-30)).tolist(),coherence=coherence,snr_acc=float(snra),snr_pressure=float(snrp),eligible=bool(snra>=12 and snrp>=12 and min(coherence)>=.8),pressure_db=float(10*np.log10(Pp[ix].mean(axis=1).sum()+1e-30)),acc_db=float(10*np.log10(Pa[ix].sum()+1e-30))))
  out[window]=dict(partials=rows,H31_db=rows[2]['H1_db']-rows[0]['H1_db'],eligible=rows[0]['eligible'] and rows[2]['eligible'],onset_sample=int(on))
 return out

def prepare(name,fields):
 p=O/'presets'/f'{name}.toml';p.parent.mkdir(exist_ok=True)
 args=[RENDER,'dump-preset',p,'--preset',BASE]
 if name!='close':args+=['--listening',R/'presets/listening/Microphones/player.toml']
 c.run(args);p.write_text(patch(p.read_text(),{'room.enabled':False,'limiter':0,'soundboard.lid_angle':0,**fields}));return p

def one(item):
 name,fields=item;p=prepare(name,fields);d=O/'signals';d.mkdir(exist_ok=True);prefix=d/name
 c.run([O/'bin/bridge_signal',p,prefix,'--note',102,'--velocity',100,'--hold',1.2,'--tail',.1])
 v=np.fromfile(str(prefix)+'_bridge.f32',dtype=np.float32).astype(float);a=np.diff(v,prepend=0)*48000;ears=np.fromfile(str(prefix)+'_mics.f32',dtype=np.float32).astype(float).reshape(-1,2)
 result=analyse(a,ears,FM)
 fs=sorted(set(FM.tolist()+FR.tolist()+list(np.arange(8500,9901,50))))
 table=c.run([O/'bin/ear_transfer',p,102,','.join(map(str,fs)),'region']).stdout;(O/f'{name}-transfer.csv').write_text(table)
 if name in ['player','grid5']:(O/f'{name}-key-transfer.csv').write_text(c.run([O/'bin/ear_transfer',p,102,','.join(map(str,fs)),'key']).stdout)
 print(name,{w:round(x['H31_db'],2) for w,x in result.items()},flush=True);return name,result

def main():
 for name in ['bridge_signal','ear_transfer']:shutil.copy2(R/'target/release/examples'/name,O/'bin'/name)
 real={}
 for file in sorted(Path('/mnt/local/SSD_B-CT4000/Piano/concert-piano-plath').glob('D1_PROD08_82_*M*.csv')):
  x=np.loadtxt(file,delimiter=';',skiprows=1,usecols=(2,3,4));x=resample_poly(x,24,25,axis=0);real[file.stem]=analyse(x[:,0],x[:,1:],FR)
 c.save(P/'real-transfers.json',real);c.save(P/'diagnostic-design.json',dict(cases=CASES,windows=WINDOWS,model_frequencies=FM,real_frequencies=FR,welch_samples=1024,hop=128,minimum_snr=12,minimum_coherence=.8))
 with ThreadPoolExecutor(3) as ex:models=dict(ex.map(one,CASES.items()))
 c.save(P/'model-transfers.json',models)
 for name,rows in real.items():print(name,{w:(round(x['H31_db'],2),x['eligible'],[round(p['snr_pressure'],1) for p in x['partials']]) for w,x in rows.items()},flush=True)
if __name__=='__main__':main()
