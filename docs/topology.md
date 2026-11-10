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
