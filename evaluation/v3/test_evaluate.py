import unittest
from evaluate import render_report, score, summarize


class EvaluationTests(unittest.TestCase):
    def setUp(self):
        self.q = {'answerable': True, 'expected_sources': [{'path':'api.rs','start_line':1,'end_line':1}]}
        self.answer = {'decision':'accepted','claims':[{'citations':['api.rs:1'],
            'claim': 'pub fn health() -> &\'static str { "status ok" }'}]}

    def test_exact_relevant_source_is_not_called_entailment(self):
        result=score(self.q,self.answer)
        self.assertEqual(result['verdict'],'expected_source_covered')
        self.assertEqual(result['semantic_entailment'],'not_asserted')

    def test_modified_source_is_a_failure(self):
        self.answer['claims'][0]['claim']='status wrong'
        self.assertEqual(score(self.q,self.answer)['verdict'],'quotation_failure')

    def test_trap_selection_is_not_success(self):
        self.q['answerable']=False; self.q['expected_sources']=[]
        self.assertEqual(score(self.q,self.answer)['verdict'],'false_selection')

    def test_provider_error_not_counted_as_refusal(self):
        self.assertEqual(score(self.q,{'decision':'model_error'})['verdict'],'provider_error')

    def test_summary_and_report_are_derived_from_raw_records(self):
        records = [
            {'question': {'id': 'Q1', 'split': 'heldout', 'answerable': True},
             'answer': {'decision': 'accepted', 'model_called': True},
             'evaluation': {'verdict': 'expected_source_covered', 'quotation_exact': True, 'expected_span_coverage': True}},
            {'question': {'id': 'Q2', 'split': 'heldout', 'answerable': True},
             'answer': {'decision': 'refused', 'model_called': True},
             'evaluation': {'verdict': 'false_refusal', 'quotation_exact': None, 'expected_span_coverage': False}},
            {'question': {'id': 'Q3', 'split': 'heldout', 'answerable': False},
             'answer': {'decision': 'accepted', 'model_called': True},
             'evaluation': {'verdict': 'false_selection', 'quotation_exact': True, 'expected_span_coverage': False}},
            {'question': {'id': 'Q4', 'split': 'heldout', 'answerable': False},
             'answer': {'decision': 'pre_model_refusal', 'model_called': False},
             'evaluation': {'verdict': 'correct_refusal', 'quotation_exact': None, 'expected_span_coverage': False}},
        ]
        summary = summarize(records)
        self.assertEqual(summary['total'], 4)
        self.assertEqual(summary['model_called'], 3)
        self.assertEqual(summary['pre_model_refusals'], 1)
        self.assertEqual(summary['provider_errors'], 0)
        self.assertEqual(summary['false_accepts'], 1)
        self.assertEqual(summary['false_rejects'], 1)

        report = render_report(summary, records)
        self.assertIn('False accepts', report)
        self.assertIn('(numerator / denominator)', report)
        self.assertIn('entailment', report)
        for record in records:
            self.assertIn(f"| {record['question']['id']} |", report)
