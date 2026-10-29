"""Fresh default/geometry exports and kernels, never reuse historical complete presets."""
from common import *
import shutil,struct

def main():
 BIN.mkdir(parents=True,exist_ok=True)
 for name in ['unsampled-piano','unsampled-piano-render','piano-survey','examples/contact_batch','examples/top_probe','examples/rayleigh_kernel','examples/rayleigh_capture','examples/rayleigh_live']:
  shutil.copy2(R/'target/release'/name,BIN/Path(name).name)
 fingerprint=run([BIN/'rayleigh_live','--fingerprint']).stdout.strip()
 run([BIN/'unsampled-piano-render','dump-preset',O/'compiled-default.toml'])
 current=tomllib.loads((O/'compiled-default.toml').read_text());old=tomllib.loads((R/'bench/main-revalidation/compiled-default.toml').read_text());file=tomllib.loads((R/'presets/default.toml').read_text())
 save(P/'defaults.json',dict(changes=changes(old,current),delivered_file_difference=changes(current,file)))
 spec=importlib.util.spec_from_file_location('rayleigh_filters',R/'bench/rayleigh-audio/filters.py');f=importlib.util.module_from_spec(spec);spec.loader.exec_module(f);f.P=P
 for geom in GEOMS:
  d=O/geom;d.mkdir(exist_ok=True)
  listen=b.listening_args() if geom=='close' else b.listening_args([R/'presets/listening/Microphones/player.toml',R/'presets/listening/Room/studio.toml'])
  run([BIN/'unsampled-piano-render','dump-preset',d/'base.toml',*listen,*b.playback_args()]);base=(d/'base.toml').read_text()
  for n in NOTES:apply_contact(base,n,d/f'n{n}.toml')
  sb=tomllib.loads(base)['soundboard'];assert sb['scatter']==0 and sb['rib_spacing']==0 and sb['bridge_beam']==0
  folder=O/f'kernel-{geom}'
  print('kernel',geom,flush=True);run([BIN/'rayleigh_kernel',d/'base.toml',folder,.005,32]);f.make(folder)
  ir=np.load(folder/'responses.npz')['driven'];gain=10**(sb['mic_gain_db']/20)
  assert ir.shape==(16,2,65536)
  (d/'responses.f64').write_bytes(b'RAYL002\n'+struct.pack('<4I',48000,16,2,65536)+(ir*gain).astype('<f8').tobytes())
  label='pianiste, couvercle 35°, studio' if geom=='player_actual' else 'micros rapprochés, studio'
  lines=[f'label = "Upstream 4a00f00 · Rayleigh intégral · contacts retenus · {label}"',f'physics_fingerprint = "{fingerprint}"','base = "base.toml"','impulses = "responses.f64"']
  for n in NOTES:lines+=['','[[contacts]]',f'note = {n}',f'preset = "n{n}.toml"']
  (d/'bundle.toml').write_text('\n'.join(lines)+'\n')
  save(P/f'{geom}-bundle.json',dict(physics_fingerprint=fingerprint,contacts=CONTACT,files={p.name:sha(p) for p in d.iterdir() if p.is_file()}))
  print('bundle',geom,flush=True)
 c.PROBE=BIN/'top_probe';params=c.note_parameters(O/'close/base.toml',NOTES);save(P/'note-parameters.json',params)
 save(P/'manifest.json',dict(upstream=run(['git','rev-parse','origin/main']).stdout.strip(),head=run(['git','rev-parse','HEAD']).stdout.strip(),physics_fingerprint=fingerprint,binaries={p.name:sha(p) for p in BIN.iterdir()},reference_files={str(p.relative_to(R)):sha(p) for p in [R/'bench/references.json',R/'bench/main-revalidation/references.json',R/'tools/bench.py',R/'tools/plath_set.py']},source_files={str(p.relative_to(R)):sha(p) for p in (R/'crates/piano-core/src').glob('*.rs')}))
if __name__=='__main__':main()
