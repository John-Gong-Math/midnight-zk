# Trash Argument — Implementation Details

## Problem: Multiplicative Selectors Increase Constraint Degree

In PLONK, gates are typically enabled by a **multiplicative selector**: `q(X) * constraint(X) = 0`. This adds 1 to the constraint degree. When the original constraint already sits at a power-of-two boundary (e.g., degree 5), the selector pushes it to degree 6, which can **double** the extended domain size (from 4n to 8n) and correspondingly double the prover's FFT and memory costs.

The **trash argument** (also called "additive selector" or "conditional zeroing") avoids this degree increase by replacing the multiplicative selector with an additive construction that absorbs non-zero constraint values on inactive rows.

---

## The Algebraic Identity

Instead of `q * constraint = 0`, the trash argument enforces:

```
compressed_constraints - (1 - q) * trash = 0
```

where `trash(X)` is a committed polynomial that equals `compressed_constraints(X)` at every row. Expanding:

- **Active row** (`q = 1`): `compressed_constraints - 0 = 0` => constraints must hold
- **Inactive row** (`q = 0`): `compressed_constraints - trash = 0` => always true since `trash = compressed_constraints`

The identity simplifies to `q * compressed_constraints = 0` — the same semantic as a multiplicative selector, but the maximum degree is `max(2, constraint_degree)` rather than `constraint_degree + 1`. The degree-2 floor comes from the `(1 - q) * trash` term.

---

## Data Structures

### `Argument<F>` (`trash.rs:11`)

```rust
pub struct Argument<F: Field> {
    name: String,
    pub(crate) selector: Expression<F>,
    pub(crate) constraint_expressions: Vec<Expression<F>>,
}
```

Stores the selector expression `q` and the list of constraint expressions that should hold when `q = 1`.

### `Evaluated<F>` (`trash.rs:53`)

A single field element `trash_eval` — the evaluation of the trash polynomial at the challenge point `x`.

### `required_degree()` (`trash.rs:31`)

```rust
pub(crate) fn required_degree(&self) -> usize {
    let degrees = self.constraint_expressions.iter().map(|e| e.degree());
    max(2, degrees.max().unwrap_or(0))
}
```

The degree is `max(2, max_constraint_degree)` — degree 2 minimum from the `(1 - q) * trash` term.

---

## What Exactly Is Stored in the Trash Column

### Raw Compressed Constraint Evaluations — Unconditionally

Looking at `trash/prover.rs:41-57`, the `commit()` function evaluates **every** constraint expression over the **entire** domain (all `n` rows), then compresses them via Horner's method with `trash_challenge`:

```rust
let compressed_expression = self
    .constraint_expressions
    .iter()
    .map(|expression| {
        domain.lagrange_from_vec(evaluate(
            expression, domain.n as usize, 1,
            fixed_values, advice_values, instance_values, challenges,
        ))
    })
    .fold(domain.empty_lagrange(), |acc, expression| {
        acc * trash_challenge + &expression
    });
```

At each row `i`:

```
trash[i] = c_1(i) + r * c_2(i) + r^2 * c_3(i) + ...
```

where `r = trash_challenge` and `c_j(i)` is the j-th constraint evaluated at row `i`.

**There is no conditional logic.** No `if active_row { ... }`. No selector masking. The trash column literally stores `compressed_constraints[i]` at every single row, unconditionally.

### Row-by-Row Breakdown

**Active rows** (rows where the selector `q = 1`):
- An honest prover satisfies all constraints, so `c_j(i) = 0` for all `j`
- Therefore `trash[i] = 0 + r*0 + r^2*0 + ... = 0`
- The trash column is **zero** on all active rows

**Inactive rows** (rows where `q = 0`, but not blinding rows):
- The constraints are **not expected to hold** — the prover may have garbage values here
- So `c_j(i)` can be arbitrary nonzero values
- `trash[i]` is some nonzero garbage value
- This is **exactly** what the trash column is designed to absorb

**Blinding rows** (the last `blinding_factors` rows):
- Advice columns contain **random** values (filled by the prover for zero-knowledge)
- The constraints evaluated at these rows produce random-looking outputs
- `trash[i]` is random-looking (a deterministic function of the random advice blinding values)

### Why No Conditional Logic Is Needed

By setting `trash = compressed_constraints` everywhere, the verifier identity:

```
compressed_constraints - (1 - q) * trash = 0
```

automatically simplifies to `q * compressed_constraints = 0`. On active rows (`q = 1`), the constraints must be zero. On inactive rows (`q = 0`), anything goes. The algebra is self-cancelling — no row-by-row branching required.

---

## Prover Protocol

### Phase 1: Commit Trash Polynomial

`Argument::commit()` (`trash/prover.rs:24`)

1. **Evaluate** all constraint expressions over the full domain (all `n` rows), unconditionally
2. **Compress** via Horner's method with `trash_challenge`: `compressed = c_1 + r*c_2 + r^2*c_3 + ...`
3. **Commit** the compressed polynomial in Lagrange basis
4. **Write** commitment to transcript
5. **Convert** to coefficient form for later evaluation

Output: `Committed { trash_poly }` — the compressed polynomial in coefficient form.

### Phase 2: Evaluate at Challenge Point

`Committed::evaluate()` (`trash/prover.rs:70`)

After `x` is sampled from the transcript:
1. Evaluate `trash(x)` from coefficient form
2. Write `trash(x)` to transcript

Output: `Evaluated(Committed)` wrapping the original committed data.

### Phase 3: Opening Proof

`Evaluated::open()` (`trash/prover.rs:83`)

Returns 1 `ProverQuery` at point `x` for the multi-opening proof.

---

## Constraint Identity

Checked in `Evaluated::expressions()` (`trash.rs:58`):

```
compressed_constraints(x) - (1 - q(x)) * trash(x) = 0
```

where `compressed_constraints(x)` is recomputed by the verifier from the constraint expressions evaluated at `x`, compressed with `trash_challenge` via Horner.

This produces **one** constraint expression per trash argument, fed into the vanishing argument.

---

## Verifier Protocol

The verifier mirrors the prover (`trash/verifier.rs`):

1. **Read commitment**: `read_committed()` reads the trash commitment from transcript
2. **Read evaluation**: `evaluate()` reads `trash(x)` from transcript
3. **Check identity**: `expressions()` recomputes `compressed_constraints(x)` from the constraint expressions (using advice/fixed/instance evaluations at `x`), then checks `compressed - (1 - q(x)) * trash(x) = 0`
4. **Return queries**: `queries()` returns 1 `VerifierQuery` at point `x` for the multi-opening verification

---

## Quotient Polynomial Integration

In the `Evaluator` (`evaluation.rs`), trash constraints are pre-compiled into a `GraphEvaluator`. During quotient polynomial construction over the extended domain, at each row:

```
value = compressed_expression[idx] - (1 - q[idx]) * trash_poly[idx]
```

This is accumulated into the quotient polynomial via Horner's method with challenge `y`.

---

## Security Analysis

### Cheating Prover Cannot Exploit the Trash Column

A cheating prover who violates a constraint on an active row (`q = 1`) would have `compressed_constraints != 0` at that row. The identity becomes:

```
compressed_constraints - (1 - 1) * trash = compressed_constraints != 0
```

The trash term vanishes (multiplied by zero) on active rows, so the trash column cannot "cover up" a constraint violation where the selector is active. The prover is trapped: the trash polynomial is committed before `x` is known, so they cannot retroactively adjust it.

### Zero-Knowledge: No Explicit Blinding, But Safe

The trash polynomial receives **no explicit random blinding**. There is no `rng` parameter in `commit()`, no random values appended — the commitment goes directly on the raw `compressed_expression`. This is safe because:

1. **The trash polynomial is a deterministic function of already-blinded advice columns.** The advice columns have random blinding values in their last rows (filled during `synthesize`). When the trash column evaluates constraints at blinding rows, the outputs are random-looking.

2. **Each trash argument adds 1 to the blinding factor count** (`circuit.rs:2415`):
   ```rust
   let factors = factors + self.trashcans.len();
   ```
   This ensures advice columns have enough random blinding rows to compensate for the extra evaluation that the trash polynomial reveals (one linear equation in the advice column values).

3. The trash polynomial reveals one evaluation at point `x`. The additional blinding factor in the advice columns ensures this does not leak information — the advice polynomial has one more random degree of freedom than what the evaluations reveal.

### Compression Challenge Ordering

The `trash_challenge` used for Horner compression is squeezed from the transcript **after** all permutation and LogUp commitments are already fixed. This means the prover cannot choose constraint expressions that cancel each other out during compression — the challenge is unknown at the time the advice is committed.

---

## Where the Trash Argument Is Used

Searching for `with_additive_selector` across the codebase reveals two call sites:

1. **`proofs/examples/two-chip.rs:286`** — a toy example
2. **`circuits/src/hash/poseidon/poseidon_chip.rs:257`** — the only production use

### The Production Use: Poseidon Partial Round Gate

The Poseidon hash chip defines two gates:

**Full round gate** (line 197-219) — uses **multiplicative selector** (`with_selector`):
- Uses a hint trick: the prover provides `x^3` as a separate advice value, then the circuit verifies `x * x^2 - x^3 = 0` (degree 3) and uses `x^3 * x^2` (degree 3) instead of computing `x^5` directly
- Constraint degree = 3, with multiplicative selector: total degree = **4**

**Partial round gate** (line 229-258) — uses **additive selector** (`with_additive_selector`):
- Batches **6 consecutive partial rounds** into a single gate row (`1 + NB_SKIPS_CIRCUIT` where `NB_SKIPS_CIRCUIT = 5`)
- The 6-round composition is pre-computed algebraically in `round_skips.rs`, producing expressions with `sbox(x) = x^5` terms (degree 5) baked in
- The hint trick cannot be applied here because the sbox appears inside composed multi-round expressions, not as an isolated operation
- Constraint degree = **5**

### Why Multiplicative Selector Cannot Be Used

The critical issue is **extended domain sizing** in `EvaluationDomain::new()` (`poly/domain.rs`):

```
extended_k = smallest k such that 2^k >= n * (cs_degree - 1)
```

The extended domain must be a power of two:

| Constraint degree | `cs_degree - 1` | Extended domain size |
|---|---|---|
| <= 5 | 4 | **4n** (4 is a power of 2) |
| 6 | 5 | **8n** (5 rounds up to 8) |

- **With multiplicative selector**: partial round degree = 5 + 1 = **6**, extended domain = **8n**
- **With trash argument**: degree stays at `max(2, 5)` = **5**, extended domain = **4n**

Going from 4n to 8n means **2x more FFT work** (superlinear `O(n log n)`), **2x more memory** for extended-domain polynomials, and **2x more pointwise evaluations** during quotient construction. This affects **every** polynomial in the system, since the extended domain is global.

The trash argument avoids this at a much smaller cost: 1 extra commitment, 1 extra evaluation, 1 extra blinding factor, and 2 extra constraints in the quotient.

---

## Summary of Proof Elements per Trash Argument

| Element | Commitments | Evaluations | Opening Points |
|---------|-------------|-------------|----------------|
| trash(X) | 1 | trash(x) | x |
| **Total** | **1** | **1** | **1** |

Additionally: 1 extra blinding factor consumed from the advice columns.
