# Convex polyhedral Boolean milestone

RCCT 0.2.0 is in development. The experimental `polyhedral` API implements a
bounded convex halfspace path and indexed planar-face boundary admission.
Broader geometry, B-rep and Boolean operations remain
in scope. The design requirements and acceptance cases below apply to the
milestone and still require qualification on a specified revision.

## Using the current API

1. Construct each oriented plane with `Plane3h::from_coefficients([a, b, c, d])`.
   The four `i64` values define the closed halfspace `ax + by + cz + d <= 0`.
   A zero normal is invalid. Positive scaling preserves the material side,
   and sign reversal switches sides. Rational coefficients can be represented
   when clearing denominators by a positive factor yields a row fitting `i64`.
2. Pass the planes and a `PolyhedralWork` meter to `ConvexSolid::from_halfspaces`.
   Admission requires a bounded convex intersection with positive interior.
   Redundant planes are allowed. An infeasible set of constraints is an input
   error. Use `ConvexSolid::empty()` for the explicit empty operand.
3. Apply `boolean3` to two admitted operands with `Operation3::Union`,
   `Intersection` or `Difference`. A successful `SolidBoundary` owns its
   vertices, oriented face loops and supports. Its coordinates and volume
   are accessible as reduced rational pairs. Face counts can include
   coplanar subdivisions, and each cavity contributes a boundary shell.
4. For a subsequent operation, call `SolidBoundary::as_convex`. It tests
   vertices against outward face halfspaces and rebuilds the convex operand.
   A nonconvex result can be inspected but fails convex re-admission.

Construction uses checked `i128` rational arithmetic from admission through
boundary assembly. No step converts a constructed point to binary64. Even
when the input coefficients fit, an intermediate can exceed capacity and
return `ArithmeticLimit`. The work meter limits operation checkpoints and
cumulative requested vector storage, with cancellation checked cooperatively.

The [convex clipping example](../examples/convex_clip.rs) constructs both
operands and inspects the rational volume.

## Supplying an indexed boundary

Use `SolidBoundary::from_indexed_faces(&vertices, &faces, &mut work)` for a
boundary already described by planar faces and shared vertices. Each point
has three `(i128, i128)` coordinate pairs in numerator, denominator order.
Each denominator must be nonzero. Admission reduces each fraction and moves
its sign to the numerator. Distinct vertex indices must denote distinct
points after reduction, and every point must appear in a face. Two empty
tables represent an empty boundary.

Each `IndexedFace { plane, vertices }` supplies its oriented `Plane3h` and
a cyclic slice of vertex indices. Supply each loop vertex once. The final edge
joins the last index to the first. A face must enclose positive area, be convex,
lie on its plane and wind counterclockwise when viewed from outside the material.
Adjacent faces must reference the same endpoints in opposite directions.
For a subdivided seam, both loops must include each subdivision vertex. A duplicate point
under a different index, an unsplit T-junction or an open seam fails admission.

The resulting owned boundary can enclose disconnected components and cavities.
Each shell must bound material on the correct side for its nesting depth,
so a cavity wall faces into the cavity. The checked arithmetic, work, storage
and cancellation limits above apply throughout admission. To use the result
in `boolean3`, first call `as_convex`. A valid nonconvex boundary returns
`UnsupportedNonconvex` at this step. Curved and trimmed surfaces, general
nonconvex Boolean operands and persistent CAD file formats remain open.

## Design requirements for the wider milestone

For future input paths that accept finite binary64 coordinates, those values
denote their exact dyadic coordinates. Subsequent rational or homogeneous constructions must retain exact
mathematical definitions. Rounded display coordinates and approximate spatial
plane offsets cannot authenticate equality, incidence, or material side.
Uncertain measurements and any healing/snap policy are separate inputs or
operations; they do not silently redefine topology.

For `r(X) = closure(interior(X))`, the proposed operations on closed solids are
`r(A ∪ B)`, `r(A ∩ B)`, and `r(A \ B)`. Lower-dimensional contact events are
preserved until aggregate validation, even when the regularized intersection is
empty. An unsuccessful or unresolved operation is not an empty result.

The initial operand domain is two nonempty, bounded, full-dimensional convex
polyhedra defined by exact rational planes, plus an explicit empty operand.
Admission must establish positive interior, boundedness, convexity, complete
finite face coverage, exact incidence and outward orientation. Specify whether
redundant supports and coplanar or collinear subdivisions are admitted or
normalized exactly; do not silently import rounded polygon soup.

## Results and nonmanifold contact

A successful result is either explicitly empty or an owned representation of
an embedded, closed, orientable manifold boundary, potentially with multiple
components and cavities. Validation applies to the entire represented set.
For example, two cubes touching only along an edge have an empty regularized
intersection, but their union cannot be accepted as two independent manifold
components to bypass the aggregate contact check. Under this proposed result
contract, that union returns a typed nonmanifold-result failure.

The output may be nonconvex. Convex-input support does not imply that a union or
difference result is admissible as a later convex operand. Reuse requires a
separate exact admission or conversion contract; general polyhedral operand
closure and exact persistence are later gates. This milestone must support reuse
of a nonempty convex intersection, including redundant coplanar subdivisions,
by documented admission or exact normalization. `Empty` needs explicit
identities for subsequent operations.

## Success obligations

```text
validated inputs
  -> complete point / curve / area intersection coverage
  -> exact event identity and common split refinement
  -> adjacent material-cell labels and fragment selection
  -> aggregate geometric and topological output validation
  -> publish empty or validated owned result
```

Success requires complete intersection discovery, justified construction and
identity, consistent splits shared across all incident faces, correct material
classification, and validated aggregate output. Coplanar support overlap and
lower-dimensional contacts need explicit paths. Volume, a small residual,
edge-use counts, or a cyclic vertex-link diagnostic alone do not prove those
obligations. A field named a certificate is not evidence without a checker and
stated assumptions.

Within the advertised milestone, copied or subdivided coincident plane supports
must be recognized exactly under nonzero proportional coefficient scaling,
including opposite signs, with orientation retained separately. Blanket rejection
of all coplanar cases does not meet this contract.

Until rational construction closure and rational-input planar subdivision are
implemented and qualified, do not convert an exact 3D construction through a
rounded `Point2` or use raw-input `orient3d` as a constructed-point predicate.
Existing retained planar witnesses avoid particular reparameterizations; they
are not closure under arbitrary repeated cuts.

## Failure, ownership and resources

Unsupported operand classes, invalid input, nonmanifold result, unresolved
coincidence/classification/event equality, inconsistent refinement, arithmetic
or allocation limits, and cancellation must remain distinguishable from
success. Inputs stay immutable; intermediate geometry and topology are private
until all required validation succeeds. An operation must own or retain its
exact construction dependencies throughout the result lifetime.

Budgets need to account for numeric bit growth, integer work, total live and
scratch storage, pair enumeration, events, fragments, and final validation.
Long operations should retain charged work on failure; a failed preflight that
performs no work may leave its budget unchanged when specified. A deadline is
cooperative, and bounded cancellation latency requires bounded primitives.
`try_reserve` does not guarantee recovery from every process-level OOM.

## Acceptance examples

These are acceptance targets. Passing evidence belongs to the exact revision and input domain tested.

- Idempotence: `A ∪ A = A`, `A ∩ A = A`, and `A \ A = Empty` without duplicate
  coincident boundaries.
- `[0,2]^3` and `[1,3]^3`: intersection volume 1, union volume 15 and difference
  volume 7, with actual topology and geometry validated in addition to volume.
- `[0,3]^3 \ [1,2]^3`: volume 26 and one solid with two oppositely oriented
  boundary shells, including the cavity.
- Partial coplanar faces, oblique planes and shared multi-face events: common
  refinements agree, without lost area overlaps or T-junctions.
- Face-only contact yields empty regularized intersection; edge/point-only
  contact requires the aggregate nonmanifold union policy above.
- Separate cubes yield a two-component union; full-face contact has no retained
  internal face. A slab subtraction may split the result into two components.
- Exact affine transforms and reflections preserve set relationships with
  appropriate orientation changes. Dyadic sliver, exact contact and gap remain
  distinct; distinct rationals with the same binary64 rounding remain distinct.
- Invalid indexed shells, reversed outer orientation, inconsistent face planes,
  missing faces and open seams receive typed admission errors. Exercise these
  cases through the public indexed constructor and check exact convex reuse
  followed by a Boolean operation.
- Each resource cap and cancellation path returns no partially validated result
  while recording work already consumed according to the operation contract.

The first milestone retains acceptance cases P01–P16 and P18–P20. P06
requires a typed nonmanifold result and P16 requires a typed failure for
nonconvex reuse. P17 exact persistence, general operand closure and curved
cases require separate qualification. Qualification of the
convex path must cover exact construction and refinement, face ownership
and geometric admission within its documented arithmetic and resource limits.
