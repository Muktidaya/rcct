# Initial planar geometry slice

This note specifies the first finite 2D geometry and topology API in the
unreleased `0.2.0-alpha.1` development line. It is a bounded foundation for
later modeling work, not a 2D CAD application or a complete topology kernel.

## Input contract

`Point2::try_new(x, y)` admits only finite binary64 coordinates. The values use
one caller-owned right-handed Cartesian frame and caller-chosen units. The API
does not rescale coordinates, merge nearby points, or apply a tolerance.
`Segment2::try_new(start, end)` rejects endpoints that compare equal. Since
`Point2` fields are private, checked construction is the only way to obtain a
point or segment.

Equality is exact `f64` equality. Thus `+0.0` and `-0.0` compare equal, while
nearby representable values remain distinct. This is identity of stored
coordinates, not an assertion about physical measurement accuracy.

`Aabb2` stores ordered finite lower and upper corners. It permits point and
line boxes, treats its boundary as included, and can form unions, intersections,
or the smallest box containing a nonempty collection of `Point2` values.
These operations select existing coordinates without arithmetic. They do not
claim conservative bounds for transformed or curved geometry.

## Exact orientation

`orient2d(a, b, c)` returns the sign of

```text
(b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
```

for the exact real values represented by the input `f64`s. It returns clockwise,
collinear, or counterclockwise. No floating-point subtraction or product is
used in the determinant evaluation.

Every finite binary64 value is an integer multiple of `2^-1074`. Its integer
coefficient needs at most 2098 magnitude bits. The implementation stores that
coefficient in 33 `u64` limbs, forms signed coordinate differences, and performs
products and determinant subtraction in 66 limbs. This covers the full finite
binary64 exponent range, including opposite-sign maximum values, subnormals,
product overflow, and product underflow. The fixed-width arithmetic is internal
and dependency-free.

The result is exact only for orientation of the supplied coordinates. It does
not make a subsequently constructed intersection coordinate exact, and it does
not imply exactness for curves, surfaces, or 3D predicates.

## Segment events

`intersect_segments(a, b)` classifies the closed-set intersection as:

- `Disjoint` if there is no shared point;
- `Point(ProperCrossing)` if both segment interiors cross with strict opposite
  orientation signs;
- `Point(BoundaryTouch)` if a segment endpoint touches the other segment, or
  the segments share exactly one endpoint;
- `Overlap` if collinear segments share an interval of positive length.

Endpoints and interval ordering use exact finite `f64` comparisons. The API
reports the event class, not a constructed intersection coordinate or an
endpoint identity. Proper crossing coordinates need a separate construction
and error policy; this API avoids returning a rounded value that callers might
mistake for an exact witness.

## Simple closed wires

`SimpleWire2::try_new(vertices)` treats the vertices as a traversal-order cycle
with an implicit final edge to the first vertex. The first vertex must not be
repeated at the end. A valid wire has at least three vertices, no zero-length
edge, and no intersection between nonadjacent edges. Adjacent edges may share
only their common endpoint; adjacent collinear backtracking is rejected as an
overlap. Straight-through collinear vertices are allowed.

Validation compares every edge pair, so it takes O(n²) exact segment-event
checks and stores the original vertices. This is intended for the initial
topology foundation and bounded wire validation. Spatial acceleration, curves,
face charts, hole containment, shells, solids, and Boolean operations are
outside this slice.

## Evidence and limits

Integration tests compare orientations for all triples from a small integer
grid against an `i128` determinant and check 256 deterministic binary64 triples
against an independent Python `Fraction` oracle. They also cover maximum finite
coordinates, subnormal products, and finite-coordinate subtraction overflow.
Segment tests cover disjoint, crossing, endpoint-touch, collinear-touch, and
overlap cases. Wire tests cover valid convex/concave cycles, collinear
intermediate vertices, degenerate edges, adjacent overlap, proper self-crossing,
and nonadjacent touch.

This evidence is finite regression coverage, not a formal proof or safety
qualification. The package keeps its own `#![forbid(unsafe_code)]`; there are no
new dependencies and no OCCT source, tests, or data are incorporated.
