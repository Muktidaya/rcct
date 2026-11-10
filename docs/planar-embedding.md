# Half-edge rotation and boundary walks

`embed_arrangement2` borrows a completed `Arrangement2` and returns a separate,
immutable `ArrangementEmbedding2`. Each undirected graph edge contributes two
half-edges with opposite origins and destinations. Their `twin` links are
mutual. For each vertex, `outgoing` lists half-edge indices counterclockwise
from the positive x direction in the caller's right-handed Cartesian frame.

Each graph edge retains a use by an original raw source. The source's direction,
adjusted for traversal, gives the exact ray direction of its half-edge. The
comparison uses exact integer determinants of raw binary64 coordinates rather
than floating-point subtraction or angles. Coincident pieces were already merged
by the graph builder; two equal outgoing rays would violate the expected graph
invariant and cause an error rather than an arbitrary tie break.

For the counterclockwise outgoing sequence at a vertex, the successor rule is:

```text
outgoing rotation:       e0, e1, e2, ...  (counterclockwise)
incoming half-edge:      twin(e1)
next(incoming):          e0              (clockwise predecessor of e1)
```

The same cyclic predecessor rule applies at every degree, including degree one.
It keeps the traversed side on the left and defines a permutation of all
half-edges. `boundary_walk` returns its closed orbits; each half-edge occurs in
exactly one walk. A bridge can occur in both directions in one walk. Nested or
disconnected components can produce multiple walks around one face, so walks
are **not** face identifiers and their count is not a face count.

## Orientation of a simple walk

`simple_walk_orientation2` rejects walks shorter than three edges or with a
repeated vertex, including walks that traverse a bridge. For a simple embedded
straight-edge polygon, its lexicographically least vertex (x, then y) is a
strict convex corner: neither adjacent vertex can lie to its left, and ties in
x cannot lie below it. At that corner the exact turn of incoming and outgoing
raw-support directions has the polygon's clockwise or counterclockwise sign.
This avoids accumulating an unbounded rational signed-area sum.

This result classifies a single simple polygonal walk. It is not a face
assignment, a hole-containment test, an area, or a point winding number.

## Resource and failure contract

| Limit | Meaning |
| --- | --- |
| `EmbeddingLimits2::max_vertices` | Input graph vertex count. |
| `max_half_edges` | Exactly twice the input graph edge count. |
| `max_edge_uses` | Source-use records scanned for direction representatives. |
| `max_comparisons` | Calls during radial sorting and duplicate-ray checks. |
| `max_boundary_walks` | Number of returned `next` orbits. |
| `max_walk_steps` | Half-edges visited in the orbit partition. |
| `WalkOrientationLimits2::max_steps` | Half-edges in one walk being classified. |
| `WalkOrientationLimits2::max_comparisons` | Repeated-vertex and exact extremum comparisons. |

The radial sort and repeated-vertex check use bounded insertion/linear search;
their comparison limits are checked before the counted calls. Limits constrain
logical counts, not exact bytes, all limb operations, allocator behavior or
time. Reservations are fallible; process or allocator aborts are not
recoverable errors. An exhausted limit or other recoverable failure returns no
partial embedding or orientation and leaves the original graph and embedding
unchanged. The embedding borrows its graph, so its indices cannot accidentally
be used through this object with a different graph.

The separate [face grouping stage](planar-faces.md) handles containment and
aggregate winding when every walk is simple. General self-touching boundaries
and arbitrary constructed-segment inputs remain unsupported.
Independent tests are finite evidence rather than a formal proof. See the
[arrangement graph contract](planar-arrangement.md) for its own limits.
