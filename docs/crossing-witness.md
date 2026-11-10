# Exact segment event witnesses

This construction covers proper crossings, single-point endpoint touches and
positive-length collinear overlaps of two nonzero straight segments whose
endpoints are finite binary64 values. `proper_crossing_witness`
retains the input segments and the exact parameter `t` on the first directed
segment. The crossing is defined as `a + t(b-a)` in exact arithmetic; the API
does not round it to `Point2`.

For `r=b-a`, `s=d-c`, and `q=c-a`, the parameter is
`cross(q,s)/cross(r,s)`. Proper crossing ensures the denominator is nonzero and
`0 < t < 1`. The implementation normalizes the denominator's sign. The
numerator and denominator need not be reduced: equality and ordering compare
cross-products exactly. `compare_on_first` requires identical directed first
segments; a reversed or different source returns `None`.

`endpoint_touch_witness` retains both segments and an original input endpoint
at the contact, including collinear segments that meet at a single point. It
computes a first-segment parameter as the exact ratio of coordinate differences
on an axis where the first segment varies. `SegmentParameter2` compares the
parameters from both witness types on the same directed source segment. Point
coordinates retain their original binary64 values; signed zeros compare equal.

`overlap_witness` returns the two boundary points and their exact parameters in
the direction of the first segment. Both boundaries are selected from the four
original endpoints: the intersection of two closed collinear finite segments is
bounded by input endpoints. Reversing the first segment reverses the returned
boundary order. Equal signed-zero coordinates are geometrically identical; the
returned zero sign is not an identity guarantee.

`SegmentParameter2::enclosure` returns the tight inclusive binary64 bounds for
the exact parameter in `[0, 1]`: either the same value twice if representable,
or the adjacent representable values immediately below and above it. It uses
integer comparisons and a bit-pattern search; no floating-point division or
process-global rounding-mode change is involved. These are parameter bounds,
not a coordinate enclosure for the constructed crossing.

## Representation decision and capacity

Rounding a crossing to binary64 loses identity, as a crossing can be at 1/3.
Arbitrary-precision rational dependencies would support broader constructions,
but require a separate dependency and unsafe-code review and growth policy.
An implicit source-only witness postpones arithmetic but cannot order events
without recomputing it. This slice retains the source and a bounded exact raw
input parameter. It adds no dependency or general-purpose rational API.

Each finite binary64 coordinate is an integer multiple of `2^-1074`, with a
magnitude coefficient below `2^2098`. Coordinate differences are below
`2^2099`, so a two-product determinant is below `2^4199`. Each numerator and
denominator fits in 66 64-bit limbs. Comparing two such parameters needs two
products below `2^8398`, which fit in 132 limbs (8448 bits). These bounds apply
only to determinants formed from original finite binary64 coordinates. They do
not apply to repeated cuts or arbitrary rational constructions. Carry and
capacity behavior is not formally proved. For a limb product, the largest
`u64 * u64` term plus an existing output limb and a carry is at most
`2^128 - 1`; it fits in `u128`. An endpoint parameter uses coordinate
differences below `2^2099`, so its products with either kind of parameter also
fit the 132-limb comparison capacity. These bounds depend on the raw finite
input contract and the stated integer representation.

To enclose `n/d`, a candidate binary64 value in `[0,1]` is represented as
`X * 2^-1074`. The integer comparison is `n * 2^1074` versus `d * X`.
Both values are below `2^5273` for raw-input determinant parameters, and the
implementation allocates 99 limbs (6336 bits). The generic limb multiplication
also fits: 66 predicate limbs times 33 coordinate limbs need at most 99 limbs.
The ordered positive binary64 bit patterns between zero and one permit a
bounded binary search, including subnormal and exact endpoint cases.

Zero-length segments, constructed crossing coordinates and their enclosures,
ordering across different source segments, rational inputs,
and repeated constructed cuts are not provided by these APIs. `None` from
`proper_crossing_witness` means the intersection is not a proper crossing;
`None` from `endpoint_touch_witness` means it is not a single-point touch.
`None` from `overlap_witness` means it is not a positive-length overlap.
Use the existing classifier to distinguish other events. Future work must
address those cases explicitly before treating the segment milestone as complete.

`tests/crossing_witness.rs` checks equal unreduced fractions, ordering,
different source segments, endpoint contacts, extreme exponents, and 768 cases
whose expected order is independently generated with Python `Fraction`.
`tests/planar_segment_oracle.rs` compares endpoint, overlap-boundary and
crossing parameters on the small integer grid against a separate integer
parametric oracle. Parameter bounds are checked against 1,028 independently
generated Python `Fraction` cases, including general crossings, exact values,
subnormal boundaries and extreme ratios. The tests are finite evidence rather
than a proof or qualification.
