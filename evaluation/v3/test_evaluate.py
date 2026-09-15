import unittest
from evaluate import score


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
