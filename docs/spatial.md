# Checked spatial primitives

`spatial` adds finite point storage, nonzero endpoint segments, closed finite
axis-aligned boxes, and line/plane representations. These are building blocks:
they do not establish 3D intersections, robust side predicates, topology,
B-rep, or solid Boolean operations. Coordinate frames and units are caller-owned.

A `FinitePoint3` rejects every nonfinite coordinate without changing the stored
values. A segment rejects exactly equal endpoints, including differences only in
signed zeros. Closed boxes allow zero width and face, edge, or point contact;
containment, union and intersection use comparisons on finite stored values.
These checks use no global tolerance. Signed zeros compare equal, with no promise
of canonical bit patterns. Derived equality compares stored representations:
reversing a segment or changing the origin along the same line can change the
representation without changing the geometric set.

`Line3` and `Plane3` reuse `gp::UnitDirection3`; normalization is approximate
and may erase relatively small components. Line evaluation checks separate
rounded products and sums. Plane signed offset checks each coordinate
subtraction, each product, and the left-associated sums. Nonfinite intermediate
or final values return a typed error, even when the exact mathematical result
would be finite. Finite values can underflow or have the wrong sign through
rounding and cancellation. Line parameters are only approximately distances;
plane offsets are neither exact distances nor reliable side/incidence predicates.
Algorithms needing topology must use a separately justified robust predicate.
