from run import *
controls=[]
for voicing in ['base','guarded','local']:
 for n in range(93,109):
  for v in [45,56,100]:
   if voicing=='local' and n in [97,99,102]:old=ROOT/f'out/contact-residuals/grid/{"local" if n==97 else "regular"}-{n}/n{n}v{v}.wav'
   else:old=ROOT/f'out/contact-voicing/validation/{"guarded" if voicing=="local" else voicing}/notes/n{n}v{v}.wav'
   new=OUT/f'attack/rect_{voicing}/n{n}v{v}.wav';x=b.load(str(old),stereo=True)[:9600];y=b.load(str(new),stereo=True)[:9600]
   exact=np.array_equal(x,y);controls.append(dict(voicing=voicing,note=n,velocity=v,exact_200ms=exact,max_delta=float(abs(x-y).max())))
   assert exact,(voicing,n,v,float(abs(x-y).max()))
# Combined playable profile versus independent global contact preset, with the outline enabled.
for n in [93,97,99,102,108]:
 p=OUT/'checks'/f'isolated-{n}.toml';s,extra=spec('local',n);c.write_preset(p,geometry(BASE,'calibrated'),**s)
 text=p.read_text()
 for key,value in extra.items():text=re.sub(rf'^{key} = .*$',f'{key} = {value}',text,flags=re.M)
 p.write_text(text);d=p.parent/f'n{n}';c.run([c.BATCH,p,d,n,'45,56,100',.2,.05])
 for v in [45,56,100]:
  a=d/f'n{n}v{v}.wav';bpath=OUT/f'dense/calibrated_local/n{n}v{v}.wav' if n!=93 else OUT/f'attack/calibrated_local/n{n}v{v}.wav'
  x=b.load(str(a),stereo=True)[:9600];y=b.load(str(bpath),stereo=True)[:9600];exact=np.array_equal(x,y);assert exact,(n,v,abs(x-y).max())
  controls.append(dict(voicing='calibrated_local_isolated',note=n,velocity=v,exact_200ms=exact,max_delta=float(abs(x-y).max())))
c.save(OUT/'controls.json',dict(checks=controls,all_exact=True))
print(len(controls),'exact audio controls')
