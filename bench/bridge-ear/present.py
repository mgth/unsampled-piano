"""Archive diagnostics, calculate compact deltas and plot frozen-support results."""
from pathlib import Path
import sys,json,hashlib,subprocess
import numpy as np,pandas as pd
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
R=Path(__file__).resolve().parents[2];sys.path.insert(0,str(R/'tools'));import contact_voicing as c
P=R/'bench/bridge-ear';O=R/'out/bridge-ear'
def main():
 s=json.load(open(P/'validation-summary.json'));m=json.load(open(P/'validation-models.json'));dense=json.load(open(P/'validation-dense.json'));design=json.load(open(P/'diagnostic-design.json'));fs=design['model_frequencies']
 analytic={};compact={}
 for path in sorted(O.glob('*-transfer.csv'))+sorted(O.glob('*-driven.csv')):
  d=pd.read_csv(path);analytic[path.stem]=d.to_dict(orient='records');a=d[(d.band=='all')&d.frequency.apply(lambda f:any(np.isclose(f,x) for x in fs))]
  h=10*np.log10((a.pl_re**2+a.pl_im**2+a.pr_re**2+a.pr_im**2)/2/(a.a_re**2+a.a_im**2))
  compact[path.stem]=dict(H_db=h.tolist(),H31_db=float(h.iloc[2]-h.iloc[0]))
 c.save(P/'analytic-transfers.json',analytic)
 residual={};deltas={}
 for name in m:
  residual[name]={};deltas[name]={}
  for v in [45,56,100]:
   rows=[r for r in s['attack_rows'] if r['note']==102 and r['velocity']==v and r['k']==3]
   residual[name][v]=dict(mean_error=float(np.mean([r['values'][name]-r['target'] for r in rows])),rows=len(rows))
  for n in [97,99,102,108]:
   deltas[name][n]={}
   for v in [45,56,100]:
    a=m[name][f'{n}:{v}'];b=m['balanced'][f'{n}:{v}']
    deltas[name][n][v]=dict(partials=[None if x is None else x-y for x,y in zip(a['partials'],b['partials'])],p1=a['p1']-b['p1'],level=a['level']-b['level'],absolute_partials=[None if x is None else (x+a['p1'])-(y+b['p1']) for x,y in zip(a['partials'],b['partials'])])
 c.save(P/'findings.json',dict(analytic=compact,fsharp_P3_residual=residual,guard_deltas=deltas,coverage=dict(attack=459,top_attack=349,transition=110,sustain=71,dense_keys=[97,99,102,108],velocities=list(range(28,128))),decision='Reject rear-gain ablations: pp darkens and C8 error grows. Driven Rayleigh is diagnostic only, not a rendered/validated replacement. Default unchanged.'))
 fig,ax=plt.subplots(2,2,figsize=(13,8),layout='constrained');colors=['#2878a8','#c68a20','#be4848'];labels=['Contact équilibré, arrière 0,6','Arrière 0,3','Arrière 0']
 for name,col,label in zip(m,colors,labels):
  for panel,n,i in [(ax[0,0],102,1),(ax[0,1],108,0)]:panel.plot(range(28,128),[dense[name][f'{n}:{v}']['partials'][i] for v in range(28,128)],color=col,label=label)
 ax[0,0].set(title='Fa♯7 : P3/P1 — les creux restent maîtrisés',ylabel='dB / P1',xlabel='Vélocité');ax[0,1].set(title='Do8 : P2/P1 — déplacement indésirable',ylabel='dB / P1',xlabel='Vélocité');ax[0,0].legend(fontsize=8)
 cats=['P1','P2','P3'];x=np.arange(3)
 for label,key,col in [('Moteur, joueur','grid5-transfer','#2878a8'),('Intégrale à la fréquence jouée, joueur','grid5-driven','#be4848')]:ax[1,0].plot(x,compact[key]['H_db'],'o-',label=label,color=col)
 ax[1,0].set(xticks=x,xticklabels=cats,title='Transmission absolue — grille 5 mm',ylabel='dB : Pa par m/s²');ax[1,0].legend(fontsize=8)
 # Contributions are complex, so their powers are not additive.
 for offset,key,label,col in [(-.18,'grid5-transfer','Moteur','#2878a8'),(.18,'grid5-driven','Intégrale à la fréquence jouée','#be4848')]:
  d=pd.DataFrame(analytic[key]);a=d[np.isclose(d.frequency,fs[2])];power=(a.pl_re**2+a.pl_im**2+a.pr_re**2+a.pr_im**2)/2;y=10*np.log10(power.iloc[1:].to_numpy()/power.iloc[0]);ax[1,1].bar(np.arange(5)+offset,y,width=.36,label=label,color=col)
 ax[1,1].set(xticks=np.arange(5),xticklabels=['<1,5k','1,5–4k','4–8k','8–12k','>12k'],title='Contribution des familles modales à P3, joueur',xlabel='Fréquence propre des modes (Hz)',ylabel='dB / somme complexe');ax[1,1].legend(fontsize=8)
 for a in ax.flat:a.grid(alpha=.2)
 fig.savefig(R/'docs/img/bridge_ear_followup.png',dpi=170);plt.close(fig)
 sha=lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
 raw=Path('/mnt/local/SSD_B-CT4000/Piano/concert-piano-plath');sources=sorted(raw.glob('D1_PROD08_82_*M*.csv'))
 sources += [next(raw.glob(f'D1_PROD08_{n-20:02d}_*M0002.csv')) for n in [100,101,103,104]]
 protected=['presets/default.toml','crates/piano-core/src/board.rs','crates/piano-core/src/soundboard.rs','tools/contact_voicing.py','tools/bench.py']
 manifest=dict(base_commit='40afaf2',acoustic_main='c0c91ce',default_changed=False,production_dsp_changed=False,reference_support_changed=False,binaries={str(p.relative_to(R)):sha(p) for p in sorted((O/'bin').iterdir())},batch_sha256=sha(R/'out/main-revalidation/bin/contact_batch'),raw={p.name:sha(p) for p in sources},protected_files={p:sha(R/p) for p in protected},control_checks={p.name:sha(p) for p in (O/'control-check').glob('*.wav')},new_candidate_wavs=896,reused_control_wavs=448,python_tests_passed=9,quadrature_checks={p.stem:p.read_text().strip() for p in O.glob('*-driven.log')},presets={str(p.relative_to(O)):sha(p) for p in sorted(O.rglob('*.toml'))})
 c.save(P/'manifest.json',manifest)
if __name__=='__main__':main()
