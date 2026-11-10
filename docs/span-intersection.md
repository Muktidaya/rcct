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

The representation retains first-generation parameters on the original support;
it does not accumulate numerator or denominator growth as spans are split. Each
query uses the existing bounded raw-segment classifier and fixed-width exact
parameter comparisons. The parameter comparison's 132-limb capacity argument
is documented in [segment event witnesses](crossing-witness.md).

This interface does not intersect two arbitrary constructed spans or accept a
point cutter. It does not implement general second-generation rational
intersections, shared arrangement identities or an arrangement. The independent
fixtures and focused edge tests are finite evidence, not a formal proof.
