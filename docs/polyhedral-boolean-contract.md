# Proposed first 3D polyhedral Boolean contract

Status: design for future work. This document defines neither an implemented
Boolean operation nor a release claim. The intended rcct 0.2.0 scope remains
broad geometry, B-rep and Booleans. The initial convex-input milestone below
is a dependency-ordered subset of that goal.

## Model and operations

Authoritative finite binary64 input coordinates denote their exact dyadic
values. Subsequent rational or homogeneous constructions must retain exact
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

These are future tests, not passing results at this revision:

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
- Invalid candidate shells, reversed outer orientation, inconsistent face planes,
  missing boundary and open loops receive typed admission errors when the
  selected input path accepts B-rep candidates. If the initial public constructor
  accepts halfspaces only, equivalent malformed internal drafts must be rejected
  before publication; do not claim an arbitrary B-rep input API exists.
- Each resource cap and cancellation path returns no partially validated result
  while recording work already consumed according to the operation contract.

With sufficient resources, the first milestone must meet P01–P16 and P18–P20
in the independent RCCT research handoff; P06's nonmanifold result and P16's
unsupported nonconvex reuse are specified outcomes, not waived cases. P17 exact
persistence and general operand closure belong to the separately qualified
follow-on milestone. The handoff also specifies further curved cases. Exact
numeric backend selection, repeated-cut closure, robust planar subdivision, face ownership and geometric admission are
separate prerequisites; none is established by this design note.
