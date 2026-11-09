# Numerical contract and verification

This document describes the implemented `gp` operations in rcct 0.1.1. It is
not a specification for a future CAD kernel. The implementation is original
Apache-2.0 Rust using elementary Cartesian formulas; no OCCT implementation,
test data, or translated source is used.

## Coordinates, units, and comparisons

All coordinates use one right-handed Cartesian frame. `Point3` is a location;
`Vector3` is a displacement. Subtracting points gives a vector. Adding a vector
to a point gives a point. Points cannot be added to points or scaled directly.
The caller chooses units and must ensure compatible axes and operands. A length
has coordinate units; a dot or cross product of two displacements has squared
coordinate units. A normalized direction is dimensionless. No unit conversion,
model scale, snapping threshold, or geometric tolerance is hidden in the API.

Constructors and public point/vector fields accept all `f64` values. `is_finite`
checks all three components; it does not establish valid geometry. `PartialEq`
compares components exactly, including `+0.0 == -0.0` and `NaN != NaN`.
NaN payloads and signs are not stable API output. No operation here is an exact
or adaptive predicate for coincidence, orthogonality, parallelism, or orientation.

## Operation contracts

| Operation | Evaluation | Exceptional behavior and conditioning |
| --- | --- | --- |
| Point/vector addition and subtraction | Independent component operations | Rounded results; finite inputs can overflow. Cancellation or prior coordinate rounding can erase a small displacement. |
| Vector scalar multiplication/division | Independent component operations | No zero-divisor guard. Nonzero divided by signed zero produces signed infinity; zero divided by zero and infinity divided by infinity produce NaN. Products and quotients can underflow to signed zero. |
| Vector negation | Negate each component | Flips signed zero; no geometric canonicalization. |
| Dot product | `(x*bx + y*by) + z*bz` | Separate products and sums, without compensation or fusion. Overflow, underflow, and cancellation can occur before the result. Finite inputs can produce NaN after opposite infinities. |
| Cross product | `(y*bz-z*by, z*bx-x*bz, x*by-y*bx)` | Right-handed; separate products and differences. Near-parallel vectors can have poorly determined results. Finite equal vectors can produce NaN if the products overflow. |
| Norm | `x.hypot(y).hypot(z)` | Avoids unnecessary squaring overflow/underflow but not an unrepresentable final length. Accuracy depends on Rust's platform implementation of `hypot`. |
| Point distance | `(other - self).norm()` | Subtraction happens first. A robust norm cannot recover an overflowed or rounded-away separation. |
| Unit direction | Finite check, zero check, maximum-magnitude scaling, norm, division | Rejects all nonfinite inputs and all signed-zero vectors; accepts every finite nonzero vector without a near-zero cutoff. |

### Norm classification

Each `hypot` gives infinity precedence over NaN. Consequently an infinite input
component always gives positive infinity. Even without infinite inputs,
`Vector3::new(f64::MAX, f64::MAX, f64::NAN).norm()` gives infinity because the
first `hypot` overflows before the NaN is combined. Otherwise NaN yields NaN.
An all-zero input yields positive zero. Finite lengths can be subnormal.

Rust specifies unspecified precision for `f64::hypot`; it may vary by platform,
Rust version, or invocation. rcct does not promise correct rounding or a fixed
universal ULP bound. Standard compilation and ordinary floating-point execution
are assumed; altered floating-point control modes are not qualified here.

### Normalization invariant

For a finite nonzero vector `v`, let `m = max(abs(v.x), abs(v.y), abs(v.z))`.
The implementation computes `s = v / m`, then `s / s.norm()`. Each scaled
component lies in `[-1, 1]`, and at least one has magnitude one. The mathematical
scaled norm is between `1` and `sqrt(3)`, subject to rounding. This avoids both
an overflowing original norm and a zero divisor, including at the smallest
subnormal input. Nonfinite input is rejected before testing for zero.

Private direction storage prevents callers from modifying the normalized
components. A returned direction is finite and approximately unit length; its
computed norm need not equal `1.0` exactly. Components tiny relative to `m` may
underflow to signed zero. Positive rescaling need not preserve identical output
bits, and no fixed angular-error bound is promised. A caller requiring a minimum
magnitude must check its own explicitly chosen threshold before construction.

The source comments preserve three maintenance constraints: validate every
component before `f64::max` can ignore a NaN, divide components directly rather
than multiply by a possibly overflowing reciprocal, and never reconstruct the
possibly overflowing original length just to normalize. These are reasons for
the existing evaluation order, not additional geometric guarantees.

## Accuracy is not conditioning

Cancellation can make a tiny exact dot or cross product unresolvable even when
all input components are finite. A relative-error test divided by that tiny
answer is therefore inappropriate as a general gate. Verification must instead
use a scale reflecting the products being accumulated, or restrict ULP checks
to suitable well-conditioned cases. Neither a small absolute residual nor a
passed test makes a computed sign a robust geometric predicate.

Similarly, a translation smaller than the spacing of stored coordinates can
vanish before a distance is computed. Larger floating-point types, local frames,
exact predicates, or caller-defined error policies may be needed by later
algorithms. They are not implemented implicitly by these primitives.

## Verification and scope

Independent reference fixtures, exceptional-value tests, and downstream/package
checks accompany this release. See [verification evidence](verification.md) for
methods, acceptance budgets, reproducible commands, and which platforms were
actually run. Numerical budgets in tests are regression acceptance criteria for
that corpus, not mathematical error bounds for every possible input.

The crate remains dependency-free and forbids unsafe Rust. No complete CAD
kernel, B-rep/Boolean operation, robust predicate, safety qualification, formal
proof, independent security audit, or certification is claimed. Tests are
finite evidence and do not establish suitability for safety-critical use.
