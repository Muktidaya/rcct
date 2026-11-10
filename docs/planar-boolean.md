# Bounded face-based planar Boolean regions

This stage operates on an existing raw-segment arrangement, half-edge embedding,
and successful simple-boundary face grouping. It selects two-dimensional faces
and returns their oriented boundary walks. It is a **regularized face-based 2D
result**: isolated point or edge contacts are not retained as standalone results.
For example, two squares sharing only an edge have an empty 2D intersection in
this representation. It does not define point-on-boundary membership or produce
an independent editable polygon or a 3D solid.

## Group winding and selection

| Stage | Input and result |
| --- | --- |
| `group_windings2` | Assign each original source to one group or `None`; compute an independent integer winding per face and group. |
| Fill rule | `NonZero` selects any nonzero winding; `EvenOdd` selects odd winding, including negative odd values. |
| `select_boolean2` | Apply union, intersection, first-minus-second, or symmetric difference to two groups' selected faces. |
| Boundary tracing | Keep each edge with a selected face on exactly one side and orient its half-edge with the selected face on the left. |

Each group must independently form a consistent closed directed chain; sources
assigned `None` contribute to neither winding nor selection. Opposite coincident
uses within one group cancel, while matching directions accumulate. Inconsistent
groups return their index in `InconsistentGroup`. Each source has one group at
most; shared geometry needed in multiple groups must be supplied as separate
source entries. Winding arithmetic is checked.

At a vertex, tracing takes the clockwise predecessor among outgoing selected
boundary rays from the incoming half-edge's twin. The builder verifies that each
selected half-edge has at most one predecessor and that all returned walks close.
Walks can share or revisit vertices; they are not promised to be simple rings.
The output borrows its face grouping and retains graph half-edge indices, so
provenance remains available through the original arrangement.

## Resource and failure contract

`GroupWindingLimits2` caps source assignments, groups, faces, half-edges,
source-use records, combined face/group and edge/group winding cells, and
edge/group propagation checks. A known-value flag is also stored per face/group
cell; its logical length is bounded by the same cell cap.

`BooleanRegionLimits2` caps faces, vertices, half-edges, selected boundary
edges, returned walks, traced edges, and outgoing-ray successor checks. Each
successor check is charged before it runs. Counts bound the named operations and
logical items, not exact bytes, all arithmetic, allocator behavior or wall time.
Allocations use fallible reservations; process or allocator aborts are outside
recoverable-error guarantees. Errors return no partial result and do not mutate
their inputs.

## Scope

All earlier stages retain their limits. In particular, face grouping rejects
bridge and other repeated-vertex boundary walks, including point-tangent input
loops. Collinear overlaps, coincident edges, shared-edge subdivisions, and
crossings are supported when the resulting face walks satisfy that contract.
Inputs remain nonzero raw binary64 straight segments. Curves, arbitrary rational
segment inputs, lower-dimensional Boolean remnants, unbounded complements,
per-source Boolean decomposition, and general CAD/B-rep Boolean
operations are not provided. Independent oracles and metamorphic tests are finite
evidence rather than a formal proof.
