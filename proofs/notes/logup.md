# LogUp Lookup Argument — Implementation Details

## Mathematical Foundation

LogUp (from [eprint 2022/1530](https://eprint.iacr.org/2022/1530)) proves that a set of witness values are all contained in a predefined table, using the **logarithmic derivative** identity:

```
Sigma_j 1/(f_j + beta) = Sigma_i m_i/(t_i + beta)
```

where `f_j` are lookup inputs, `t_i` are table entries, `m_i` is the multiplicity of `t_i` (how many times it was looked up), and `beta` is a random challenge. This holds for all `beta` if and only if every `f_j` appears in the table. The proof follows from **partial fraction decomposition** -- two rational functions with the same poles and residues must be identical.

The original LogUp uses sum-check over multilinear polynomials. This implementation adapts it to **univariate polynomials** by replacing sum-check with a **running sum accumulator**.

---

## Data Structures

### `BatchedArgument<F>` (`logup.rs:129`)

All lookups against the same table, collected together. Has two dimensions:

- **Lookup width** (inner `Vec`): multi-column lookups like `(a, b) in (t_1, t_2)` -- compressed via theta-batching into a single field element
- **Parallel lookups** (outer `Vec`): independent lookups per row, e.g., 8 range checks per row against the same table

```
input_expressions: Vec<Vec<Expression<F>>>
                       ^^^^^^^^^^^^^^^^^^  lookup width (theta-compressed)
                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^  parallel lookups
table_expressions: Vec<Expression<F>>
                       ^^^^^^^^^^^^^^  lookup width (theta-compressed)
```

### `FlattenedArgument<F>` (`logup.rs:150`)

A `BatchedArgument` split into chunks that respect the constraint system's degree bound. The helper constraint has degree `1 + lookup_degree * num_parallel_lookups`, so when too many parallel lookups exist, the argument is partitioned into multiple `FlattenedArgument`s via `BatchedArgument::split()` (`logup.rs:244`).

### `Committed<F>` (`logup/prover.rs:42`)

The three committed polynomials in coefficient form:
- `multiplicities` -- m(X)
- `helper_poly` -- h(X)
- `aggregator_poly` -- Z(X)

### `Evaluated<F>` (`logup.rs:277`)

The four scalar evaluations at challenge point x:
- `multiplicities_eval` -- m(x)
- `helper_eval` -- h(x)
- `accumulator_eval` -- Z(x)
- `accumulator_next_eval` -- Z(omega*x)

---

## Circuit Registration

Users register lookups through `ConstraintSystem` methods (`circuit.rs`):

- **`lookup()`** (`circuit.rs:1863`) -- single input per table column, wraps `batched_lookup`
- **`batched_lookup()`** (`circuit.rs:1878`) -- multiple parallel inputs against `TableColumn`s (fixed columns)
- **`lookup_any()`** (`circuit.rs:1915`) -- single input against arbitrary expressions, wraps `batch_lookup_any`
- **`batch_lookup_any()`** (`circuit.rs:1930`) -- multiple parallel inputs against arbitrary expressions

All four ultimately call `logup::BatchedArgument::new()` and push onto `cs.lookups`.

---

## Prover Protocol (3 phases)

### Phase 1: Compute & Commit Multiplicities

`FlattenedArgument::commit_multiplicities()` (`logup/prover.rs:72`)

1. **Evaluate & compress** input and table expressions using theta-batching (Horner's method):
   ```
   compressed = e_1 + theta*e_2 + theta^2*e_3 + ...
   ```
   Each parallel lookup column gets its own compressed polynomial. The table gets one compressed polynomial.

2. **Count multiplicities** via `compute_multiplicities()` (`logup/prover.rs:327`):
   - Build a hash map of table value -> occurrence count (active rows only)
   - Count how often each value appears across all input columns
   - **Normalize**: if value `v` appears `t` times in the table and is looked up `k` times, each table position gets `m = k/t` (done via batch inversion of table counts)
   - Blinding rows get multiplicity 0

3. **Commit** m(X) in Lagrange basis, write commitment to transcript

Output: `ComputedMultiplicities` holding `multiplicities`, `compressed_input_expression`, `compressed_table_expression`

### Phase 2: Compute & Commit Helper + Accumulator

`ComputedMultiplicities::commit_logderivative()` (`logup/prover.rs:149`)

After beta is sampled from the transcript:

1. **Compute table denominators**: `1/(t(X) + beta)` for each row, via `BatchInvert`

2. **Compute input denominators per column** (parallelized with rayon): for each parallel lookup column, compute `1/(f_j(X) + beta)` via `BatchInvert`

3. **Sum into helper polynomial**: `h(X) = Sigma_j 1/(f_j(X) + beta)` -- sum across all parallel lookup columns at each row

4. **Compute log-derivative difference**: `logderivative_poly[i] = h(i) - m(i)/(t(i) + beta)` -- this is what the running sum accumulates

5. **Build accumulator Z(X) as a running sum** (prefix sum):
   ```
   Z[0] = 0
   Z[i+1] = Z[i] + logderivative_poly[i]
   ```
   Take `n - blinding_factors` values, then chain random blinding values.

6. **Debug assertion** (`logup/prover.rs:226`): verify `Z[0] = 0` and `Z[last_active_row] = 0`. The accumulator returning to zero after all rows is what makes LogUp **sound** -- it means the two sums are equal.

7. **Commit** h(X) and Z(X), write both commitments to transcript. Convert all three polynomials to coefficient form.

Output: `Committed { multiplicities, helper_poly, aggregator_poly }`

### Phase 3: Evaluate at Challenge Point

`Committed::evaluate()` (`logup/prover.rs:257`)

After x is sampled, evaluate and write four values to transcript:
- `m(x)`, `h(x)`, `Z(x)`, `Z(omega*x)`

Then `Evaluated::open()` (`logup/prover.rs:286`) returns 4 `ProverQuery`s for the multi-opening: three at point x and one at omega*x (for the accumulator).

---

## Constraint Identities

Three constraints are enforced, computed in two places -- pointwise at the verifier (`Evaluated::expressions()` in `logup.rs:300`) and over the extended domain for the quotient polynomial (`evaluation.rs:501`):

### 1. Boundary constraint
```
(l_0(X) + l_last(X)) * Z(X) = 0
```
Ensures the accumulator is zero at the first row and the last active row. (`logup.rs:378`)

### 2. Helper constraint
```
h(X) * Prod_j(f_j(X) + beta) = Sigma_j Prod_{k!=j}(f_k(X) + beta)
```
This is the "clearing denominators" form of `h(X) = Sigma_j 1/(f_j(X) + beta)`. It avoids divisions in the constraint system. The degree is `1 + lookup_degree * num_parallel_lookups`, which is why splitting is needed. (`logup.rs:352-365`)

The verifier computes `product = Prod_j(f_j + beta)` and then each partial product as `product * (f_j + beta)^{-1}`, sums them, and checks `h * product - sum = 0`.

In the evaluator (`evaluation.rs:542-543`), this is pre-compiled into a `GraphEvaluator` that computes `sum_partial_products` and `product` via a Horner-like scheme, avoiding explicit inversions by restructuring the computation.

### 3. Accumulator constraint
```
Z(omega*X)*(t(X) + beta) = (Z(X) + h(X))*(t(X) + beta) - m(X)
```
Equivalently: `Z(omega*X) - Z(X) = h(X) - m(X)/(t(X) + beta)`. Multiplied through by `(t(X) + beta)` to avoid division. Only active on non-blinding rows (multiplied by `l_active_row`). (`logup.rs:370-375`, `evaluation.rs:545-551`)

---

## Verifier Protocol

The verifier mirrors the prover in three phases (`verifier.rs`):

1. **Read multiplicities commitments** from transcript (`verifier.rs:109-119`): calls `FlattenedArgument::read_multiplicities()` which reads one commitment per flattened argument
2. **Read helper + accumulator commitments** (`verifier.rs:134-142`): calls `CommittedMultiplicities::read_commitment()` which reads 2 more commitments
3. **Read evaluations** (`verifier.rs:291-299`): calls `Committed::evaluate()` which reads 4 field elements (m(x), h(x), Z(x), Z(omega*x))
4. **Check identities** (`logup.rs:300-383`): computes the three constraint expressions and feeds them into the vanishing argument check
5. **Return queries** (`logup/verifier.rs:112-144`): returns 4 `VerifierQuery`s for the multi-opening verification (pairing check)

---

## Quotient Polynomial Integration

The `Evaluator` (`evaluation.rs:178`) pre-compiles lookup constraints into a `LookupGraphEvaluator` -- an optimized computation graph that avoids re-parsing expressions at each row. During quotient construction (`evaluation.rs:501-553`):

1. Convert m(X), h(X), Z(X) to extended domain (coset evaluation)
2. At each row in the extended domain, evaluate the three constraints
3. Accumulate into the quotient polynomial using Horner's method with random challenge y (each constraint is a separate "layer" combined via `value = value * y + constraint`)

---

## Degree Bound Management

The helper constraint's degree grows linearly with the number of parallel lookups. `BatchedArgument::nb_parallel_lookups()` (`logup.rs:173`) computes the maximum that fits:

```
max_parallel = (next_power_of_two(cs_degree - 1) + 1 - 1) / lookup_degree
```

`split()` then partitions into chunks of that size. Each chunk becomes a separate `FlattenedArgument` with its own m(X), h(X), Z(X) -- meaning more parallel lookups costs more commitments and proof size, but the constraint degree stays bounded.

---

## Summary of Proof Elements per FlattenedArgument

| Element | Commitments | Evaluations | Opening Points |
|---------|-------------|-------------|----------------|
| m(X)    | 1           | m(x)        | x              |
| h(X)    | 1           | h(x)        | x              |
| Z(X)    | 1           | Z(x), Z(omega*x) | x, omega*x |
| **Total** | **3**     | **4**       | **2 distinct** |
