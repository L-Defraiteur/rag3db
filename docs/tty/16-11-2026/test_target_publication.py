"""Exercise actual verdict-file publication, including delayed open/write."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import launch_full


class PublicationTests(unittest.TestCase):
    def test_open_crossing_deadline_never_publishes_green(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / 'result.json'
            with patch.object(launch_full.time, 'monotonic', return_value=11):
                result = launch_full.publish_result(path, dict(verdict='VERT'), 10)
            self.assertEqual(result['verdict'], 'ROUGE')
            self.assertEqual(json.loads(path.read_text())['verdict'], 'ROUGE')

    def test_write_crossing_deadline_is_replaced_by_red_and_returns_red(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / 'result.json'
            with patch.object(launch_full.time, 'monotonic', side_effect=[9, 11]):
                result = launch_full.publish_result(path, dict(verdict='VERT'), 10)
            self.assertEqual(result['verdict'], 'ROUGE')
            self.assertEqual(json.loads(path.read_text())['verdict'], 'ROUGE')

    def test_real_publication_is_exclusive_and_success_needs_fresh_time(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / 'result.json'
            with patch.object(launch_full.time, 'monotonic', return_value=9):
                result = launch_full.publish_result(path, dict(verdict='VERT'), 10)
                self.assertEqual(result['verdict'], 'VERT')
                with self.assertRaises(FileExistsError):
                    launch_full.publish_result(path, dict(verdict='VERT'), 10)
            self.assertEqual(json.loads(path.read_text())['verdict'], 'VERT')

    def test_main_uses_actual_publication_result_for_exit_status(self):
        # Execute the actual main tail, with only its earlier computation stubbed.
        import ast
        source = Path(launch_full.__file__).read_text()
        tree = ast.parse(source)
        main = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == 'main')
        start = next(i for i, node in enumerate(main.body) if isinstance(node, ast.Assign)
                     and isinstance(node.value, ast.Call) and isinstance(node.value.func, ast.Name)
                     and node.value.func.id == 'publish_result')
        tail = ast.Module(body=main.body[start:], type_ignores=[])
        with tempfile.TemporaryDirectory() as temp:
            context = dict(Q=Path(temp), result=dict(verdict='VERT'), start=0,
                           publish_result=launch_full.publish_result)
            with patch.object(launch_full.time, 'monotonic', return_value=4000):
                with self.assertRaises(SystemExit) as error:
                    exec(compile(ast.fix_missing_locations(tail), '<actual-main-tail>', 'exec'), context)
            self.assertEqual(error.exception.code, 1)
            self.assertEqual(json.loads((Path(temp) / 'composed-verdict.json').read_text())['verdict'], 'ROUGE')


if __name__ == '__main__':
    unittest.main()
