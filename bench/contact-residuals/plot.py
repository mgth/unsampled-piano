from search import *
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
models=json.load(open(OUT/'validation-models.json'));odd=json.load(open(ROOT/'bench/contact-voicing/odd-check.json'))
for n in ['base','guarded']:models[n].update(odd[n])
fig,axes=plt.subplots(2,2,figsize=(13,8),layout='constrained')
for ax,(n,i,title) in zip(axes.flat,[(97,0,'Do♯7 · P2'),(99,1,'Ré♯7 · P3'),(102,1,'Fa♯7 · P3'),(108,0,'Do8 · P2 (contrôle inchangé)')]):
 for name,col,lab,sty in [('guarded','#9b6d48','Profil précédent','-'),('geometry','#a5a8b2','Meilleur score, creux rejetés',':'),('local','#147d92','Candidat local','-')]:
  a=models[name];vels=sorted(int(k.split(':')[1]) for k in a if k.startswith(f'{n}:'));ax.plot(vels,[a[f'{n}:{v}']['partials'][i] for v in vels],sty,color=col,label=lab,lw=2)
 for v in [45,56,100]:
  values=[r[f'{n}:{v}']['partials'][i] for r in REFS.values() if f'{n}:{v}' in r and r[f'{n}:{v}']['partials'][i] is not None]
  ax.scatter(np.full(len(values),v)+np.linspace(-1.2,1.2,len(values)),values,s=22,color='#666666',alpha=.65,zorder=3)
  if values:ax.scatter([v],[np.mean(values)],s=46,marker='D',color='#222222',zorder=4)
 ax.set(title=title,xlabel='Vélocité MIDI',ylabel='Niveau relatif à P1 (dB)',xlim=(27,128),ylim=((-85,-15) if i==1 else (-60,-10)))
 ax.grid(alpha=.17)
axes[0,0].legend(loc='lower right',fontsize=8)
fig.suptitle('Contact marteau–cordes : progrès ciblés et contrôle de la dynamique\n349 observations réelles figées · stéréo en puissance · attaque 0–60 ms',fontsize=14)
fig.savefig(OUT/'contact_residuals.png',dpi=150)
# Explicit evidence of the legacy P4 coverage artifact.
rows=[]
for n in [96,99,102,105,108]:
 for name in ['guarded','geometry','geometry_heights','regular']:
  d=ROOT/'out/contact-voicing/full/guarded/notes' if name=='guarded' else OUT/'full'/name/'notes';file=c.b.find(str(d),n,8);x=c.b.load(file,stereo=True)
  p=PARAMS[str(n)];rows.append(dict(note=n,profile=name,v2=c.b.partials(x,n,0,.06,[2,3,4,6]),physical=c.measure_model(file,p)['partials'],physical_p4_hz=p['f0']*4*np.sqrt(1+p['B']*16)))
c.save(OUT/'legacy-coverage-audit.json',rows)
