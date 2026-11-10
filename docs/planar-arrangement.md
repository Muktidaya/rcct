# Bounded straight-segment arrangement graph

`build_arrangement2` creates an immutable embedded graph from an ordered slice of
nonzero finite-binary64 `Segment2` inputs. Its vertices are exact point
identities retained as first-generation `SegmentParameter2` values. Its edges
are unique undirected straight pieces between distinct vertex indices. Each
edge use identifies one input segment and whether that source traverses the
edge in its stored endpoint order. The endpoints of an edge are stored in
increasing **vertex-index** order, not geometric or source order.

For example, in this conceptual collinear arrangement the first source runs
from A to C and the second runs from D to B:

```text
A ---- B ===== C ---- D

source 0: A -> B -> C
source 1: D -> C -> B
```

The shared piece B--C is one edge with two uses, one from each source, in
opposite geometric directions. Input duplicates and partial overlaps are
represented the same way. Returned uses are grouped by input source and ordered
along that source; vertex and edge indices are deterministic for the same
ordered inputs but need not persist when inputs are reordered.

The builder enumerates every unordered pair of original supports. Endpoints,
proper crossings, touches and overlap boundaries become exact cuts on both
sources. It sorts and deduplicates cuts on each source, merges equal points
across sources, and merges pieces with the same pair of vertex identities.
Because every cut is derived directly from the original raw inputs, repeated
rational construction depth does not grow. `SegmentParameter2::point_enclosure`
provides tight binary64 coordinate bounds for display or evaluation; those
bounds do not replace exact point identity.

## Resource and failure contract

The caller supplies `ArrangementLimits2`:

| Limit | Counted work or storage |
| --- | --- |
| `max_segments` | Input segment count. |
| `max_pairs` | All `n * (n - 1) / 2` source pairs, including disjoint pairs. |
| `max_cuts` | Cut records before deduplication: two per input, two per point event and four per overlap event. |
| `max_vertices` | Distinct exact point identities. |
| `max_edges` | Distinct undirected edge pieces. |
| `max_edge_uses` | Source traversals over edge pieces. |
| `max_comparisons` | Calls used for sorting, cut deduplication, vertex search and edge search. |

The sort uses bounded insertion sort; point and edge searches are linear. The
comparison limit is checked before each counted comparison. Its units do not
count all integer limb operations, pair-classification arithmetic, copies,
allocator work, time or bytes. Item limits constrain logical lengths, not exact
allocated capacity or bytes. `point_enclosure` is a separate fixed-width
operation and is not charged to the build. The implementation uses fallible
reservations and reports `AllocationFailed`, but allocator or process aborts are
outside the API's recoverable-error guarantee.

All graph data stays in a private draft until the build succeeds. A limit,
allocation or invariant error returns no partial graph, and the input slice is
never changed. No cancellation interface or persistent topology transaction is
provided. Callers must choose limits appropriate to their resources.

## Scope

The graph handles overlapping and coincident raw segments by retaining source
uses. A separate [half-edge embedding](planar-embedding.md) provides radial
order and boundary walks. A [separate face stage](planar-faces.md) groups simple
walks and computes aggregate source winding. Isolated point inputs, curves,
arbitrary rational segment endpoints and general CAD Boolean operations remain
unsupported. The [face-based 2D Boolean stage](planar-boolean.md) has narrower
input and output contracts. Independent rational
graph tests provide finite evidence rather than a formal proof. See
[repeated-cut prerequisites](repeated-cut-prerequisites.md) for further gates.
