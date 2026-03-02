# Proofs Crate Architecture

## Module Tree

```
proofs/src/
|
|-- lib.rs                       crate root, re-exports all public modules
|
|-- circuit/                     circuit model & synthesis
|   |-- mod.rs                   Chip, Layouter, Region, Cell, AssignedCell, Table traits
|   |-- value.rs                 Value<V> (known/unknown wrapper for witness data)
|   |-- layouter.rs              RegionLayouter trait (dyn dispatch for region assignment)
|   |-- table_layouter.rs        SimpleTableLayouter, TableLayouter trait
|   +-- floor_planner/           layout strategies
|       |-- single_pass.rs       SimpleFloorPlanner (single-pass, one region at a time)
|       +-- v1/                  V1FloorPlanner (optimized column packing)
|           +-- strategy.rs      slot allocation strategy
|
|-- plonk/                       PLONK proving system
|   |-- mod.rs                   VerifyingKey, ProvingKey, evaluate_identities()
|   |-- circuit.rs               ConstraintSystem, Expression, Gate, Column types,
|   |                            Selector, Challenge, VirtualCells
|   |-- keygen.rs                keygen_vk(), keygen_pk()
|   |-- prover.rs                create_proof(), compute_nu_poly(), create_opening_proof()
|   |-- verifier.rs              parse_trace(), verify_proof()
|   |-- evaluation.rs            Evaluator, GraphEvaluator, LookupGraphEvaluator
|   |-- error.rs                 Error enum
|   |-- traces.rs                ProverTrace, VerifierTrace
|   |
|   |-- permutation/             copy-constraint argument
|   |   |-- permutation.rs       Argument, VerifyingKey, ProvingKey, Evaluated
|   |   |-- keygen.rs            Assembly, keygen()
|   |   |-- prover.rs            commit(), evaluate(), open()
|   |   +-- verifier.rs          read_commitments(), evaluate(), queries()
|   |
|   |-- logup/                   LogUp lookup argument
|   |   |-- logup.rs             BatchedArgument, FlattenedArgument, Evaluated, split()
|   |   |-- prover.rs            commit_multiplicities(), commit_logderivative()
|   |   +-- verifier.rs          read_multiplicities(), read_commitment(), queries()
|   |
|   |-- vanishing/               quotient polynomial argument
|   |   |-- vanishing.rs         data types
|   |   |-- prover.rs            commit_random(), construct(), evaluate()
|   |   +-- verifier.rs          verify(), queries()
|   |
|   |-- trash/                   conditional constraint zeroing
|   |   |-- trash.rs             Argument
|   |   |-- prover.rs            commit(), evaluate(), open()
|   |   +-- verifier.rs          read_commitments(), evaluate(), queries()
|   |
|   +-- bench/                   internal benchmarking support (feature-gated)
|
|-- poly/                        polynomial arithmetic & commitment
|   |-- mod.rs                   Polynomial<F, B>, Coeff, LagrangeCoeff,
|   |                            ExtendedLagrangeCoeff, Rotation
|   |-- domain.rs                EvaluationDomain (FFT, iFFT, coset ops, vanishing poly)
|   |-- commitment.rs            PolynomialCommitmentScheme trait, Guard trait, Params trait
|   |-- query.rs                 ProverQuery, VerifierQuery, CommitmentLabel
|   +-- kzg/                     KZG commitment scheme
|       |-- mod.rs               KZGCommitmentScheme<E>, multi_open(), multi_prepare()
|       |-- params.rs            ParamsKZG (SRS), ParamsVerifierKZG
|       |-- msm.rs               MSMKZG, DualMSM (multi-scalar multiplication)
|       +-- utils.rs             construct_intermediate_sets()
|
|-- transcript/                  Fiat-Shamir transcript
|   |-- mod.rs                   Transcript, TranscriptHash, Hashable, CircuitTranscript<H>
|   +-- implementors.rs          Blake2b hash impl, field/curve Hashable impls
|
|-- dev/                         development & testing tools
|   |-- mod.rs                   MockProver
|   |-- failure.rs               VerifyFailure, FailureLocation
|   |   +-- emitter.rs           error message formatting
|   |-- gates.rs                 CircuitGates (introspection)
|   |-- tfp.rs                   TracingFloorPlanner
|   |-- cost_model.rs            circuit cost estimation
|   |-- metadata.rs              metadata types for error reporting
|   +-- util.rs                  test helpers
|
+-- utils/                       shared utilities
    |-- mod.rs                   re-exports
    |-- arithmetic.rs            parallelize(), eval_polynomial(), kate_division(),
    |                            lagrange_interpolate(), MSM, CurveAffine, CurveExt
    |-- helpers.rs               SerdeFormat, serialization helpers
    |-- rational.rs              Rational<F> (numerator/denominator pair)
    +-- multicore.rs             rayon thread-pool helpers
```


## Inter-Module Dependencies

```
                            +-------------+
                            |   circuit   |
                            | Chip,       |
                            | Layouter,   |
                            | Region,     |
                            | FloorPlanner|
                            +------+------+
                                   |
                                   | defines how users
                                   | describe circuits
                                   v
+-------------+            +-------+-------+           +--------------+
|  transcript |<---------->|     plonk     |<--------->|     poly     |
| Fiat-Shamir |  challenges|               | polynomial|              |
| Transcript, |  & hashing | keygen        | commit,   | Polynomial,  |
| Hashable,   |            | prover        | open,     | Domain,      |
| Blake2b     |            | verifier      | verify    | KZG,         |
+-------------+            | ConstraintSys |           | MSM          |
                           +---+---+---+---+           +--------------+
                               |   |   |
              +----------------+   |   +----------------+
              |                    |                     |
     +--------v---+       +-------v-------+     +-------v------+
     | permutation|       |     logup     |     |   vanishing  |
     | copy       |       | lookup arg    |     | quotient     |
     | constraints|       | (log-deriv)   |     | polynomial   |
     +------------+       +---------------+     +--------------+
                                                        |
                                                        |  also uses
                                                +-------v------+
                                                |    trash     |
                                                | conditional  |
                                                | zeroing      |
                                                +--------------+

     +------------+            +------------+
     |    dev     |            |   utils    |
     | MockProver,|            | arithmetic,|
     | Tracing    |----------->| helpers,   |
     | FloorPlanner,           | rational,  |
     | CostModel  |            | multicore  |
     +------------+            +------------+
```


## Key Types & Traits

```
+-----------------------------------------------------------------------+
|                           PUBLIC TRAITS                                |
+-----------------------------------------------------------------------+
|                                                                       |
|  Circuit<F>                     PolynomialCommitmentScheme<F>         |
|  +-- type Config                +-- type Parameters                   |
|  +-- type FloorPlanner          +-- type Commitment                   |
|  +-- configure(meta) -> Config  +-- type VerificationGuard            |
|  +-- synthesize(config, layouter) +-- commit(params, poly)            |
|                                 +-- commit_lagrange(params, poly)     |
|  Chip<F>                        +-- multi_open(params, queries, tx)   |
|  +-- type Config                +-- multi_prepare(queries, tx)        |
|  +-- type Loaded                                                      |
|                                 Transcript                            |
|  Layouter<F>                    +-- squeeze_challenge()               |
|  +-- assign_region()            +-- common(input)                     |
|  +-- assign_table()             +-- read() / write()                  |
|  +-- constrain_instance()       +-- finalize() -> proof bytes         |
|  +-- get_challenge()                                                  |
+-----------------------------------------------------------------------+

+-----------------------------------------------------------------------+
|                           KEY STRUCTS                                  |
+-----------------------------------------------------------------------+
|                                                                       |
|  ConstraintSystem<F>             VerifyingKey<F, CS>                  |
|  +-- gates: Vec<Gate>            +-- domain: EvaluationDomain         |
|  +-- advice/fixed/instance cols  +-- fixed_commitments                |
|  +-- lookups: Vec<BatchedArg>    +-- permutation VK                   |
|  +-- permutation: Argument       +-- cs: ConstraintSystem             |
|  +-- trashcans: Vec<Argument>    +-- transcript_repr                  |
|  +-- challenges: Vec<Challenge>                                       |
|                                  ProvingKey<F, CS>                    |
|  Expression<F>                   +-- vk: VerifyingKey                 |
|  +-- Constant(F)                 +-- l0, l_last, l_active_row         |
|  +-- Selector(Selector)         +-- fixed_values/polys/cosets        |
|  +-- Fixed/Advice/Instance(Q)   +-- permutation PK                   |
|  +-- Challenge(Challenge)        +-- ev: Evaluator                    |
|  +-- Negated/Sum/Product/Scaled                                       |
|                                  EvaluationDomain<F>                  |
|  Polynomial<F, B>                +-- n, k, extended_k                 |
|    B = Coeff                     +-- omega, omega_inv                 |
|      | LagrangeCoeff             +-- coeff_to_lagrange()              |
|      | ExtendedLagrangeCoeff     +-- lagrange_to_coeff()              |
|                                  +-- coeff_to_extended()              |
+-----------------------------------------------------------------------+
```


## End-to-End Proof Flow

```
                         USER CODE
                            |
                            v
              +---------------------------+
              |  impl Circuit<F> for C    |
              |    configure(meta)        |----> define gates, columns, lookups
              |    synthesize(cfg, lay)   |----> assign witness & fixed values
              +---------------------------+
                            |
          +-----------------+-------------------+
          v                                     v
   +-------------+                      +---------------+
   |  KEYGEN     |                      |    PROVING    |
   +-------------+                      +---------------+
   |                                    |
   | 1. configure() -> CS              | 1. synthesize() -> advice values
   | 2. synthesize() -> fixed values   | 2. Commit advice (per phase)
   | 3. Selector optimization          | 3. theta <- transcript
   | 4. EvaluationDomain::new(2^k)    | 4. LogUp: commit multiplicities
   | 5. Commit fixed polys (KZG)      | 5. beta, gamma <- transcript
   | 6. Build permutation sigma polys  | 6. Permutation: commit Z(X)
   | 7. Build Evaluator (graph)        | 7. LogUp: commit h(X), Z(X)
   |                                    | 8. trash_challenge <- transcript
   | Output:                            | 9. Trash: commit trash poly
   |   ProvingKey                       | 10. y <- transcript
   |   VerifyingKey                     | 11. Evaluate all constraints
   |                                    |     on extended domain (coset FFT)
   +-------------+                      | 12. Quotient poly h(X) / t(X)
                 |                      | 13. Split & commit quotient pieces
                 |                      | 14. x <- transcript
                 v                      | 15. Evaluate all polys at x
          +-------------+              | 16. Multi-opening proof (KZG)
          |  VERIFYING   |              |
          +-------------+              | Output: proof bytes
          |                             +---------------+
          | 1. Read advice commitments
          | 2. Reconstruct challenges
          |    (theta, beta, gamma,
          |     trash, y, x)
          | 3. Read permutation,
          |    logup, trash commitments
          | 4. Read vanishing commitments
          | 5. Read evaluations at x
          | 6. evaluate_identities():
          |    - custom gates
          |    - permutation constraints
          |    - logup constraints
          |    - trash constraints
          | 7. Verify quotient identity
          | 8. Multi-open verify
          |    (KZG pairing check)
          |
          | Output: accept / reject
          +-------------+
```


## PLONK Sub-Arguments

Each sub-argument follows the same prover/verifier lifecycle:

```
                    PROVER SIDE                         VERIFIER SIDE
              +-------------------+               +-------------------+
              |  commit()         |    proof      |  read()           |
              |  Lagrange -> KZG  |   ------->    |  from transcript  |
              +-------------------+               +-------------------+
                      |                                    |
                      v                                    v
              +-------------------+               +-------------------+
              |  evaluate(x)     |   evals at x  |  evaluate()       |
              |  write to tx     |   ------->    |  read from tx     |
              +-------------------+               +-------------------+
                      |                                    |
                      v                                    v
              +-------------------+               +-------------------+
              |  open()           |   queries     |  queries()        |
              |  -> ProverQuery[] |               |  -> VerifierQuery[]
              +-------------------+               +-------------------+
                      |                                    |
                      +----------> multi-open <------------+
                                   (KZG batched)
```

### Permutation (copy constraints)

```
  Advice columns wired via permutation polynomials sigma(X).
  Grand product Z(X) proves the permutation relation:

    Z(wX) * Prod(p_i(X) + beta*sigma_i(X) + gamma)
      = Z(X) * Prod(p_i(X) + beta*delta^i*X + gamma)

  Commits: Z(X) per chunk   |  Evals: Z(x), Z(wx), sigma_i(x)
```

### LogUp (lookup argument)

```
  Proves f_j in Table via logarithmic derivative identity.
  Three polynomials per FlattenedArgument:

    m(X) = multiplicities          (how many times each table row is used)
    h(X) = Sigma_j 1/(f_j(X)+beta) (helper, aggregates all parallel lookups)
    Z(X) = running sum of h(X) - m(X)/(t(X)+beta)

  Constraints:
    1. (l_0 + l_last) * Z(X) = 0                      boundary
    2. h(X) * Prod_j(f_j+beta) = Sigma_j Prod_{k!=j}(f_k+beta)  helper
    3. Z(wX)*(t+beta) = (Z(X)+h(X))*(t+beta) - m(X)  accumulator

  Commits: m, h, Z   |  Evals: m(x), h(x), Z(x), Z(wx)
```

### Vanishing (quotient polynomial)

```
  Collects ALL constraint evaluations into one polynomial:

    h(X) = (gate_constraints + perm_constraints + lookup_constraints + ...) / (X^n - 1)

  h(X) is split into degree-(n-1) pieces, each blinded for ZK.

  Commits: random blinding poly, quotient pieces
  Evals:   random(x), h_pieces(x)
```

### Trash (conditional zeroing)

```
  For constraints that should only hold conditionally.
  A "trash column" absorbs non-zero values when the constraint is inactive.

  Commits: trash poly   |  Evals: trash(x)
```


## Polynomial Representations & Conversions

```
                         coeff_to_lagrange (FFT)
        Coeff  ---------------------------------------->  LagrangeCoeff
    (coefficient form)  <-------------------------------  (evaluation form)
          |              lagrange_to_coeff (iFFT)             |
          |                                                   |
          |  coeff_to_extended                                |
          |  (coset FFT to larger domain)                     |
          v                                                   |
   ExtendedLagrangeCoeff                                      |
   (extended coset evals)                                     |
   Used for constraint                                        |
   evaluation & quotient                                      |
   construction                                               |
                                                              |
   KZG commit works on both: ---> commit(Coeff)               |
                              ---> commit_lagrange(LagrangeCoeff)
```


## Transcript Protocol (challenge schedule)

```
  VK hash
    |
    v
  [instance commitments]        (if committed-instances)
    |
    v
  [advice commitments phase 1]
  [advice commitments phase 2]  (if multi-phase)
  [advice commitments phase 3]  (if multi-phase)
    |
    v
  squeeze theta  -----------------> lookup column compression
    |
    v
  [multiplicity commitments]        one per FlattenedArgument
    |
    v
  squeeze beta   -----------------> permutation & logup
  squeeze gamma  -----------------> permutation
    |
    v
  [permutation product Z commits]
  [logup helper h + accumulator Z commits]
    |
    v
  squeeze trash_challenge --------> trash argument
    |
    v
  [trash poly commitments]
    |
    v
  squeeze y      -----------------> combine all constraints via Horner
    |
    v
  [vanishing: random poly commit]
  [vanishing: quotient piece commits]
    |
    v
  squeeze x      -----------------> evaluation point
    |
    v
  [all polynomial evaluations at x (and wx where needed)]
    |
    v
  [multi-opening proof (KZG)]
```
