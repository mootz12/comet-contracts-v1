#!/usr/bin/env python3
"""Exact reference values for the Comet math tests.

Computes with Python's `decimal` at high precision so the fixed-point results in the contract can
be checked against a source that has no rounding of its own.

    python3 scripts/math_reference.py c-pow-grid      # table for tests/c_num_test.rs C_POW_GRID
    python3 scripts/math_reference.py single-sided    # exact values in c_math.rs rounding tests

The `c-pow-grid` output is pasted verbatim into `contracts/src/tests/c_num_test.rs`. The
`single-sided` output is what the `// exact ...` comments in `contracts/src/c_math.rs`
`test_single_sided_rounding_*` were taken from.
"""

import sys
from decimal import Decimal as D, getcontext, ROUND_CEILING, ROUND_FLOOR

getcontext().prec = 80

BONE = D(10) ** 18  # 18-decimal fixed point used inside c_num / c_math
STROOP = D(10) ** 7  # 7-decimal token / LP units


def c_pow_grid() -> None:
    """`(base, exp, floor(exact), ceil(exact))` at 18 decimals.

    Bases cover the production-reachable range [0.7, 1.5] (MAX_IN_RATIO, MAX_OUT_RATIO) plus probes
    within 1e9 units of 1.0, where the series converges on its first terms. Exponents cover the
    normalized weights and their reciprocals that appear in the pool math.
    """
    bases = [D(b) for b in ("0.70", "0.75", "0.80", "0.90", "0.99", "1.01", "1.10", "1.30", "1.43", "1.50")]
    bases_i = [int(b * BONE) for b in bases] + [
        10**18 - 10**9, 10**18 - 10**6, 10**18 - 1000, 10**18 - 1,
        10**18 + 1, 10**18 + 1000, 10**18 + 10**6, 10**18 + 10**9,
    ]
    exps = ("0.1", "0.3", "0.5", "0.7", "0.9", "1.0", "1.1111111", "1.4285714", "2.5", "3.3333333", "5", "10")
    exps_i = [int(D(e) * BONE) for e in exps]

    for b in bases_i:
        for e in exps_i:
            exact = (D(b) / BONE) ** (D(e) / BONE) * BONE
            fl = int(exact.to_integral_value(rounding=ROUND_FLOOR))
            ce = int(exact.to_integral_value(rounding=ROUND_CEILING))
            print(f"        ({b}, {e}, {fl}, {ce}),")


def single_sided() -> None:
    """Exact single-sided results for the fixtures in `test_single_sided_rounding_*`.

    Closed-form Balancer formulas with the true 1/w, fee = MIN_FEE (10 stroops = 1e-6), amount = 1
    stroop. Values are in stroops.
    """
    fee = D(10) / STROOP
    amount = D(1)
    fixtures = [
        ("high_token_per_share", D(1_000_000) * STROOP),
        ("low_token_per_share", D(1_000_000_000_000) * STROOP),
    ]
    records = [
        ("record_1", D(5_000_000_000) * STROOP, D(3) / 10),
        ("record_2", D(6_000_000_000) * STROOP, D(7) / 10),
    ]
    for name, supply in fixtures:
        print(f"# {name}: supply = {supply:.0f}")
        for rec, bal, w in records:
            f = 1 - (1 - w) * fee
            lp_given_dep = supply * ((1 + amount * f / bal) ** w - 1)
            dep_given_lp = bal * (((supply + amount) / supply) ** (1 / w) - 1) / f
            lp_given_wdr = supply * (1 - (1 - amount / f / bal) ** w)
            wdr_given_lp = bal * (1 - ((supply - amount) / supply) ** (1 / w)) * f
            print(f"  {rec} (balance {bal:.0f}, w {w}):")
            print(f"    calc_lp_token_amount_given_token_deposits_in      = {lp_given_dep:.6f}  (LP out, round down)")
            print(f"    calc_token_deposits_in_given_lp_token_amount      = {dep_given_lp:.6f}  (token in, round up)")
            print(f"    calc_lp_token_amount_given_token_withdrawal_amount = {lp_given_wdr:.6f}  (LP in, round up)")
            print(f"    calc_token_withdrawal_amount_given_lp_token_amount = {wdr_given_lp:.6f}  (token out, round down)")


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    if cmd == "c-pow-grid":
        c_pow_grid()
    elif cmd == "single-sided":
        single_sided()
    else:
        print(__doc__)
        sys.exit(1)
