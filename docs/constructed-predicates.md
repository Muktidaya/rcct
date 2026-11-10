# Bounded constructed-point operations

`orient_constructed2d` accepts finite `Point2` values and first-generation
`ExactCrossingPoint2` values constructed directly from finite binary64 segment
inputs. It evaluates the orientation sign exactly on those retained values;
it does not turn their floating enclosures into point identities. It does not
accept arbitrary rational inputs or intersections of constructed segments.

For each coordinate, a point is represented as `X / (D * 2^1074)` with
positive `D`. Input points use `D = 1`; first-generation crossings have
`|X| < 2^6298` and `D < 2^4199`. A coordinate-difference numerator such as
`Xb*Da - Xa*Db` is below `2^10498` in magnitude and fits 165 limbs. The two
products in the orientation determinant are each below `2^20996`; their
difference is below `2^20997` and fits 330 limbs (21120 bits). All omitted
denominator factors are strictly positive. These bounds apply only to the
documented first-generation input domain, not arbitrary repeated rational cuts.

The caller provides `ConstructedPredicateBudget`. One orientation is charged a
conservative fixed maximum of 106,722 limb multiplications: eight 99-by-66 and
two 165-by-165 products. This is not a wall-time, allocation, total-instruction
or memory budget. The fixed-width operation itself allocates no variable-size
integer storage. An insufficient budget returns `WorkLimitExceeded` before
arithmetic and leaves the budget and inputs unchanged. A successful result
consumes the fixed charge. No topology is mutated by this predicate.

`SegmentSpan2` provides a narrower repeated-split operation on the parameter
domain of one original directed input segment. It can split an existing span at
another supported exact parameter from that same source. It does not convert
constructed points into segment inputs or perform general rational segment
intersections. `SpanSplitBudget` caps successful splits; it does not limit
rejected calls or caller-owned storage. Splits return two new spans, and any
error leaves both the source span and budget unchanged. Callers must retain
separate witnesses for cutter provenance and decide how to publish topology.

The mixed constructed-point orientation corpus uses an independent Python
`Fraction` oracle. Tests also cover equal constructions, overlapping floating
enclosures with different exact values, finite exponent extremes, repeated
one-third/two-thirds splits, source mismatch, boundary cuts, and exhausted
budgets. The finite corpus and the stated capacity derivation are evidence,
not a formal proof or safety qualification.
