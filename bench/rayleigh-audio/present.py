from audio import *
import hashlib,subprocess,tomllib
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

def main():
 s=json.load(open(P/'validation-summary.json'));m=json.load(open(P/'validation-models.json'));dense=json.load(open(P/'validation-dense.json'));names=['balanced','driven','guarded'];player=json.load(open(P/'player-audio.json'))
 residual={};deltas={}
 for name in names:
  residual[name]={};deltas[name]={}
  for v in [45,56,100]:
   rows=[r for r in s['attack_rows'] if r['note']==102 and r['velocity']==v and r['k']==3]
   residual[name][v]=dict(mean=float(np.mean([r['values'][name]-r['target'] for r in rows])),rows=len(rows))
  for n in [97,99,102,108]:
   deltas[name][n]={}
   for v in [45,56,100]:
    a=m[name][f'{n}:{v}'];b=m['balanced'][f'{n}:{v}'];deltas[name][n][v]=dict(p1=a['p1']-b['p1'],partials=[None if x is None else x-y for x,y in zip(a['partials'],b['partials'])],absolute_partials=[None if x is None else (x+a['p1'])-(y+b['p1']) for x,y in zip(a['partials'],b['partials'])],raw_frame_level=a['level']-b['level'])
    def padded_level(which):
     f=(OLD/'attack/balanced' if which=='balanced' else O/'validation/attack'/which)/f'n{n}v{v}.wav'
     _,x=wavfile.read(f);return float(c.b.onset_level(np.pad(x,((1024,0),(0,0)))))
    deltas[name][n][v]['level_with_common_preroll']=padded_level(name)-padded_level('balanced')
 late={}
 for name in names:
  y=np.array([dense[name][f'102:{v}']['partials'][1] for v in range(88,128)]);late[name]=float(max(np.maximum.accumulate(y)-y))
 c.save(P/'findings.json',dict(fsharp_P3_residual=residual,guard_deltas=deltas,late_drawdown=late,level_latency_check=json.load(open(P/'latency-level-check.json')),player=player,decision='Default unchanged. Full physical replacement rejected on pp/C8 guards. Guarded voicing preserves those outputs and improves F# forte modestly; not a physical radiation calibration.'))
 fig,ax=plt.subplots(2,2,figsize=(13,8),layout='constrained');colors=['#2878a8','#be4848','#328760'];labels=['Contrôle équilibré','Rayonnement intégral','Interpolation protégée']
 for name,col,label in zip(names,colors,labels):
  a=[dense[name][f'102:{v}']['partials'][1] for v in range(28,128)];ax[0,0].plot(range(28,128),a,color=col,label=label);ax[0,1].plot(range(28,128),a,color=col,label=label)
  ax[1,0].plot([45,56,100],[residual[name][v]['mean'] for v in [45,56,100]],'o-',color=col,label=label)
  ax[1,1].plot(range(28,128),[dense[name][f'108:{v}']['partials'][0] for v in range(28,128)],color=col,label=label)
 ax[0,0].set(title='Fa♯7 : effet du rayonnement sur P3/P1',xlabel='Vélocité',ylabel='dB / P1');ax[0,0].legend(fontsize=8)
 ax[0,1].set(title='Garde des creux : détail des fortes vélocités',xlabel='Vélocité',ylabel='dB / P1',xlim=(74,108),ylim=(-44,-38.5));ax[0,1].axvline(88,color='#666',ls=':',lw=1);ax[0,1].axvline(100,color='#666',ls=':',lw=1)
 ax[1,0].set(title='Fa♯7 P3 : écart moyen aux références fixes',xticks=[45,56,100],xticklabels=['pp (45)','mf (56)','forte (100)'],ylabel='Écart en dB');ax[1,0].axhline(0,color='#777',ls=':')
 ax[1,1].set(title='Do8 : le prototype intégral déplace P2/P1',xlabel='Vélocité',ylabel='dB / P1');ax[1,1].text(.03,.93,'Contrôle et variante protégée identiques',transform=ax[1,1].transAxes,fontsize=9,va='top')
 for a in ax.flat:a.grid(alpha=.2)
 fig.savefig(R/'docs/img/rayleigh_audio.png',dpi=170);plt.close(fig)
 # Each group: control, guarded, full; same gain for all excerpts.
 clips=[];sequence=[];clock=0
 for n,v in [(102,45),(102,100),(108,100)]:
  for name in ['balanced','guarded','driven']:
   path=(OLD/'attack/balanced' if name=='balanced' else O/'validation/attack'/name)/f'n{n}v{v}.wav';_,x=wavfile.read(path);x=x.astype(float)
   # Common end fade only for the listening montage; metric WAVs untouched.
   fade=min(960,len(x));x[-fade:]*=np.linspace(1,0,fade)[:,None]
   sequence.append(dict(start_seconds=clock/FS,note=n,velocity=v,variant=name));clips.extend([x,np.zeros((14400,2))]);clock+=len(x)+14400
 audio=np.concatenate(clips);gain=min(2.,.9/np.max(abs(audio)));wavfile.write(O/'comparison.wav',FS,(audio*gain).astype(np.float32));c.save(P/'listening.json',dict(file=str(O/'comparison.wav'),common_gain=gain,end_fade_ms=20,sequence=sequence))
 sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
 recon=json.load(open(P/'reconstruction-checks.json'));peaks={}
 for name in ['driven','guarded']:
  peaks[name]=max(float(np.max(abs(wavfile.read(f)[1]))) for f in (O/'validation/attack'/name).glob('*.wav'))
 assert max(peaks.values())<1
 preset=(OLD/'presets/balanced/n102.toml').read_bytes();sb=tomllib.loads(preset.decode())['soundboard']
 assert all(tomllib.loads((OLD/'presets/balanced'/f'n{n}.toml').read_text())['soundboard']==sb for n in range(93,109))
 manifest=dict(base_commit='aac9cb1',branch='codex/piano-main-revalidation',default_changed=False,runtime_render_path_changed=False,core_changes='Two read-only diagnostic accessors: radiation paths and sympathetic force.',binaries={p.name:sha(p) for p in (O/'bin').iterdir()},presets={str(n):sha(OLD/'presets/balanced'/f'n{n}.toml') for n in range(93,109)},fixed_reference_sha256=sha(R/'bench/main-revalidation/references.json'),analysis_sha256=sha(R/'tools/contact_voicing.py'),default_sha256=sha(R/'presets/default.toml'),selected=sha(P/'guard-selected.json'),kernels={d:{f:sha(O/d/f) for f in ['modes.csv','shape.txt','geometry.txt','responses.npz']} for d in ['kernel5','kernel75','kernel-player']},control_bit_identical=sum(len(v) for v in recon.values()),worst_reconstruction_rms_db=max(x['rms_error_db'] for rows in recon.values() for x in rows.values()),candidate_peaks=peaks,core_tests_passed=74,python_tests_passed=9,coverage=dict(attack=459,top=349,transition=110,sustain=71,dense_keys=[97,99,102,108],velocities=list(range(28,128))),guard_bypassed_files=s['guard_equal_files'],level_metric_policy='Historical raw levels retained; level guards additionally use a common 1024-sample preroll for all variants to avoid delay-dependent 10 ms frame-grid bias.')
 c.save(P/'manifest.json',manifest)
 print(json.dumps(c.clean(dict(residual=residual,late=late,peaks=peaks,manifest_summary={k:manifest[k] for k in ['control_bit_identical','worst_reconstruction_rms_db']})),indent=1))
if __name__=='__main__':main()
