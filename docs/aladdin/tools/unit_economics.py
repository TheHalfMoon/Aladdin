"""Aladdin unit-economics model (planning estimate, not a measurement).

Every input is an explicit assumption listed in ASSUMPTIONS; change them and
re-run with `python -I unit_economics.py`. No network access, no dependencies.
"""

ASSUMPTIONS = {
    # Pricing hypotheses (founder-proposed, not validated).
    "core_price": 5.00,
    "ai_price": 12.00,
    # Payment processing: card processor 2.9% + $0.30 per charge (merchant of
    # record would be higher, about 5% + $0.50). Taxes are passed through.
    "card_pct": 0.029,
    "card_fixed": 0.30,
    # Hala One per visual step through a per-token API (no idle cost):
    # ~1,500 uncached input tokens (screenshot ~1,300 + delta text),
    # ~2,500 cached prefix tokens, ~150 output tokens.
    # Prices: $0.30/M input, $0.03/M cached input, $2.00/M output.
    "hala_step_low": 0.30e-6 * 1500 + 0.03e-6 * 2500 + 2.00e-6 * 100,
    "hala_step_expected": 0.30e-6 * 1500 + 0.03e-6 * 2500 + 2.00e-6 * 150,
    "hala_step_high": 0.30e-6 * 4000 + 2.00e-6 * 400,  # no cache, verbose
    # Reliance decision (d1-3B class, zero output tokens, ~8-30 ms GPU).
    "reliance_low": 0.000005,
    "reliance_expected": 0.00002,
    "reliance_high": 0.0001,
    # Voice per minute (streaming STT + TTS, self-hosted GPU at moderate load).
    "voice_min_low": 0.001,
    "voice_min_expected": 0.003,
    "voice_min_high": 0.01,
    # Control plane (gateway, auth, relay rendezvous, DB, monitoring) fixed
    # monthly cost by scale, plus per-subscriber variable cost.
    "fixed_by_subs": {100: 60.0, 1000: 250.0, 10000: 1500.0, 100000: 9000.0},
    "per_sub_ops": 0.15,  # logs, storage, egress, email, support tooling
}

# Monthly usage profiles for one Aladdin AI subscriber.
USAGE = {
    "low": {"hala": 300, "reliance": 1500, "voice_min": 10},
    "expected": {"hala": 1500, "reliance": 6000, "voice_min": 45},
    "high": {"hala": 6000, "reliance": 20000, "voice_min": 240},
}


def net_revenue(price):
    return price - (price * ASSUMPTIONS["card_pct"] + ASSUMPTIONS["card_fixed"])


def ai_variable_cost(profile, price_level):
    a = ASSUMPTIONS
    u = USAGE[profile]
    return (
        u["hala"] * a[f"hala_step_{price_level}"]
        + u["reliance"] * a[f"reliance_{price_level}"]
        + u["voice_min"] * a[f"voice_min_{price_level}"]
        + a["per_sub_ops"]
    )


def table_subscribers():
    rows = []
    for subs, fixed in ASSUMPTIONS["fixed_by_subs"].items():
        for profile in ("low", "expected", "high"):
            level = "expected" if profile != "high" else "high"
            var = ai_variable_cost(profile, level)
            revenue = subs * net_revenue(ASSUMPTIONS["ai_price"])
            cost = subs * var + fixed
            rows.append((subs, profile, revenue, cost, revenue - cost))
    return rows


def ten_thousand_calls():
    a = ASSUMPTIONS
    per_call = {
        "1 local tool invocation (runs on user device)": 0.0,
        "2 remote device action via relay (~20 KB, $0.02/GB egress)": 20e3 / 1e9 * 0.02,
        "3 Reliance decision": a["reliance_expected"],
        "4 Hala One visual inference": a["hala_step_expected"],
        "5 full AI workflow step (1 Hala + 3 Reliance + relay + log)": a["hala_step_expected"]
        + 3 * a["reliance_expected"]
        + 150e3 / 1e9 * 0.02
        + 0.00005,
    }
    budget = net_revenue(a["ai_price"]) - a["per_sub_ops"]
    out = []
    for name, cost in per_call.items():
        total = cost * 10_000
        max_calls = budget / cost if cost else float("inf")
        out.append((name, cost, total, max_calls))
    return budget, out


# Self-hosted fine-tuned Hala One (required if the fine-tuned weights cannot be
# served by a per-token provider). Assumptions: one H100-class GPU at
# $3.95/hour serverless or $2.20/hour reserved; ~0.2 GPU-seconds per step at
# good batching; 30% achievable average utilization; at least one warm GPU to
# avoid 30-90 s cold starts on a ~35B-parameter checkpoint.
SELF_HOST = {"gpu_hour_serverless": 3.95, "gpu_hour_reserved": 2.20,
             "gpu_s_per_step": 0.2, "utilization": 0.30, "min_warm_gpus": 1}


def self_host_hala_monthly(subs, steps_per_sub, hourly):
    import math
    sh = SELF_HOST
    capacity = 30 * 24 * 3600 * sh["utilization"] / sh["gpu_s_per_step"]
    gpus = max(sh["min_warm_gpus"], math.ceil(subs * steps_per_sub / capacity))
    return gpus, gpus * hourly * 24 * 30


def core_check():
    return net_revenue(ASSUMPTIONS["core_price"])


if __name__ == "__main__":
    print("Net revenue per charge: Core $%.2f, AI $%.2f" % (core_check(), net_revenue(ASSUMPTIONS["ai_price"])))
    print("\nHala step cost low/expected/high: $%.5f / $%.5f / $%.5f" % (
        ASSUMPTIONS["hala_step_low"], ASSUMPTIONS["hala_step_expected"], ASSUMPTIONS["hala_step_high"]))
    print("\nAI variable cost per subscriber-month:")
    for p in USAGE:
        lvl = "expected" if p != "high" else "high"
        print("  %-9s $%.2f" % (p, ai_variable_cost(p, lvl)))
    print("\n| Subscribers | Usage | Net revenue | Total cost | Monthly margin |")
    print("|---:|---|---:|---:|---:|")
    for subs, prof, rev, cost, margin in table_subscribers():
        print("| %d | %s | $%s | $%s | $%s |" % (subs, prof, f"{rev:,.0f}", f"{cost:,.0f}", f"{margin:,.0f}"))
    print("\nSelf-hosted Hala One (reserved $%.2f/h, 30%% utilization, >=1 warm GPU), expected usage:" % SELF_HOST["gpu_hour_reserved"])
    print("| Subscribers | GPUs | Hala GPU cost/month | Per subscriber | Margin after all costs |")
    print("|---:|---:|---:|---:|---:|")
    for subs, fixed in ASSUMPTIONS["fixed_by_subs"].items():
        g, c = self_host_hala_monthly(subs, USAGE["expected"]["hala"], SELF_HOST["gpu_hour_reserved"])
        other = subs * (ai_variable_cost("expected", "expected") - USAGE["expected"]["hala"] * ASSUMPTIONS["hala_step_expected"])
        margin = subs * net_revenue(ASSUMPTIONS["ai_price"]) - c - other - fixed
        print("| %d | %d | $%s | $%.2f | $%s |" % (subs, g, f"{c:,.0f}", c / subs, f"{margin:,.0f}"))
    budget, rows = ten_thousand_calls()
    print("\nCompute budget per $12 subscriber after fees and ops: $%.2f" % budget)
    print("| Interpretation of one 'call' | Cost per call | Cost of 10,000 | Break-even calls per month |")
    print("|---|---:|---:|---:|")
    for name, c, total, maxc in rows:
        mc = "unbounded" if maxc == float("inf") else f"{maxc:,.0f}"
        print("| %s | $%.6f | $%.2f | %s |" % (name, c, total, mc))
