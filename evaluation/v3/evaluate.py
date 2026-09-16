"""All held-out questions: quotation integrity is NOT semantic correctness."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
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


def summarize(records):
    counts = {}
    for record in records:
        key = record['evaluation']['verdict']
        counts[key] = counts.get(key, 0) + 1
    return {
        'total': len(records),
        'answerable': sum(r['question']['answerable'] for r in records),
        'unanswerable': sum(not r['question']['answerable'] for r in records),
        'model_called': sum(r['answer'].get('model_called', False) for r in records),
        'pre_model_refusals': sum(r['answer'].get('decision') == 'pre_model_refusal' for r in records),
        'provider_errors': sum(r['answer'].get('decision') == 'model_error' for r in records),
        'false_accepts': counts.get('false_selection', 0) + counts.get('irrelevant_selection', 0) + counts.get('partial_source_coverage', 0),
        'false_rejects': counts.get('false_refusal', 0) + counts.get('quotation_failure', 0),
        'counts': counts,
        'semantic_correctness': 'not automatically judged; inspect selected excerpts and question rationale',
        'scoring_rule': 'a question counts as expected_source_covered only when the decision is accepted, the quoted text matches the cited range exactly, all selected spans are relevant to the question, and every expected source span is covered',
    }


def render_report(summary, records):
    lines = [
        '# Extractive regression evaluation',
        '',
        'Mode: `extractive-selection-v1`. The model returns evidence IDs only; the application',
        'renders verbatim source text. This report is rendered from `raw.jsonl`; regenerate with',
        '`python3 evaluation/v3/evaluate.py --render-only <run-dir>` (no model calls).',
        '',
        '## Aggregate (numerator / denominator)',
        '',
        f"- Questions: {summary['total']} held-out ({summary['answerable']} answerable, {summary['unanswerable']} unanswerable)",
        f"- Model calls: {summary['model_called']} / {summary['total']}; pre-model refusals (no lexical anchor, model never called): {summary['pre_model_refusals']}",
        f"- Provider errors: {summary['provider_errors']}",
        f"- False accepts (accepted selection that failed the rule): {summary['false_accepts']}",
        f"- False rejects (answerable question not accepted): {summary['false_rejects']}",
        '',
        '| Verdict | Count |',
        '|---|---:|',
    ]
    for verdict in sorted(summary['counts']):
        lines.append(f"| `{verdict}` | {summary['counts'][verdict]} |")
    lines += [
        '',
        '## Per-question verdicts',
        '',
        '| Question | Split | Answerable | Decision | Model called | Verdict | Quotation exact | Expected span coverage |',
        '|---|---|---|---|---|---|---|---|',
    ]
    for record in records:
        question = record['question']
        answer = record['answer']
        evaluation = record['evaluation']
        lines.append(
            "| {id} | {split} | {answerable} | `{decision}` | {called} | `{verdict}` | {exact} | {coverage} |".format(
                id=question['id'],
                split=question['split'],
                answerable=question['answerable'],
                decision=answer.get('decision', 'unknown'),
                called=answer.get('model_called', False),
                verdict=evaluation['verdict'],
                exact=evaluation.get('quotation_exact'),
                coverage=evaluation.get('expected_span_coverage'),
            )
        )
    lines += [
        '',
        '## Limits',
        '',
        '- Source overlap is **not** entailment. `expected_source_covered` means the model selected',
        '  the frozen expected span with an exact quotation; it does not prove the excerpt answers',
        '  the question in natural language. Semantic correctness is not automatically judged.',
        '- The corpus and questions are authored and were previously exposed; this is a regression',
        '  record, not unseen-generalization evidence. No thresholds were tuned on this run.',
        '- Accepted text is always verbatim source text, so quotation integrity is checked; the',
        '  remaining risk is relevance and interpretation, which stay manual.',
        '',
    ]
    return '\n'.join(lines)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--output', type=Path)
    ap.add_argument('--render-only', type=Path, dest='render_only')
    args = ap.parse_args()

    if args.render_only:
        run_dir = args.render_only
        records = [json.loads(line) for line in (run_dir / 'raw.jsonl').read_text().splitlines() if line.strip()]
        summary = summarize(records)
        (run_dir / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
        (run_dir / 'report.md').write_text(render_report(summary, records))
        print(json.dumps({k: v for k, v in summary.items() if k != 'counts'}, indent=2))
        return

    if not args.output:
        ap.error('--output is required unless --render-only is used')
    args.output.mkdir(parents=True, exist_ok=False)
    questions = json.loads((ROOT / 'evaluation/v3/questions.json').read_text())
    with urllib.request.urlopen('http://127.0.0.1:11434/api/tags', timeout=5) as r:
        models = [m for m in json.load(r)['models'] if m['name'] in ('qwen2.5-coder:1.5b', 'nomic-embed-text:latest')]
    subprocess.run(['cargo', 'build', '--locked'], cwd=ROOT, check=True)
    status_out = subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True)
    if status_out.strip():
        print("ERROR: Working tree is dirty. Please commit before running evaluations to ensure verifiable provenance.")
        sys.exit(1)

    manifest = {
        'commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        'status': subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT, text=True),
        'source_sha256': {str(p.relative_to(ROOT)): digest(p) for p in sorted((ROOT / 'src').glob('*.rs'))},
        'models': models,
        'questions_sha256': digest(ROOT / 'evaluation/v3/questions.json'),
        'corpus_sha256': {p.name: digest(p) for p in sorted(CORPUS.iterdir()) if p.is_file()},
        'mode': 'extractive-selection-v1',
        'latency_scope': 'cold CLI including index rebuild and model call',
        'limitation': 'Previously exposed synthetic questions; regression evaluation, not fresh unseen generalization evidence.',
    }
    (args.output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    records = []
    env = dict(os.environ, USE_OLLAMA='1', OLLAMA_MODEL='qwen2.5-coder:1.5b', RI_EMBEDDING_PROVIDER='nomic')
    with (args.output / 'raw.jsonl').open('w') as out:
        for q in questions:
            if q['split'] != 'heldout':
                continue
            started = time.monotonic()
            try:
                proc = subprocess.run(
                    [str(ROOT / 'target/debug/repository-intelligence'), '--answer-json', str(CORPUS), q['question']],
                    cwd=ROOT, env=env, text=True, capture_output=True, timeout=90,
                )
                answer = json.loads(proc.stdout)
                if proc.returncode:
                    answer['decision'] = 'model_error'
                stderr = proc.stderr
            except (subprocess.TimeoutExpired, json.JSONDecodeError) as exc:
                answer = {'decision': 'model_error', 'error': str(exc)}
                stderr = str(exc)
            record = {'question': q, 'answer': answer, 'evaluation': score(q, answer), 'elapsed_seconds': time.monotonic() - started, 'stderr': stderr}
            records.append(record)
            out.write(json.dumps(record) + '\n')
            out.flush()
            print(q['id'], record['evaluation']['verdict'], flush=True)
    summary = summarize(records)
    (args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    (args.output / 'report.md').write_text(render_report(summary, records))
    print(json.dumps(summary, indent=2))


if __name__ == '__main__':
    main()
