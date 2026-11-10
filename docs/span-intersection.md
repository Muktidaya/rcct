# Original-support span intersections

`SegmentSpan2` represents a nonempty closed parameter interval `[a, b]` on
one original, directed, nonzero finite-binary64 segment. `intersect_raw`
intersects that span with another nonzero raw segment without rounding the span
boundaries. The caller retains the span and cutter to preserve provenance.

For a single-point support intersection at parameter `t`, the event belongs
to the span exactly when `a <= t <= b`. A proper crossing of the original
support becomes a boundary touch when `t` equals `a` or `b`. For an overlap
with support parameters `[u, v]`, clipping uses exact comparisons:

| Clipped bounds | Result |
| --- | --- |
| `max(a, u) > min(b, v)` | Disjoint |
| `max(a, u) = min(b, v)` | Boundary touch |
| `max(a, u) < min(b, v)` | Overlap |

Parameters are ordered in original-support traversal order. Reversing that
support reverses the spatial order. An overlap boundary can be a non-dyadic
span parameter, so the returned boundary is an exact parameter, not a rounded
`Point2`. Retain the original witnesses if constructed point identity is needed.

`SpanQueryBudget` limits completed queries, including disjoint results. A zero
budget returns `QueryLimitExceeded` before the query and leaves the budget and
inputs unchanged. A completed query consumes one unit and never mutates topology.
This is a query count, not a wall-time, instruction, allocation or caller-storage
limit. `SpanSplitBudget` separately limits successful splits.

## Two retained spans

`intersect_span` accepts two spans whose endpoints are retained parameters on
original nonzero binary64 supports. It intersects the raw supports first. A
single support intersection is kept only when its exact parameters lie inside
both spans. It is a proper crossing only when it lies in both span interiors
and is a proper crossing of the supports. A point event retains parameters on
both supports.

For a positive collinear support overlap, the implementation chooses a
coordinate that varies on the first support. The two span intervals are ordered
on that axis in first-support traversal order and clipped as above. A collinear
point contact retains parameters on both supports; an overlap retains each
boundary as a parameter tagged with its supplying span. The overlap boundaries
can originate on different supports. A returned numerical parameter is not
automatically available on the other support. The located-point path below
can instead retain that exact point and split the other support.

`SegmentParameter2::compare_point_lexicographically` and `same_point` compare
points retained on different supports without rounding. They establish point
identity and order but do not perform reparameterization or construct a new
arbitrary rational point. They do not consume `SpanQueryBudget`; their fixed
arithmetic bounds come from the first-generation parameter construction.

The representation retains first-generation parameters on the original support;
it does not accumulate numerator or denominator growth as spans are split. Each
query uses the existing bounded raw-segment classifier and fixed-width exact
comparisons. The parameter comparison uses 132 limbs. Across supports, each
weighted coordinate numerator is below 2^6298 and each positive denominator is
below 2^4199; cross-products are below 2^10497 and fit in 165 limbs. These
bounds apply to retained parameters from raw inputs, including endpoint
parameters, because the numerator is between zero and the denominator. See
[segment event witnesses](crossing-witness.md) for the derivation.

## Locating overlap boundaries on another support

`LocatedPoint2` pairs a raw target segment with a retained first-generation
parameter from any original support. The retained parameter continues to name
its original support; it is not a numerical parameter on the target. Its point
is proven to lie on the target by exact orientation and closed coordinate bounds.
Once located, points compare in target traversal order by their retained exact
coordinates. The target may be reversed independently of the point's origin.

| Operation | Result and budget |
| --- | --- |
| `LocatedPoint2::locate` | Returns a located point or `None` after one completed incidence check; `SpanLocationBudget` charges either result. |
| `LocatedSpan2::split_at` | Returns two immutable spans at a strictly interior located point; `SpanSplitBudget` charges successful splits. |
| `LocatedSpan2::intersect_span` | Clips the raw-target event to both spans; `SpanQueryBudget` charges every completed query, including disjoint results. |

For an overlap, take each returned boundary's retained parameter, locate it on
the other raw target, and split that target's span at the located point. A
located span can then be intersected again while its endpoints retain their
original first-generation origins. Provenance is not rewritten and numerator or
denominator widths do not grow through these transfers. Exhausted budgets return
before their respective operations and leave inputs and budget unchanged; the
caller must stage multiple operations to make a larger topology update atomic.

An incidence check includes an exact orientation with the 330-limb determinant
bound in [bounded constructed predicates](constructed-predicates.md), plus
fixed-width coordinate comparisons. A location unit counts one completed check;
it is not a total limb-operation, instruction, memory or wall-time limit. Split
and query units similarly count operations rather than bounding caller storage.
`compare_on_target` and span construction do fixed-width work without charging
these budgets; rejected split attempts are also uncharged. Callers must bound
their own attempts and storage when those limits matter.

This interface does not accept arbitrary rational segment endpoints or a point
cutter. It does not materialize numerical overlap parameters on another target,
implement general second-generation rational intersections, or build an
arrangement. Exact point comparison does not by itself assign or manage shared
arrangement identities. The tests are finite evidence, not a formal proof.
