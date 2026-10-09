"""Planning-only cost sensitivity; no prices, network, dependencies or measured throughput.
Historical API comparator is illustrative, not an Aladdin AI hosting proposal.
"""
import math

ASSUMPTIONS = {
    "month_hours": 720,
    "warm_gpu_hour": 2.20,  # illustrative; obtain a dated GPU/region quote
    "min_warm_gpus": 1,
    "gpu_seconds_per_step": 0.2,  # unmeasured amortized batch work, not latency
    "utilization": 0.30,  # unmeasured average, not peak capacity
    "steps_per_task": 10,  # illustrative sensitivity input
    "historical_api_step": 0.000825,
    "historical_steps_per_account": 1500,
}


def estimate(a):
    for key in ("month_hours", "warm_gpu_hour", "min_warm_gpus",
                "gpu_seconds_per_step", "steps_per_task",
                "historical_api_step", "historical_steps_per_account"):
        if a[key] <= 0:
            raise ValueError(key + " must be positive")
    if not 0 < a["utilization"] <= 1:
        raise ValueError("utilization must be in (0, 1]")
    seconds = a["month_hours"] * 3600
    capacity = seconds * a["utilization"] / a["gpu_seconds_per_step"]
    warm = a["month_hours"] * a["warm_gpu_hour"] * a["min_warm_gpus"]
    return {
        "warm_monthly_floor": warm,
        "steps_per_gpu_month_assumed": capacity,
        "amortized_busy_step_cost": a["gpu_seconds_per_step"] / 3600 * a["warm_gpu_hour"],
        "historical_api_crossover_accounts": warm / (a["historical_api_step"] * a["historical_steps_per_account"]),
    }


def cost_at_load(steps, a):
    if steps < 0:
        raise ValueError("steps must be nonnegative")
    e = estimate(a)
    workers = max(a["min_warm_gpus"], math.ceil(steps / e["steps_per_gpu_month_assumed"]))
    return workers, workers * a["month_hours"] * a["warm_gpu_hour"]


if __name__ == "__main__":
    a = ASSUMPTIONS
    e = estimate(a)
    print("ILLUSTRATIVE ESTIMATES; throughput/latency/provider pricing NOT MEASURED")
    print("Warm GPU monthly floor: $%.2f" % e["warm_monthly_floor"])
    print("Assumed steps/GPU-month: %.0f" % e["steps_per_gpu_month_assumed"])
    print("Amortized busy-step cost: $%.8f (excludes idle/load/storage/egress)" % e["amortized_busy_step_cost"])
    print("Historical API-equivalent crossover: %.0f accounts; NOT a hosting/pricing decision" % e["historical_api_crossover_accounts"])
    print("| Attempted steps/month | Assumed warm GPUs | GPU cost/month | Cost/attempted task at %d steps |" % a["steps_per_task"])
    print("|---:|---:|---:|---:|")
    for steps in (150000, 450000, 1500000, 15000000):
        workers, spend = cost_at_load(steps, a)
        print("| %d | %d | $%.2f | $%.4f |" % (steps, workers, spend, spend / (steps / a["steps_per_task"])))
    print("Cost/verified task needs measured success counts and all failed/idle/load/storage/egress spend.")
