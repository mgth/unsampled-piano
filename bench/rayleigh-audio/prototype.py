"""Render one isolated note with the experimental offline radiation chain.
Requires the frozen presets and kernels built by this study; no app/default edit.
"""
from audio import *
from guard_search import guarded
import argparse

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--note',type=int,choices=range(93,109),default=102);p.add_argument('--velocity',type=int,choices=range(28,128),default=100);p.add_argument('--variant',choices=['control','driven','guarded'],default='guarded');p.add_argument('--out',type=Path,required=True);args=p.parse_args()
 preset=OLD/'presets/balanced'/f'n{args.note}.toml';folder=O/'single'/f'n{args.note}v{args.velocity}'
 c.run([BIN,'capture',preset,folder,args.note,args.velocity,1.1,.1]);dry,new,check=stages(folder,args.note,args.velocity)
 y=dry if args.variant=='control' else new
 if args.variant=='guarded':
  g=json.load(open(P/'guard-selected.json'));y=guarded(dry,new,args.velocity,g['maximum'],g['end'],g['start'],g['shape']) if args.note==g['note'] else dry
 save_audio(preset,y,args.out);print(args.out.resolve());print('Reconstruction check:',check)
if __name__=='__main__':main()
