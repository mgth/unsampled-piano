"""Render an isolated note with the experimental integral radiator/contact."""
from common import *
import argparse

def main():
 p=argparse.ArgumentParser();p.add_argument('--profile',choices=['contact','contact_pp'],default='contact');p.add_argument('--note',type=int,choices=range(93,109),required=True);p.add_argument('--velocity',type=int,choices=range(28,128),required=True);p.add_argument('--out',type=Path,required=True);args=p.parse_args();profiles=json.load(open(P/'selected.json'))['profiles'];e=profiles[args.profile].get(str(args.note),entry(args.note));render(args.note,args.profile,e,vel=[args.velocity],hold=1.1,tail=.1,root=O/'manual');args.out.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(O/'manual'/args.profile/f'n{args.note}'/f'n{args.note}v{args.velocity}.wav',args.out)
if __name__=='__main__':main()
