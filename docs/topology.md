# Combinatorial incidence and wires

The `topology` module is a bounded, coordinate-free incidence foundation. It
is not a B-rep model or a geometric validator. A graph owns an immutable list
of edges over contiguous vertex indices `0..vertex_count`; isolated vertices
and distinct parallel edges are permitted. Edges with equal endpoint IDs are
not supported. An edge ID is its index within that graph. Raw IDs from another
graph may happen to be in range and alias: callers must supply IDs from the
intended graph. The graph borrow keeps a constructed wire tied to its graph but
does not authenticate the origin of input integers.

An edge use pairs an edge ID with a traversal direction. A successful wire
owns a copied sequence of at least three uses and immutably borrows its graph:

```text
   use 0          use 1          use 2
v0 -----> v1 -----> v2 -----> v0
    end = next start       closure
```

All edge IDs must exist, every destination must match the next origin including
wraparound, and each origin ID must be distinct. Thus the supported result is a
vertex-simple directed combinatorial cycle. Single-edge loops and two-edge
cycles are intentionally outside this API, though they may be meaningful in a
future more general representation. Validation says nothing about coincident
coordinates, crossings, planarity, manifoldness, underlying curves, surface
trims, face orientation, shells, or solids.

Graph construction caps vertex and edge counts before scanning or allocation.
Wire construction caps uses and preflights `n*(n-1)/2` comparisons with checked
arithmetic; a count outside `usize` or the configured budget fails. Both reserve
storage fallibly and return no partially built object. Inputs and limits are
immutable. These caps bound logical items and comparisons, not allocator latency,
whole-process memory exhaustion, or total CPU instructions.

Future geometry-to-topology links need an explicit identity/provenance model,
curve and surface representations, consistent endpoint and trimming contracts,
and robust predicates and constructions before face, shell, or solid validation
can be claimed. This module makes no promise about those future APIs.

## Edge-use incidence across wires

`analyze_edge_incidence` counts uses in a supplied collection of wire
occurrences. Each wire must borrow the identical graph object; a separately
constructed, structurally identical graph is rejected. Parallel edges remain
distinct IDs. Duplicate wire references are counted once per occurrence. The
report has one record per graph edge in edge-ID order, including unused edges,
and preserves exact forward and reverse counts.

| Forward, reverse counts | Classification in this collection |
| --- | --- |
| 0, 0 | unused |
| 1, 0 or 0, 1 | single use |
| 1, 1 | opposite pair |
| 2, 0 or 0, 2 | same-direction pair |
| total greater than 2 | more than two uses |

The input may omit boundaries or contain duplicates, so these classes are
histograms of the supplied occurrences, not conclusions about physical shell
boundaries or manifoldness. In particular, opposite pairs do not establish
connectedness, valid vertex neighborhoods, embedded surfaces, face interiors,
orientation of a solid, or any geometric property. The analysis has no face,
hole, shell, or solid type.

The graph-edge and wire-occurrence counts are capped first. Each graph identity
and the checked total use count are preflighted before allocating the edge-sized
report and tallying uses. Reservation is fallible; inputs are immutable and no
partial report is returned. The caps bound logical items rather than allocator
latency or all instructions.

## Scoped handles and declared boundary groups

`ScopedSnapshot` borrows an immutable incidence graph and receives a fresh,
private identity on every successful construction, including when multiple
snapshots wrap the same graph. The process-local counter never recycles an ID;
exhaustion and mutex poison return typed failures. Poison disables future
issuance in that loaded crate instance. Identities are ephemeral and are not
persistent credentials across processes, fork, restart, or separately loaded
crate instances. A snapshot cannot be cloned. The graph remains borrowed and
immutable while its snapshot or a scoped wire uses it.

A snapshot mints distinct typed vertex and edge handles for in-range indices.
Resolution checks both snapshot identity and range. A handle proves which
snapshot minted it; supplying a raw index from elsewhere to a minting function
cannot establish that index's history. Safe callers cannot construct handle
fields directly. Raw indices exposed by the older graph API and read-only
combinatorial diagnostics remain graph-local and do not authenticate re-entry.

A `ScopedWire` admits only scoped edge uses from its own snapshot, then applies
all existing simple closed-wire checks. Its construction preflights uses and
quadratic comparisons, checks every handle, and reserves temporary and owned
storage fallibly; peak construction can hold both use buffers. The owned wire
borrows its snapshot and graph, and failures return no partial value.

`DeclaredFaceBoundaries` holds one designated outer wire and zero or more inner
wires from the same snapshot. It rejects any shared graph vertex across its
boundaries, including reused edges or repeated/reversed wires. This is a
restricted declaration: outer/inner labels do not prove a disk-with-holes patch,
containment, orientation, planarity, geometric disjointness, a face, a shell or
a solid. It deliberately excludes some valid contact and singular cases. The
constructor preflights total boundaries, total uses and the checked sum of all
cross-boundary pair products; it fallibly reserves references and returns no
partial group on error. These logical caps do not bound allocator latency or
whole-process memory availability.

## Vertex-link diagnostics

`analyze_vertex_links` considers each graph-edge end used at a vertex as a
link node. At a wire corner it adds a link edge from the incoming use's
endpoint to the outgoing use's endpoint:

```text
       incoming graph edge       outgoing graph edge
              -----> vertex ----->
                         |
             link node --+-- link node
                        corner
```

Every wire occurrence contributes independently. Parallel graph edges remain
separate nodes, and duplicate corners produce parallel link edges; two nodes
joined by two link edges form a multigraph cycle. Unused graph-edge ends are
inactive. For each vertex the report gives active ends, corner occurrences,
connected components, and counts of nodes with degree one, degree two, and
other degrees. It classifies the active link as isolated, a single path, a
single cycle, or other (including branching or disconnected links).

These are facts about the supplied combinatorial occurrences. They do not prove
surface geometry, manifoldness, connectedness of the whole collection, shell
validity, or solid orientation. For example, duplicating a wire can create link
cycles without constructing a geometric surface. Future B-rep work needs an
explicit model for faces, holes, curves, surface trims and vertex neighborhoods.

The analysis requires every wire to borrow the same graph object. It caps graph
vertices, twice the graph-edge count, wire occurrences, total uses, and the
number of union-find parent pointers followed. Checked arithmetic and fallible
allocation precede publication; errors leave input untouched and return no
partial report. Runtime is bounded by logical vertices, edge ends, uses and the
charged parent traversals; caps do not promise allocator latency or total
process memory availability.
