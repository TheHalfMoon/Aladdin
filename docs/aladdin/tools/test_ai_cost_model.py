"""Independent arithmetic, boundary, conservation and sensitivity checks."""
import copy
import json
import unittest
import ai_cost_model as model


class EconomicsTests(unittest.TestCase):
    def setUp(self):
        self.a = json.loads(model.INPUTS.read_text(encoding="utf-8"))

    def test_one_hour_gpu_matches_quote_and_no_token_pricing(self):
        c = copy.deepcopy(self.a["cohorts"]["regular"])
        c.update(tasks_month=1, calls_task=1, retry_fraction=0,
                 small_gpu_seconds_call=3600, escalation_probability=0)
        self.a["hosting"].update(idle_seconds_per_request=0, small_cold_probability=0)
        result = model.cohort_usage(self.a, c)
        self.assertAlmostEqual(result["gpu_usd"], 0.69)
        c["input_tokens_call"] = 100000
        self.assertEqual(model.cohort_usage(self.a, c)["gpu_usd"], result["gpu_usd"])

    def test_cold_idle_retry_work_is_billed_once(self):
        c = copy.deepcopy(self.a["cohorts"]["regular"])
        c.update(tasks_month=10, calls_task=2, retry_fraction=0.5,
                 small_gpu_seconds_call=4, escalation_probability=0)
        self.a["hosting"].update(small_cold_probability=0.2, small_cold_seconds=50,
                                  idle_seconds_per_request=5)
        result = model.cohort_usage(self.a, c)
        self.assertEqual(result["calls_month"], 30)
        self.assertEqual(result["small_billed_seconds"], 570)
        self.assertAlmostEqual(result["gpu_usd"], 570 * 0.69 / 3600)

    def test_escalation_adds_small_and_large_work(self):
        c = copy.deepcopy(self.a["cohorts"]["light"])
        c.update(tasks_month=1, calls_task=1, retry_fraction=0,
                 small_gpu_seconds_call=2, escalation_probability=1,
                 advanced_gpu_seconds_call=10)
        self.a["hosting"].update(small_cold_probability=0, advanced_cold_probability=0,
                                  idle_seconds_per_request=0)
        self.assertAlmostEqual(model.cohort_usage(self.a, c)["gpu_usd"],
                               (2 * 0.69 + 10 * 2.72) / 3600)

    def test_failures_lower_success_denominator_without_refunding_compute(self):
        good = model.fleet(self.a, 100, 10)
        for c in self.a["cohorts"].values():
            c["success_probability"] /= 2
        bad = model.fleet(self.a, 100, 10)
        self.assertEqual(good["gpu_usd"], bad["gpu_usd"])
        self.assertAlmostEqual(bad["operating_usd_per_verified_task"],
                               2 * good["operating_usd_per_verified_task"])

    def test_tax_is_not_revenue_and_fees_are_on_gross(self):
        self.a["payment"].update(percent=0.05, fixed_usd=0.5, refund_rate=0,
                                  chargeback_rate=0, tax_included_rate=0.15)
        self.assertAlmostEqual(model.revenue(self.a, 11.5), 10 - 0.575 - 0.5)

    def test_fixed_cost_is_not_multiplied_by_subscribers(self):
        ten, hundred = model.fleet(self.a, 10, 10), model.fleet(self.a, 100, 10)
        self.assertEqual(ten["fixed_cash_usd"], hundred["fixed_cash_usd"])
        self.assertAlmostEqual(hundred["gpu_usd"], 10 * ten["gpu_usd"])
        self.assertAlmostEqual(hundred["cash_contribution_usd"],
                               10 * ten["cash_contribution_usd"] + 9 * ten["fixed_cash_usd"])

    def test_warm_floor_and_capacity_rounding(self):
        warm = model.fleet(self.a, 10, 10, "warm")
        self.assertEqual(warm["assumed_workers"], 1)
        self.assertAlmostEqual(warm["gpu_usd"], 730 * 0.59)
        self.assertIsInstance(model.fleet(self.a, 1000, 10, "warm")["assumed_workers"], int)

    def test_bad_inputs_fail_before_calculation(self):
        for invalid in (float("nan"), float("inf"), -1, True):
            a = copy.deepcopy(self.a)
            a["hosting"]["small_flex_usd_hour"] = invalid
            with self.assertRaises(ValueError):
                model.fleet(a, 100, 10)
        for invalid in (-0.1, 1.1):
            a = copy.deepcopy(self.a)
            a["cohorts"]["light"]["success_probability"] = invalid
            with self.assertRaises(ValueError):
                model.fleet(a, 100, 10)
        with self.assertRaises(ValueError):
            model.fleet(self.a, 0, 10)
        with self.assertRaises(ValueError):
            model.fleet(self.a, 1.5, 10)
        self.a["cohorts"]["light"]["share"] = 0.9
        with self.assertRaises(ValueError):
            model.fleet(self.a, 100, 10)

    def test_heavy_spend_and_stress_are_visible(self):
        r = model.report(self.a)
        ordinary = next(x for x in r["fleet"] if x["subscribers"] == 100 and x["price_usd"] == 10)
        stress = next(x for x in r["stress_at_10_usd"] if x["subscribers"] == 100)
        capped = next(x for x in r["capped_at_10_usd"] if x["subscribers"] == 100)
        self.assertGreater(stress["gpu_usd"], ordinary["gpu_usd"])
        self.assertLess(capped["gpu_usd"], ordinary["gpu_usd"])
        self.assertLess(capped["verified_tasks_assumed"], ordinary["verified_tasks_assumed"])
        power = model.account(self.a, "power", 100, 20)
        self.assertLess(power["cash_contribution_usd"], 0)
        json.dumps(r, allow_nan=False)
        self.assertIn("ILLUSTRATIVE", model.markdown(r))

    def test_basic_break_even_uses_ceiling(self):
        result = model.basic(self.a, 100)
        n = result["break_even_subscribers_labor"]
        self.assertGreaterEqual(model.basic(self.a, n)["after_maintenance_usd"], 0)
        self.assertLess(model.basic(self.a, n - 1)["after_maintenance_usd"], 0)

    def test_one_failed_cohort_keeps_spend_and_other_successes(self):
        before = model.fleet(self.a, 10, 10)
        self.a["cohorts"]["light"]["success_probability"] = 0
        after = model.fleet(self.a, 10, 10)
        self.assertEqual(after["gpu_usd"], before["gpu_usd"])
        self.assertGreater(after["verified_tasks_assumed"], 0)
        self.assertLess(after["verified_tasks_assumed"], before["verified_tasks_assumed"])
        self.assertEqual(after["verified_cost_status"], "DEFINED")

    def test_all_failed_tasks_have_undefined_success_cost_and_real_spend(self):
        before = model.fleet(self.a, 10, 10)
        for c in self.a["cohorts"].values():
            c["success_probability"] = 0
        after = model.fleet(self.a, 10, 10)
        self.assertEqual(after["gpu_usd"], before["gpu_usd"])
        self.assertEqual(after["verified_tasks_assumed"], 0)
        self.assertIsNone(after["inference_infrastructure_usd_per_verified_task"])
        self.assertIsNone(after["operating_usd_per_verified_task"])
        self.assertEqual(after["verified_cost_status"], "UNDEFINED_NO_VERIFIED_TASKS")
        json.dumps(model.report(self.a), allow_nan=False)

    def test_cap_can_admit_fractional_or_zero_expected_tasks(self):
        self.a["cohorts"]["light"]["small_gpu_seconds_call"] = 10000
        result = model.report(self.a)
        self.assertGreater(result["capped_at_10_usd"][0]["gpu_usd"], 0)
        for c in self.a["cohorts"].values():
            c["tasks_month"] = 0
        result = model.report(self.a)
        self.assertEqual(result["capped_at_10_usd"][0]["gpu_usd"], 0)
        self.assertIsNone(result["capped_at_10_usd"][0]["operating_usd_per_verified_task"])

    def test_discrete_small_fleet_exposes_power_user_concentration(self):
        result = model.report(self.a)
        discrete = next(x for x in result["one_power_among_ten"] if x["price_usd"] == 10)
        mixed = next(x for x in result["fleet"] if x["subscribers"] == 10 and x["price_usd"] == 10)
        self.assertGreater(discrete["gpu_usd"], 190)
        self.assertGreater(discrete["gpu_usd"], mixed["gpu_usd"])


if __name__ == "__main__":
    unittest.main()
