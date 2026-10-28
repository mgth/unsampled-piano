"""Render an isolated note with the Do8 energy candidate and frozen F-sharp."""
from support import *
import argparse,shutil

def main():
 p=argparse.ArgumentParser();p.add_argument('--profile',choices=['recovered','recovered_mf'],default='recovered');p.add_argument('--note',type=int,choices=range(93,109),default=108);p.add_argument('--velocity',type=int,choices=range(28,128),required=True);p.add_argument('--out',type=Path,required=True);args=p.parse_args();ps=json.load(open(P/'selected.json'))['profiles'];e=ps[args.profile].get(str(args.note),prev.entry(args.note));prev.render(args.note,args.profile,e,vel=[args.velocity],hold=1.1,tail=.1,root=O/'manual');args.out.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(O/'manual'/args.profile/f'n{args.note}'/f'n{args.note}v{args.velocity}.wav',args.out)
if __name__=='__main__':main()
