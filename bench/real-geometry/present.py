from run import *
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from scipy.io import wavfile
x=json.load(open(OUT/'summary.json'));dense=json.load(open(OUT/'dense-models.json'))
fig,axes=plt.subplots(2,2,figsize=(13,8),layout='constrained')
for ax,(n,k,label) in zip(axes.flat,[(97,2,'Do♯7 · P2'),(99,3,'Ré♯7 · P3'),(102,3,'Fa♯7 · P3'),(108,2,'Do8 · P2')]):
 for g,col,lab in [('rect','#737780','Rectangle'),('outline','#168397','Contour seul'),('calibrated','#c5752b','Contour + calibration')]:
  a=dense[g+'_local'];ax.plot(range(28,128),[a[f'{n}:{v}']['partials'][k-2] for v in range(28,128)],color=col,lw=2,label=lab)
 for v in [45,56,100]:
  rs=[a[f'{n}:{v}']['partials'][k-2] for a in REFS.values() if f'{n}:{v}' in a and a[f'{n}:{v}']['partials'][k-2] is not None]
  ax.scatter(np.full(len(rs),v)+np.linspace(-1,1,len(rs)),rs,s=20,color='#444',alpha=.4)
  if rs:ax.scatter([v],[np.mean(rs)],marker='D',s=35,color='black')
 ax.set(title=label,xlabel='Vélocité MIDI',ylabel=f'P{k} / P1 (dB)',xlim=(27,128),ylim=((-82,-15) if k==3 else (-58,-10)));ax.grid(alpha=.18)
axes[0,0].legend(fontsize=9,loc='lower right')
fig.suptitle('Même profil de contact local, trois géométries/calibrations\n349 observations réelles figées · stéréo en puissance · 100 vélocités par courbe',fontsize=14)
fig.savefig(OUT/'geometry_transfer.png',dpi=150)
for name in ['rect_local','outline_base','outline_guarded','outline_local','calibrated_local']:
 clips=[]
 for n in [97,99,102,108]:
  for v in [45,56,100]:
   a=b.load(str(OUT/'attack'/name/f'n{n}v{v}.wav'),stereo=True)[:52800];clips.extend([a,np.zeros((14400,2))])
 wavfile.write(OUT/f'listen-{name}.wav',b.SR,np.concatenate(clips).astype('float32'))
# Preserve exactly which observations were compared, including the numeric target.
previous=json.load(open(OLD/'summary-final.json'))['coverage_rows']
def support(rows):
 return sorted((r['source'],r['note'],r['velocity'],r['k'],r['target']) for r in rows if r['note']>=96)
assert support(previous)==support(x['attack_rows'])
sust=json.load(open(OLD/'sustain-check.json'))['coverage_rows'];assert support(sust)==support(x['sustain_rows'])
c.save(OUT/'coverage.json',dict(attack_rows=len(support(x['attack_rows'])),sustain_rows=len(support(x['sustain_rows'])),attack_identical=True,sustain_identical=True,attack_support_sha256=hashlib.sha256(json.dumps(support(previous)).encode()).hexdigest(),sustain_support_sha256=hashlib.sha256(json.dumps(support(sust)).encode()).hexdigest()))
print('Coverage identical; plot and listening files ready')
