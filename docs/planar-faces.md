# Faces and aggregate winding for simple boundaries

`build_faces2` groups the boundary walks of an `ArrangementEmbedding2` into
faces. This path accepts only embeddings in which **every** boundary walk is a
simple closed walk. A bridge, short walk, or repeated-vertex boundary causes
`UnsupportedNonSimpleBoundary`; no partial face grouping is returned. The empty
graph has one face: the unbounded face at index zero.

Every counterclockwise walk supplies the outer boundary of one bounded face.
Each clockwise walk belongs to the innermost counterclockwise walk that strictly
contains a point of it, or to the unbounded face when none does. If multiple
containing candidates cannot be established as strictly nested, the build
returns `AmbiguousContainment`. A boundary contact is not counted as strict
containment. Walk-to-face assignments refer to the face on the walk's left.

For example, two disjoint nested rings produce four walks and three faces:

```text
unbounded | outer ring | annulus | inner ring | inner face
             CW -> unbounded     CW -> annulus
             CCW -> annulus      CCW -> inner face
```

Each ring's two walks traverse the same edges in opposite directions. The
diagram shows nesting, not a linear geometric layout. The annular face owns the
outer ring's counterclockwise walk and the inner ring's clockwise walk.
Disconnected rings, bounded subdivisions and multiple holes use the same rule.

Containment uses exact half-open ray crossings against the candidate walk.
Coordinates remain retained first-generation constructions, and edge orientation
uses the original raw support. No floating enclosure is used for incidence or
containment. This relies on the input graph's exact splitting and the simple
walk requirement; general self-touching boundaries are deliberately unsupported.

## Aggregate winding

`face_windings2` computes one aggregate integer winding value per grouped face
for all directed source uses together. The unbounded face starts at zero. For
each edge, oriented according to its stored endpoint order, the propagation
constraint is:

```text
winding(left face) = winding(right face) + signed source uses
```

A use following that edge order contributes +1; a reverse use contributes -1.
Opposite coincident uses cancel, and matching uses accumulate. Constraints are
checked throughout the graph; an open or otherwise inconsistent aggregate chain
returns `InconsistentChain`. Integer arithmetic is checked and may return
`ArithmeticOverflow`. This computes face winding for the aggregate chain, not
per-source winding, arbitrary point-on-boundary classification, or a Boolean
operation. Fill rules and region selection remain caller decisions.

## Limits and failure

Face grouping caps input half-edges and walks, output faces, walk-orientation
comparisons, containment queries, and boundary-edge checks. Winding caps input
faces, half-edges, source-use records, and edge-constraint checks during
propagation. These limits bound the named logical counts. They do not promise
exact bytes, a limit on every arithmetic operation, allocator behavior, or wall
time. Scratch allocations are requested fallibly; process or allocator aborts
are outside recoverable-error guarantees.

Before classifying a walk of length `m`, face grouping conservatively reserves
`m * (m - 1) / 2 + (m - 1)` orientation-comparison units, the full cost for a
simple walk. A non-simple walk may therefore hit that limit before it can be
rejected. Winding propagation charges each examined edge constraint, including
the final consistency pass.

Both builders stage their outputs and borrow their inputs. A recoverable limit,
allocation, unsupported-topology, inconsistency, or invariant failure returns no
partial result and leaves previous objects unchanged. See
[half-edge embedding](planar-embedding.md) and
[arrangement graph](planar-arrangement.md) for earlier-stage limits.

Self-touching or bridge boundaries, isolated point inputs, curves, arbitrary
constructed segments, face editing, and general Boolean operations remain
unsupported. Independent tests are finite evidence rather than a formal proof.
