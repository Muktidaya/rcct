# Connected components and graph bridges

`analyze_bridges2` analyzes the undirected merged edges of an existing
`Arrangement2`. It returns a component index for each vertex and whether each
graph edge is a bridge: removing that edge increases the number of connected
components. Component indices are deterministic for the same graph ordering.
The empty graph has zero components.

```text
cycle A ---- bridge ---- cycle B
```

In this conceptual graph, removing the middle graph edge separates the two
cycles; removing one edge from either cycle does not. Coincident input segments
are already merged into one graph edge with multiple source uses. A merged edge
can be a graph bridge even when it has multiple source uses: this analysis asks
about removing the geometric graph edge, not removing one input source.

The implementation builds two adjacency entries per edge and uses an iterative
depth-first search. For a DFS tree edge from a parent to a child, it marks the
edge as a bridge exactly when the child's lowest reachable discovery index is
greater than the parent's discovery index. The explicit frame stack avoids
recursion. The result borrows the input arrangement and is returned only after
the full analysis succeeds.

## Limits and scope

`BridgeAnalysisLimits2` caps vertices, edges, adjacency visits, simultaneous
stack frames, and connected components. Exactly two adjacency entries are
scanned per graph edge. The adjacency limit is checked before traversal; stack
and component limits are checked before adding the corresponding item. These
are logical counts, not byte, total-instruction, allocator, or wall-time limits.
Reservations are fallible; recoverable errors return no partial analysis and do
not modify the graph. Process or allocator aborts are outside that guarantee.

This analysis does not remove bridges, assign faces, or identify bridges for
individual source uses. `build_faces2` still rejects non-simple walks; the
separate [bridge-aware face builder](planar-faces.md) accepts a restricted
one-core-or-empty subset. Input geometry remains subject to
the [arrangement graph contract](planar-arrangement.md). Independent edge-removal
oracle tests are finite evidence rather than a formal proof.
