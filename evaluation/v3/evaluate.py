"""All held-out questions: quotation integrity is NOT semantic correctness."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
CORPUS = ROOT / 'evaluation/corpus'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def score(q, answer):
    exact, spans = True, []
    for item in answer.get('claims', []):
        for citation in item['citations']:
            name, bounds = citation.rsplit(':', 1)
            nums = bounds.split('-'); lo, hi = int(nums[0]), int(nums[-1])
            path = (CORPUS / name).resolve()
            if not path.is_relative_to(CORPUS.resolve()) or not path.is_file():
                exact = False; continue
            lines = path.read_text().splitlines()
            exact &= 1 <= lo <= hi <= len(lines) and '\n'.join(lines[lo-1:hi]).strip() == item['claim'].strip()
            spans.append((name, lo, hi))
    accepted = answer.get('decision') == 'accepted'
    relevant = bool(spans) and all(any(name == e['path'] and lo <= e['end_line'] and hi >= e['start_line'] for e in q['expected_sources']) for name, lo, hi in spans)
    coverage = bool(q['expected_sources']) and all(any(name == e['path'] and lo <= e['start_line'] and hi >= e['end_line'] for name, lo, hi in spans) for e in q['expected_sources'])
    if answer.get('decision') == 'model_error': verdict = 'provider_error'
    elif not accepted: verdict = 'false_refusal' if q['answerable'] else 'correct_refusal'
    elif not exact or not spans: verdict = 'quotation_failure'
    elif not q['answerable']: verdict = 'false_selection'
    elif not relevant: verdict = 'irrelevant_selection'
    elif not coverage: verdict = 'partial_source_coverage'
    else: verdict = 'expected_source_covered'
    return {'verdict':verdict, 'quotation_exact':exact if accepted else None,
            'expected_span_overlap':relevant,'expected_span_coverage':coverage,'semantic_entailment':'not_asserted'}


def main():
    ap = argparse.ArgumentParser(); ap.add_argument('--output',required=True,type=Path); args=ap.parse_args()
    args.output.mkdir(parents=True,exist_ok=False)
    questions=json.loads((ROOT/'evaluation/v3/questions.json').read_text())
    with urllib.request.urlopen('http://127.0.0.1:11434/api/tags',timeout=5) as r:
        models=[m for m in json.load(r)['models'] if m['name'] in ('qwen2.5-coder:1.5b','nomic-embed-text:latest')]
    subprocess.run(['cargo','build','--locked'],cwd=ROOT,check=True)
    manifest={'commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        'status':subprocess.check_output(['git','status','--porcelain'],cwd=ROOT,text=True),
        'source_sha256':{str(p.relative_to(ROOT)):digest(p) for p in sorted((ROOT/'src').glob('*.rs'))},
        'models':models,'questions_sha256':digest(ROOT/'evaluation/v3/questions.json'),
        'corpus_sha256':{p.name:digest(p) for p in sorted(CORPUS.iterdir()) if p.is_file()},
        'mode':'extractive-selection-v1','latency_scope':'cold CLI including index rebuild and model call',
        'limitation':'Previously exposed synthetic questions; regression evaluation, not fresh unseen generalization evidence.'}
    (args.output/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    records=[]
    env=dict(os.environ,USE_OLLAMA='1',OLLAMA_MODEL='qwen2.5-coder:1.5b',RI_EMBEDDING_PROVIDER='nomic')
    with (args.output/'raw.jsonl').open('w') as out:
        for q in questions:
            if q['split']!='heldout': continue
            started=time.monotonic()
            try:
                proc=subprocess.run([str(ROOT/'target/debug/repository-intelligence'),'--answer-json',str(CORPUS),q['question']],cwd=ROOT,env=env,text=True,capture_output=True,timeout=90)
                answer=json.loads(proc.stdout)
                if proc.returncode: answer['decision']='model_error'
                stderr=proc.stderr
            except (subprocess.TimeoutExpired,json.JSONDecodeError) as exc:
                answer={'decision':'model_error','error':str(exc)}; stderr=str(exc)
            record={'question':q,'answer':answer,'evaluation':score(q,answer),'elapsed_seconds':time.monotonic()-started,'stderr':stderr}
            records.append(record); out.write(json.dumps(record)+'\n'); out.flush()
            print(q['id'],record['evaluation']['verdict'],flush=True)
    counts={}
    for r in records:
        key=r['evaluation']['verdict']; counts[key]=counts.get(key,0)+1
    summary={'total':len(records),'answerable':sum(r['question']['answerable'] for r in records),
        'unanswerable':sum(not r['question']['answerable'] for r in records),
        'model_called':sum(r['answer'].get('model_called',False) for r in records),
        'pre_model_refusals':sum(r['answer'].get('decision')=='pre_model_refusal' for r in records),
        'counts':counts,'semantic_correctness':'not automatically judged; inspect selected excerpts and question rationale'}
    (args.output/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
    (args.output/'report.md').write_text('# Extractive regression evaluation\n\n'+json.dumps(summary,indent=2)+'\n\nSource overlap is not entailment. All 42 previously exposed held-out questions evaluated; no tuning on this run.\n')
    print(json.dumps(summary,indent=2))


if __name__=='__main__': main()
