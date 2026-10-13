"""Offline Aladdin sensitivity model: quotes + labeled estimates, never benchmarks.

No network, paid APIs, provisioning, credentials or production imports.
Run: python docs/aladdin/tools/ai_cost_model.py [--inputs PATH] [--json]
"""
import argparse
import copy
import json
import math
from pathlib import Path

INPUTS = Path(__file__).with_name("ai_cost_inputs.json")


def finite(value, label, minimum=0, maximum=None):
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(label + " must be numeric")
    if not math.isfinite(value) or value < minimum:
        raise ValueError(label + " is out of range")
    if maximum is not None and value > maximum:
        raise ValueError(label + " is out of range")


def validate(a):
    finite(a["month_hours"], "month_hours", minimum=1)
    for price in a["prices"]:
        finite(price, "price", minimum=0.01)
    for count in a["subscribers"]:
        finite(count, "subscribers", minimum=1)
        if count != int(count):
            raise ValueError("subscribers must be whole numbers")
    for section in ("payment", "hosting", "labor", "basic"):
        for key, value in a[section].items():
            if key != "note":
                finite(value, section + "." + key)
    for key in ("percent", "refund_rate", "chargeback_rate"):
        finite(a["payment"][key], key, maximum=1)
    for key in ("small_cold_probability", "advanced_cold_probability"):
        finite(a["hosting"][key], key, maximum=1)
    finite(a["hosting"]["utilization_ceiling"], "utilization", 0.001, 1)
    finite(a["hosting"]["peak_to_mean"], "peak_to_mean", 1)
    for name, cohort in a["cohorts"].items():
        for key, value in cohort.items():
            finite(value, name + "." + key)
        for key in ("share", "escalation_probability", "retry_fraction"):
            finite(cohort[key], name + "." + key, maximum=1)
        finite(cohort["success_probability"], name + ".success", 0, 1)
        # Derived admission scenarios are expected counts, possibly below
        # one or zero; failed/denied cohorts can have zero success.
        finite(cohort["tasks_month"], name + ".tasks", minimum=0)
    if not math.isclose(sum(c["share"] for c in a["cohorts"].values()), 1):
        raise ValueError("cohort shares must sum to 1")


def fixed_cost(a):
    h, l = a["hosting"], a["labor"]
    cash = (h["control_plane_usd_month"] + h["disk_backup_usd_month"]
            + h["model_storage_gb"] * h["model_storage_usd_gb_month"]
            + l["signing_compliance_usd_month"])
    if any(c["escalation_probability"] > 0 for c in a["cohorts"].values()):
        cash += h["advanced_model_storage_gb"] * h["model_storage_usd_gb_month"]
    maintenance = l["maintenance_hours_month"] * l["maintenance_usd_hour"]
    return cash, maintenance


def cohort_usage(a, c):
    h = a["hosting"]
    calls = c["tasks_month"] * c["calls_task"] * (1 + c["retry_fraction"])
    advanced_calls = calls * c["escalation_probability"]
    busy = calls * c["small_gpu_seconds_call"]
    cold = calls * h["small_cold_probability"] * h["small_cold_seconds"]
    idle = calls * h["idle_seconds_per_request"]
    advanced_busy = advanced_calls * c["advanced_gpu_seconds_call"]
    advanced_billed = advanced_calls * (c["advanced_gpu_seconds_call"]
        + h["idle_seconds_per_request"]
        + h["advanced_cold_probability"] * h["advanced_cold_seconds"])
    small_gpu = (busy + cold + idle) / 3600 * h["small_flex_usd_hour"]
    advanced_gpu = advanced_billed / 3600 * h["advanced_flex_usd_hour"]
    # Bytes cover both upload and reply as a conservative transfer reserve;
    # provider egress may be smaller. Token counts are not priced as AI tokens.
    bytes_per_call = (c["images_call"] * c["image_kib"] * 1024
                      + 4 * (c["input_tokens_call"] + c["output_tokens_call"]))
    network = (calls + advanced_calls) * bytes_per_call / 1e9 * h["egress_usd_gb"]
    return {
        "calls_month": calls, "small_busy_seconds": busy,
        "small_cold_seconds": cold, "small_idle_seconds": idle,
        "small_billed_seconds": busy + cold + idle,
        "advanced_busy_seconds": advanced_busy,
        "advanced_billed_seconds": advanced_billed,
        "small_gpu_usd": small_gpu, "advanced_gpu_usd": advanced_gpu,
        "gpu_usd": small_gpu + advanced_gpu,
        "network_state_usd": network + h["per_account_state_usd_month"],
        "support_usd": c["support_minutes_month"] / 60 * a["labor"]["support_usd_hour"],
        "verified_tasks": c["tasks_month"] * c["success_probability"],
    }


def revenue(a, price):
    p = a["payment"]
    sales = price / (1 + p["tax_included_rate"])
    # Fees charged on gross collected, with no fee refund assumed.
    deductions = (price * p["percent"] + p["fixed_usd"]
        + sales * p["refund_rate"]
        + p["chargeback_rate"] * (sales + p["chargeback_fee_usd"]))
    return sales - deductions


def fleet(a, subscribers, price, mode="flex"):
    validate(a)
    finite(subscribers, "subscribers", minimum=1)
    finite(price, "price", minimum=0.01)
    if subscribers != int(subscribers):
        raise ValueError("subscribers must be whole numbers")
    if mode not in ("flex", "warm"):
        raise ValueError("unknown mode")
    usage = {k: cohort_usage(a, c) for k, c in a["cohorts"].items()}
    weighted = {key: sum(a["cohorts"][k]["share"] * v[key] for k, v in usage.items())
                for key in next(iter(usage.values()))}
    cash_fixed, maintenance = fixed_cost(a)
    workers = 0
    gpu = subscribers * weighted["gpu_usd"]
    if mode == "warm":
        seconds = a["month_hours"] * 3600
        # Mean-load sizing with a peak multiplier is a capacity HYPOTHESIS,
        # not a queue simulation or proof of p95/p99 service.
        workers = max(1, math.ceil(subscribers * weighted["small_busy_seconds"]
            * a["hosting"]["peak_to_mean"]
            / (seconds * a["hosting"]["utilization_ceiling"])))
        gpu = (workers * a["month_hours"] * a["hosting"]["small_warm_usd_hour"]
               + subscribers * weighted["advanced_gpu_usd"])
    support = subscribers * weighted["support_usd"]
    network_state = subscribers * weighted["network_state_usd"]
    operating = gpu + cash_fixed + network_state + support
    net_receipts = subscribers * revenue(a, price)
    verified = subscribers * weighted["verified_tasks"]
    return {
        "subscribers": subscribers, "price_usd": price, "mode": mode,
        "assumed_workers": workers, "gpu_usd": gpu,
        "fixed_cash_usd": cash_fixed, "support_usd": support,
        "network_state_usd": network_state, "operating_usd": operating,
        "maintenance_usd": maintenance,
        "cash_contribution_usd": net_receipts - operating,
        "after_maintenance_usd": net_receipts - operating - maintenance,
        "verified_tasks_assumed": verified,
        "verified_cost_status": "DEFINED" if verified > 0 else "UNDEFINED_NO_VERIFIED_TASKS",
        "inference_infrastructure_usd_per_verified_task": (gpu + cash_fixed + network_state) / verified if verified > 0 else None,
        "operating_usd_per_verified_task": operating / verified if verified > 0 else None,
    }


def account(a, cohort, subscribers, price):
    validate(a)
    finite(subscribers, "subscribers", minimum=1)
    finite(price, "price", minimum=0.01)
    u = cohort_usage(a, a["cohorts"][cohort])
    cash, maintenance = fixed_cost(a)
    variable = u["gpu_usd"] + u["network_state_usd"] + u["support_usd"]
    return {"cohort": cohort, "price": price, **u,
        "idle_allocation_usd": u["small_idle_seconds"] / 3600 * a["hosting"]["small_flex_usd_hour"],
        "contribution_before_fixed_usd": revenue(a, price) - variable,
        "cash_contribution_usd": revenue(a, price) - variable - cash / subscribers,
        "after_maintenance_usd": revenue(a, price) - variable - (cash + maintenance) / subscribers}


def basic(a, subscribers):
    validate(a)
    finite(subscribers, "subscribers", minimum=1)
    b, l, h = a["basic"], a["labor"], a["hosting"]
    variable = (b["support_minutes_month"] / 60 * l["support_usd_hour"]
                + b["per_account_infrastructure_usd_month"])
    before_fixed = revenue(a, b["price_usd"]) - variable
    fixed = h["control_plane_usd_month"] + b["relay_usd_month"] + l["signing_compliance_usd_month"]
    maintenance = l["maintenance_hours_month"] * l["maintenance_usd_hour"]
    return {"contribution_before_fixed_usd": before_fixed,
        "cash_contribution_usd": subscribers * before_fixed - fixed,
        "after_maintenance_usd": subscribers * before_fixed - fixed - maintenance,
        "break_even_subscribers_cash": math.ceil(fixed / before_fixed) if before_fixed > 0 else None,
        "break_even_subscribers_labor": math.ceil((fixed + maintenance) / before_fixed) if before_fixed > 0 else None}


def report(a):
    validate(a)
    rows = [fleet(a, n, p) for n in a["subscribers"] for p in a["prices"]]
    accounts = [account(a, c, 100, p) for c in a["cohorts"] for p in a["prices"]]
    stress = copy.deepcopy(a)
    stress["hosting"]["small_cold_probability"] = 1
    stress["hosting"]["small_cold_seconds"] = 120
    for c in stress["cohorts"].values():
        c["small_gpu_seconds_call"] *= 3
        c["escalation_probability"] = 0.10
    capped = copy.deepcopy(a)
    # Research scenario: admission refuses further inference before its
    # 7,200 billed-small-GPU-second allowance is consumed. Usage beyond the
    # admitted fraction is excluded from verified tasks as well as spend.
    for c in capped["cohorts"].values():
        seconds = cohort_usage(capped, c)["small_billed_seconds"]
        c["tasks_month"] *= min(1, 7200 / seconds) if seconds else 1
    discrete = copy.deepcopy(a)
    for name, share in {"light": 0.4, "regular": 0.4, "heavy": 0.1, "power": 0.1}.items():
        discrete["cohorts"][name]["share"] = share
    return {"status": a["status"], "assumptions": a, "fleet": rows,
            "accounts_at_100": accounts, "basic_at_100": basic(a, 100),
            "warm_at_10_usd": [fleet(a, n, 10, "warm") for n in a["subscribers"]],
            "stress_at_10_usd": [fleet(stress, n, 10) for n in a["subscribers"]],
            "capped_at_10_usd": [fleet(capped, n, 10) for n in a["subscribers"]],
            "capped_fleet": [fleet(capped, n, p) for n in a["subscribers"] for p in a["prices"]],
            "one_power_among_ten": [fleet(discrete, 10, p) for p in a["prices"]]}


def markdown(r):
    lines = ["# Generated Aladdin cost sensitivity", "", "ILLUSTRATIVE ESTIMATES; no inference or capacity measurements.", "",
             "| Users | Price | GPU/month | Operating/month | Cash contribution | After maintenance |",
             "|---:|---:|---:|---:|---:|---:|"]
    for row in r["fleet"]:
        lines.append("| %d | $%.0f | $%.2f | $%.2f | $%.2f | $%.2f |" % tuple(row[k] for k in
            ("subscribers", "price_usd", "gpu_usd", "operating_usd", "cash_contribution_usd", "after_maintenance_usd")))
    lines += ["", "| Cohort at 100 users | Price | GPU/month | Support/month | Contribution after fixed cash | After maintenance |",
              "|---|---:|---:|---:|---:|---:|"]
    for row in r["accounts_at_100"]:
        lines.append("| %s | $%.0f | $%.2f | $%.2f | $%.2f | $%.2f |" % tuple(row[k] for k in
            ("cohort", "price", "gpu_usd", "support_usd", "cash_contribution_usd", "after_maintenance_usd")))
    lines += ["", "| Users at $10 | Flex uncapped GPU | Warm GPU / assumed workers | Capped GPU | Capped operating | Capped after maintenance | Stress GPU | Stress after maintenance |",
              "|---:|---:|---:|---:|---:|---:|---:|---:|"]
    for n in r["assumptions"]["subscribers"]:
        f = next(x for x in r["fleet"] if x["subscribers"] == n and x["price_usd"] == 10)
        w = next(x for x in r["warm_at_10_usd"] if x["subscribers"] == n)
        c = next(x for x in r["capped_at_10_usd"] if x["subscribers"] == n)
        s = next(x for x in r["stress_at_10_usd"] if x["subscribers"] == n)
        lines.append("| %d | $%.2f | $%.2f / %d | $%.2f | $%.2f | $%.2f | $%.2f | $%.2f |" %
            (n, f["gpu_usd"], w["gpu_usd"], w["assumed_workers"], c["gpu_usd"], c["operating_usd"],
             c["after_maintenance_usd"], s["gpu_usd"], s["after_maintenance_usd"]))
    lines += ["", "| Capped users | Price | Delivered verified tasks (assumed) | Cash contribution | After maintenance |",
              "|---:|---:|---:|---:|---:|"]
    for row in r["capped_fleet"]:
        lines.append("| %d | $%.0f | %.1f | $%.2f | $%.2f |" % tuple(row[k] for k in
            ("subscribers", "price_usd", "verified_tasks_assumed", "cash_contribution_usd", "after_maintenance_usd")))
    lines += ["", "| Ten customers: 4 light, 4 regular, 1 heavy, 1 power | GPU | Operating | Cash contribution | After maintenance |",
              "|---:|---:|---:|---:|---:|"]
    for row in r["one_power_among_ten"]:
        lines.append("| $%.0f | $%.2f | $%.2f | $%.2f | $%.2f |" % tuple(row[k] for k in
            ("price_usd", "gpu_usd", "operating_usd", "cash_contribution_usd", "after_maintenance_usd")))
    lines += ["", "Warm counts are capacity hypotheses, not measured fleet requirements or queue-latency predictions.",
              "Capped scenarios reduce delivered tasks as well as spend. Stress includes 3x busy time, 120s cold loads on every small call, 10% additional large-model requests and 80GB extra model storage.",
              "JSON cost per verified task is null with UNDEFINED_NO_VERIFIED_TASKS when all cohorts fail; spend is retained."]
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inputs", type=Path, default=INPUTS)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    r = report(json.loads(args.inputs.read_text(encoding="utf-8")))
    if args.json:
        print(json.dumps(r, indent=2, allow_nan=False))
    else:
        print(markdown(r), end="")


if __name__ == "__main__":
    main()
