# Repeated-cut prerequisites

The current raw-input segment APIs can classify intersections, order event
parameters on one directed segment, retain exact rational proper-crossing
points, compare those points with each other and with binary64 input points,
and produce tight parameter and crossing-coordinate enclosures. These are
bounded first-generation constructions. `SegmentSpan2` can repeatedly split a
parameter span on one original raw input segment at retained parameters. It
does not accept an exact crossing point as a new segment endpoint or intersect
arbitrary constructed segments, and an enclosure is not an exact point identity.
`orient_constructed2d` handles raw and first-generation crossing points under a
fixed caller budget; see [its contract](constructed-predicates.md).

## Required arithmetic boundary

Repeated intersections can increase numerator and denominator bit lengths at
each construction. The fixed capacity proofs for original binary64 inputs do
not bound arbitrary chains. A production repeated-cut representation needs:

- Exact rational or homogeneous coordinates with positive denominators and
  equality/order established by canonicalization or exact cross-products.
- Checked arithmetic with explicit bit and work budgets. Exhaustion must return
  a typed limit result before publishing a partial topology update; it must not
  round, truncate, or report a false disjoint result.
- A reviewed choice between a safe, bounded internal integer backend and an
  audited dependency. Review license, provenance, build dependencies, unsafe
  code and the trusted computing base before adoption. The current crate remains
  dependency-free; this document does not approve a backend.
- General predicates and intersections accepting retained constructed inputs,
  not rounded `Point2` substitutes. First-generation orientation now exists;
  arbitrary rational and repeated-cut inputs do not. Evaluation and enclosures
  must stay distinct from identity and incidence decisions.

## Arrangement and validation requirements

An arrangement can use exact event order to split source segments, but it must
also assign shared point identities across different sources, merge proven-equal
events, distinguish overlaps from isolated contacts, and preserve direction and
source provenance. Point segments have no directed parameter and need explicit
incidence handling. Construction should operate in a draft and publish only after
all required comparisons, splits and structural checks succeed within budget.

No arrangement or general rational intersection API is provided by the current
crate. The original-support span split is narrower than general repeated cuts.
Required evidence includes independent rational oracles for
second and later cuts, equivalent unreduced representations, exact equality
across distinct sources, collinear overlap and coincident events, exponent and
denominator growth, budget exhaustion, cancellation and atomic failure. A
hosted target matrix and independent review remain separate release gates.
The preliminary [backend evaluation](arithmetic-backend-evaluation.md) records
licensing, dependency and unsafe-code questions without approving a backend.
