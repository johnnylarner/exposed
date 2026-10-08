from pathlib import Path
import unittest

import sys
sys.path.insert(0, str(Path(__file__).parents[1]))
from exposed_resolution import worker


def row(key, name, address=None, needs_resolution=True):
    return {"key": key, "name": name, "address": address, "needs_resolution": needs_resolution}


class RealSplinkIntegration(unittest.TestCase):
    def test_exact_and_fuzzy_scores_are_actual_splink_outputs(self):
        result = worker.score({"version": 1, "candidate_budget": 10, "rows": [
            row("a", "john smith", "1 road"), row("b", "john smith", "1 road"),
            row("c", "jon smith", "1 road"), row("d", "john smith")]})
        pairs = {(p["left"], p["right"]): p for p in result["pairs"]}
        self.assertEqual(result["model"]["splink_version"], "4.0.17")
        self.assertFalse(result["model"]["calibrated"])
        self.assertGreater(pairs["a", "b"]["probability"], 0.999)
        self.assertEqual(pairs["a", "b"]["name_level"], 2)
        self.assertEqual(pairs["a", "b"]["address_level"], 1)
        self.assertLess(pairs["a", "c"]["probability"], pairs["a", "b"]["probability"])
        self.assertLess(pairs["a", "d"]["probability"], 0.999)

    def test_person_candidates_do_not_require_address_or_equal_name_prefix(self):
        result = worker.score({"version": 1, "candidate_budget": 10,
            "rows": [row("a", "john smith"), row("b", "jon smith"), row("c", "james smyth")]})
        self.assertEqual({(p["left"], p["right"]) for p in result["pairs"]}, {("a", "b"), ("a", "c"), ("b", "c")})
        self.assertTrue(all(p["address_level"] == -1 for p in result["pairs"]))

    def test_known_company_anchors_are_candidates_only_for_unanchored_profiles(self):
        result = worker.score({"version": 1, "candidate_budget": 10,
            "rows": [row("a", "acme", "1 road", False), row("b", "acme", "1 road", False), row("c", "acme", "1 road")]})
        self.assertEqual({(p["left"], p["right"]) for p in result["pairs"]}, {("a", "c"), ("b", "c")})

    def test_budget_refuses_instead_of_truncating(self):
        with self.assertRaisesRegex(ValueError, "candidate budget"):
            worker.score({"version": 1, "candidate_budget": 1, "rows": [row(key, "same", "same") for key in ["a", "b", "c"]]})


if __name__ == "__main__":
    unittest.main()
