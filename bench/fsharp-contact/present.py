"""Summarize the completed fixed-support experiment; plots and listening preview."""
from pathlib import Path
import json,hashlib,subprocess,shutil,sys
import numpy as np
import matplotlib.pyplot as plt
from scipy.io import wavfile
R=Path(__file__).resolve().parents[2];P=R/'bench/fsharp-contact';O=R/'out/fsharp-contact';V=O/'validation'
s=json.load(open(P/'validation-summary.json'));models=json.load(open(P/'validation-models.json'));dense=json.load(open(P/'validation-dense.json'));prev=json.load(open(R/'bench/main-revalidation/dense-models.json'));refs=json.load(open(R/'bench/main-revalidation/references.json'));diag=json.load(open(P/'diagnostics.json'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
counts={p.stem:len(json.load(open(p))) for p in P.glob('*results.json')}
manifest=dict(engine_base_commit='d0d3fef',synthesis_unchanged=True,production_default_sha256=sha(R/'presets/default.toml'),reference_sha256=sha(R/'bench/main-revalidation/references.json'),binaries={str(p.relative_to(R)):sha(p) for p in [R/'out/main-revalidation/bin/contact_batch',R/'out/main-revalidation/bin/piano-survey',O/'contact_stages']},screen_counts=counts,screen_velocities=27,validation_wavs={x:len(list((V/x).rglob('*.wav'))) for x in ['attack','dense','full']},python_tests_passed=9,probe_compiles=True,probe_build_note='The probe logic was unchanged; source was rustfmt-formatted between initial and player diagnostics. Hash identifies the final formatted build.',source_hashes={str(p.relative_to(R)):sha(p) for p in [R/'crates/piano-render/examples/contact_stages.rs',*P.glob('*.py')]})
(P/'manifest.json').write_text(json.dumps(manifest,indent=1)+'\n')
for src,dst in [('/tmp/piano-fsharp-tests.log','tests.log'),('/tmp/piano-fsharp-build.log','build.log')]:
 (P/dst).write_text(Path(src).read_text().rstrip()+'\n')
# Retain calibrated numbers; listening gains never feed the benchmark.
chunks=[]
for name in ['main','protected','balanced']:
 for vel in [45,56,100]:
  path=R/f'out/main-revalidation/attack/base/n102v{vel}.wav' if name=='main' else V/f'attack/{name}/n102v{vel}.wav'
  sr,a=wavfile.read(path);assert sr==48000;chunks.extend([a,np.zeros((14400,2),np.float32)])
preview=np.concatenate(chunks);gain=min(4.,.95/np.max(np.abs(preview)));wavfile.write(O/'fsharp-comparison.wav',48000,(preview*gain).astype(np.float32))
(P/'preview.json').write_text(json.dumps(dict(file='out/fsharp-contact/fsharp-comparison.wav',order=['main v45 v56 v100','protected v45 v56 v100','balanced v45 v56 v100'],common_gain_db=20*np.log10(gain),per_note_normalization=False),indent=1)+'\n')

fig,axes=plt.subplots(1,2,figsize=(12,4.6),layout='constrained')
for i,ax in enumerate(axes):
 for name,title,color,style in [('main','Défaut main','#8a8a8a','--'),('protected','Contact local précédent','#d08127','-'),('damped','Variante régularité','#9b72ad',':'),('balanced','Variante équilibrée','#208a74','-')]:
  source=prev['base'] if name=='main' else dense[name]
  ax.plot(range(28,128),[source[f'102:{v}']['partials'][i] for v in range(28,128)],label=title,color=color,ls=style,lw=2)
 for v in [45,56,100]:
  a=[r[f'102:{v}']['partials'][i] for r in refs.values() if f'102:{v}' in r and r[f'102:{v}']['partials'][i] is not None]
  ax.errorbar(v,np.mean(a),yerr=np.std(a),fmt='o',color='#222222',capsize=4,label='Pianos : moyenne ± écart-type' if v==45 else None)
 ax.set(title=f'Fa♯7 — P{i+2}',xlabel='Vélocité MIDI',ylabel=f'P{i+2} / P1 (dB)',xlim=(28,127));ax.grid(alpha=.2)
axes[0].legend(fontsize=8,loc='lower right');fig.suptitle('Même couverture de référence — 100 vélocités vérifiées',fontsize=13)
img=R/'docs/img/fsharp_contact_validation.png';fig.savefig(img,dpi=160);plt.close(fig)
fig,axes=plt.subplots(1,2,figsize=(12,4.4),layout='constrained');vv=sorted(map(int,diag['base']))
for i,ax in enumerate(axes):
 for key,label,col in [('audio','Son stéréo','#3576ab'),('bridge_force','Force au chevalet','#d08127'),('bridge_velocity','Vitesse du chevalet','#25886c')]:
  ax.plot(vv,[diag['base'][str(v)][key]['partials'][i] for v in vv],label=label,color=col,lw=2)
 ax.set(title=f'Défaut — P{i+2}',xlabel='Vélocité MIDI',ylabel=f'P{i+2} / P1 (dB)');ax.grid(alpha=.2)
axes[0].legend(fontsize=9);fig.suptitle('Les creux existent avant le rayonnement — mesures de chaque étage',fontsize=13);fig.savefig(R/'docs/img/fsharp_contact_stages.png',dpi=160);plt.close(fig)

names=['protected','damped','balanced']
t=['| Mesure | Contrôle local protégé | Régularité | Équilibrée |','|---|---:|---:|---:|']
for key,label in [('fsharp','MAE Fa♯7, 34 observations (dB)'),('fsharp_sustain','MAE tenue Fa♯7, 12 observations (dB)'),('attack','MAE attaques C7–C8, 349 observations (dB)'),('sustain','MAE tenues, 71 observations (dB)')]:t.append('| '+label+' | '+' | '.join(f"{s[key][n]['mae']:.3f}" for n in names)+' |')
for i in [0,1]:t.append(f'| Creux maximal de P{i+2}, vélocités 28–127 (dB) | '+' | '.join(f"{s['dips'][n]['102'][i]:.2f}" for n in names)+' |')
t.append('| Score secondaire, 191 composantes (RMS z) | '+' | '.join(f"{s['full'][n]['summary']['score']:.3f}" for n in names)+' |')
targets=['| Fa♯7 : écart à la moyenne des pianos | Contrôle local | Régularité | Équilibrée |','|---|---:|---:|---:|']
for v in [45,56,100]:
 for i in [0,1]:
  rr=[r[f'102:{v}']['partials'][i] for r in refs.values() if f'102:{v}' in r and r[f'102:{v}']['partials'][i] is not None];mean=np.mean(rr)
  targets.append(f'| v{v}, P{i+2} (dB) | '+' | '.join(f"{models[n][f'102:{v}']['partials'][i]-mean:+.2f}" for n in names)+' |')
text='''# Fa♯7 forte et creux de vélocité : contact, tige et transmission

28 octobre 2026. Base moteur : `d0d3fef` (main acoustique `c0c91ce` et banc corrigé). **Aucun changement du défaut, du DSP de production ou de la baseline.** Les nouvelles lois sont des expériences par touche rendues hors ligne ; elles ne sont pas chargées globalement dans l’UI.

## Décision

La tige du marteau fortement amortie apporte une réduction reproductible des creux. La variante équilibrée ramène les creux de Fa♯7 P2/P3 à 1,28/1,39 dB, contre 5,64/4,93 pour le précédent contact local et 14,48/6,55 pour le défaut de main (100 vélocités). P3 forte reste trop fort : +8,40 dB contre +9,49 pour le contact local, et +25,16 pour main. Ce progrès de P3 est modeste ; le progrès net est la régularité.

**Pas de promotion.** P3 pianissimo est davantage atténué qu’avec le contact local, la MAE des 349 attaques ne s’améliore pas, et un contrôle indépendant sur Plath montre un écart restant à la fois en amont et dans la transmission vers les oreilles. La cohérence au clavier s’améliore, mais abaisser encore le contact pour corriger tout l’écart acoustique risque d’assombrir le pp.

## Causes isolées avant recherche des réglages

La nouvelle sonde `contact_stages` produit, pour chaque vélocité, les forces individuelles des cordes, leurs états modaux à la fin du contact, et un WAV à quatre canaux : son gauche/droite, force au chevalet et vitesse du chevalet. Le WAV à quatre canaux n’est jamais passé au chargeur stéréo ; ses canaux sont analysés séparément. Les canaux audio sont identiques bit à bit aux rendus normaux aux trois vélocités de référence.

Sur le défaut, le premier balayage donne :

| Partiel | Creux du son | Creux de la force au chevalet | Creux de la vitesse du chevalet |
|---|---:|---:|---:|
| P2 | 14,46 dB | 13,46 dB | 13,44 dB |
| P3 | 6,31 dB | 6,49 dB | 6,47 dB |

La corrélation des courbes audio/force au chevalet dépasse 0,999. Les creux se forment donc avant le rayonnement. Les spectres des forces individuelles et de leur somme montrent aussi l’effet des phases et des durées de contact ; le creux de la somme des forces ne prédit toutefois pas à lui seul la position exacte de tous les creux acoustiques, car les cordes sont ensuite couplées.

![Étages du moteur](img/fsharp_contact_stages.png)

Ablations séparées, sur le défaut, même couverture de vélocités :

- Cordes exactement à la même hauteur : P2 empire, 23,55 dB de creux. Supprimer l’écart de hauteur n’est pas la solution.
- Tige rigide : P2 descend à 10,85 dB de creux, mais P3 forte devient 5,21 dB plus fort. La souplesse contribue au compromis.
- Frappe répartie également : creux P2/P3 de 12,57/3,82 dB ; c’est un facteur secondaire, sans correction suffisante de P3 forte.
- Noyau dur du feutre retiré : effet négligeable ici (P3 forte varie de 0,006 dB). Bruit d’action et limiteur retirés : négligeables sur ces partiels. Sympathie retirée : P3 forte change de 0,25 dB, les creux demeurent.
- Désaccord des cordes retiré : creux de 13,12/9,37 dB ; le désaccord seul ne les explique pas.
- Calcul du contact à 5 puis 20 sous-pas : écart maximal de 0,13 dB pour P2 et 0,30 dB pour P3 aux cinq vélocités contrôlées dans le spectre de la force. Ce contrôle concerne le contact isolé ; le nombre de sous-pas de l’audio de la sonde n’est pas modifié.

Un essai de `bridge.g_1k` n’a rien changé : lecture du code confirmée, cette loi de secours n’est pas utilisée lorsque la table modale fournit l’admittance (`note_model.rs`). Ce résultat **ne démontre pas** que le couplage au chevalet serait sans effet.

## Réglages comparés

278 essais paramétriques, contrôles inclus, sur 27 vélocités ont testé la force du feutre à compression fixée, son exposant et son hystérésis, les hauteurs et la répartition de frappe, la position et la largeur du contact, la masse et la tige du marteau. Les essais qui déplacent simplement un zéro spectral sont conservés dans les résultats mais rejetés pour la validation finale. Les références ont servi à sélectionner les candidats : il ne s’agit pas d’un test aveugle.

Le **contrôle protégé** reprend le précédent profil local, avec Ré♯7 remis au défaut de main. Do♯7 est déjà au défaut, Do8 conserve le contact validé précédemment. Les deux nouveaux profils ne changent ensuite que Fa♯7, ce qui distingue le progrès de la restauration de Ré♯7.

| Paramètre Fa♯7 | Contrôle local | Régularité | Équilibrée |
|---|---:|---:|---:|
| Facteur de force à 0,2 mm `q` | 0,20 | 0,18 | 0,25 |
| Hystérésis | 0,10 | 0,30 | 0,30 |
| Part de masse dans la tige | 0,30 | 0,45 | 0,45 |
| Fréquence tête–tige | 300 Hz | 150 Hz | 150 Hz |
| Perte de la tige | 0,10 | 2,0 | 2,0 |

Exposant inchangé ; largeur du contact ×1 ; écart de hauteur 0,1 mm ; `strike_ratio=0.105`, soit une position effective de 0,0675 de la longueur à Fa♯7. Le `q` correspond ici à `hardness=0.3*q`. La perte 2,0 concerne l’oscillation relative tête–tige, pas un amortissement général du piano. Ces valeurs restent des hypothèses du modèle, sans mesure mécanique d’un marteau réel à l’appui.

Après sélection, on retire un changement à la fois de la variante équilibrée :

| Ablation de la variante équilibrée | P3 forte (dB/P1) | Creux P2 | Creux P3 |
|---|---:|---:|---:|
| Variante complète | −40,04 | 1,21 | 1,11 |
| Perte de la tige remise à 0,1 | −29,08 | 5,16 | 2,29 |
| Part de masse remise à 0,3 | −35,62 | 6,80 | 5,74 |
| Fréquence remise à 300 Hz | −35,23 | 0,00 | 0,00 |
| Hystérésis remise à 0,1 | −34,94 | 0,20 | 0,10 |

Ces chiffres sont ceux du balayage de diagnostic (27 vélocités) ; les critères finaux ci-dessous portent sur les 100 vélocités entières. L’amortissement, la répartition de masse et le feutre interagissent : un seul bouton ne résout pas à la fois le spectre et la régularité. La tentative de diminuer l’exposant du feutre après amortissement a réintroduit des creux dans plusieurs candidats ; elle n’est pas retenue.

## Validation à couverture constante

1 614 nouveaux WAV : 144 attaques longues de 1,2 s, 1 200 frappes pour les balayages 28–127 sur Do♯7/Ré♯7/Fa♯7/Do8, et 270 notes de 20,5 s (90 par profil). Les mêmes 459 observations d’attaque sont conservées : 349 C7–C8 et 110 de transition ; 71 observations de tenue, dont 12 pour Fa♯7. Huit pianos au total. Les identités des observations correspondent exactement à la validation précédente. Une tenue sous le seuil SNR reste incluse dans chaque profil, aucun candidat ne gagne de score en supprimant une observation.

'''+ '\n'.join(t)+'''

La MAE est calculée par piano puis moyennée entre pianos. La pondération des sources diffère entre le sous-ensemble Fa♯7 et l’ensemble C7–C8 : une petite amélioration du premier n’implique pas mécaniquement une amélioration du second. Les trois scores globaux ont les mêmes 191 clés, mais restent secondaires car la sélection SNR interne du banc historique peut varier.

'''+ '\n'.join(targets)+'''

La variante équilibrée fait passer l’écart P3 pp de −3,38 à −7,08 dB : c’est le coût principal restant. La variante régularité descend à −10,10 dB. À forte, les pianos disponibles ont eux-mêmes un écart-type de 11,86 dB sur P3 (sept sources), et au pp de 9,89 dB (quatre sources) : ces moyennes ne sont pas une consigne exacte de voicing.

Le niveau d’attaque de la variante équilibrée baisse de 0,14/0,41/0,87 dB face au contrôle aux vélocités 45/56/100 ; aucune normalisation ne masque ce changement. Le niveau total ne présente pas de creux sur 28–127. Les creux de partiels sous la vélocité 28 ne sont pas validés ici.

![Vélocités et références](img/fsharp_contact_validation.png)

**Touches protégées :** 45 WAV d’attaque, 300 WAV de balayage et 87 WAV longs sont identiques bit à bit entre contrôle et candidats (tous ceux hors Fa♯7 dans ces lots). Do♯7 et Ré♯7 correspondent également aux rendus de main, et Do8 au profil local précédent. Les creux de Do8 (5,23 dB sur P2) et Do♯7 (7,53 dB sur P2) restent présents : ils ne sont ni aggravés ni corrigés par cette expérience limitée à Fa♯7.

## Ce qui reste de P3 forte : contrôle réel supplémentaire

Les cinq prises de Fa♯7 de Plath permettent de séparer pression binaurale et accélération du chevalet, sans supposer la signification du canal `F` (non utilisé). Ce contrôle n’a pas servi à choisir les candidats. Même analyse de puissance sur 60 ms ; pression et accélération alignées sur leur propre attaque. P3 n’est pas suivi dans le survey tardif de cette note : sa fenêtre utilise le f0/B estimé à partir de P1/P2, avec SNR explicitement vérifié. Les cinq prises passent 12 dB : 17–21 dB dans la pression, 34–40 dB dans l’accélération.

| P3/P1 à forte | Plath, cinq prises | Modèle main, position joueur | Variante équilibrée, position joueur |
|---|---:|---:|---:|
| Pression | −67,4 à −69,3 dB | −31,96 | −47,62 |
| Accélération du chevalet | −41,6 à −45,1 dB | −14,63 | −30,27 |
| Différence pression − accélération | −23,3 à −25,8 dB | −17,33 | −17,36 |

Le modèle est alors entendu aux micros joueur, sans salle ni couvercle et sans limiteur. Avec les micros rapprochés du banc, la dernière ligne vaut environ −9 dB : comparer directement cette écoute à Plath aurait attribué à tort l’effet de position au moteur.

Même après ce rapprochement du dispositif, **l’excès n’est pas seulement un problème de contact** : l’accélération relative de P3 reste plus forte, et sa transmission relative vers les oreilles reste moins atténuée. Le nouveau réglage ne change pratiquement pas l’accélération P3/P1 forte par rapport au précédent contact local (−30,27 contre −30,33 dB), ni la pression à la position joueur (−47,62 contre −47,66). Son petit gain forte aux micros rapprochés n’est donc pas une correction générale de l’excitation P3. Les creux, eux, sont nettement réduits.

Ceci est un diagnostic sur un piano et une fenêtre d’attaque, pas une identification exacte de transfert : positions approximatives, réponse de tête artificielle inconnue, inharmonicités différentes et dynamique réelle non appariée. Plath est aussi beaucoup plus sombre en P3 que la moyenne des sept pianos. Il faut mesurer la transmission sur plusieurs fenêtres et confronter l’écoute avant de transformer ce diagnostic en cible de calibration.

## Fichiers et reproduction

`bench/fsharp-contact/` conserve les plans de recherche, tous les essais, les ablations, les mesures et les observations fixes. `out/fsharp-contact/` conserve les WAV, CSV, presets par frappe et la sonde utilisée. Le manifeste donne les empreintes des binaires et des références.

Ordre : compiler `cargo build --offline --release -p piano-render --example contact_stages`, puis exécuter `diagnose.py`, `search.py`, `refine.py`, `mechanics.py`, `damping.py`, `damped_law.py`, `validate.py`, `ablate_selected.py`, `unison_check.py`, `plath_check.py`, `player_check.py` et `present.py`. Les essais supplémentaires de désaccord et de loi de chevalet sont conservés dans `unison-diagnostics.json` et leur plan ; les fichiers de référence et de contrôle du worktree restent nécessaires.

Les neuf tests Python du banc passent ; la nouvelle sonde compile et son audio de contrôle est identique au moteur normal. Le DSP de production n’a pas changé, donc la suite Rust déjà passée lors de la revalidation n’a pas été relancée intégralement.

Une écoute comparative est dans `out/fsharp-contact/fsharp-comparison.wav` : main, contrôle local, variante équilibrée ; chaque groupe joue 45, 56 puis 100. Un seul gain commun est appliqué à tout le fichier pour l’écoute, sans normalisation par note et sans réutilisation dans les scores. Le défaut et l’UI demeurent ceux de la revalidation précédente.
'''
(R/'docs/fsharp_contact_followup.md').write_text(text)
print(counts,manifest['validation_wavs'],flush=True)
if __name__=='__main__':pass
