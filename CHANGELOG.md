# Changelog

## 0.2.0-alpha.1 (unreleased)

- Add process-local snapshot-scoped handles, scoped wires and bounded declared
  boundary groups without claiming geometric or face validity.
- Analyze bounded vertex-link multigraphs of wire occurrences without claiming
  geometric or manifold validity.

- Count directed edge-use incidence across bounded wire collections, preserving
  graph identity and explicitly leaving shell validity unestablished.

- Classify finite raw segments against three-point raw planes, retaining the
  original supports as an opaque proper-crossing witness.

- Add exact 3D orientation for four finite binary64 input points using bounded
  integer arithmetic; general constructed-point predicates remain future work.

- Add bounded graph incidence, vertex-simple combinatorial closed wires and
  directed edge-use counts; shell and solid validity remain future work.
- Add finite-control quadratic 3D Bézier curves with explicitly rounded,
  fallible parameter evaluation and comparison-only control bounds.

- Group faces with bridge excursions when each filtered boundary is empty or
  one simple cycle, retaining original half-edge ownership.
- Add checked finite 3D points, segments, closed bounds, and fallible rounded
  line and plane evaluations; exact 3D predicates and topology remain future work.

- Classify finite raw points against selected 2D regions with exact boundary
  incidence, half-open ray crossing and a preflight edge-check budget.
- Analyze connected components and merged graph-edge bridges with an iterative,
  resource-bounded search.
- Compute per-group face winding and bounded face-based 2D Boolean boundary
  selections with nonzero and even-odd fill rules.
- Group simple boundary walks into bounded and unbounded faces and compute
  aggregate directed-source winding with explicit limits and failure modes.
- Build bounded half-edge rotations and boundary walks, and classify the exact
  orientation of simple walks without assigning faces or accumulating area.
- Build resource-bounded raw straight-segment arrangement graphs with exact
  shared vertices, merged coincident edges, source uses and atomic failure.
- Locate retained exact points on other raw supports, then split and intersect
  provenance-preserving spans without materializing wider parameter fractions.
- Intersect pairs of original-support spans and compare retained parameter points
  across supports exactly, including source-tagged overlap boundaries.
- Intersect retained original-support spans with raw segments using exact
  parameter clipping and a caller-supplied query budget.
- Add budgeted exact orientation for raw and first-generation crossing points,
  and bounded repeated parameter-span splitting on one original source segment.
- Compare exact raw-input proper-crossing points with each other and with
  finite input points without rounding or fraction reduction.
- Add tight finite coordinate enclosures for proper crossings without rounding
  the retained parameter or using process-global rounding modes.
- Retain exact parameters for finite input points lying on nonzero segments.
- Classify closed finite segment sets with zero-length point segments, retaining
  an original point for contact without inventing a directed parameter.
- Add tight binary64 enclosures for exact retained segment parameters, using
  integer comparison without rounded division.
- Add exact closed-segment point containment for finite binary64 inputs.
- Retain the two original boundaries of a positive-length collinear overlap
  and their exact parameters in first-segment traversal order.
- Retain exact endpoint-touch witnesses and compare their parameters with
  proper crossings on the same directed raw-input segment.
- Retain exact rational proper-crossing parameters and compare their order on
  the same directed raw-input segment; other intersection classes remain gated.
- Add checked finite 2D axis-aligned bounds, including closed intersections,
  unions, point containment, and construction from finite points.
- Compare all 5,184 pairs of directed segments on a small integer grid against
  an independent integer parametric intersection oracle.
- Begin the 0.2.0 line with checked finite planar points and segments, exact
  orientation predicates, segment-intersection event classification, and
  simple closed-wire validation. General B-rep and Boolean operations remain
  future milestones.
- Raise the unreleased package minimum to Rust 1.99.1 and retain the pinned
  development toolchain; published 0.1.1 remains supported on Rust 1.85.
- Add independent small-integer orientation-grid checks and Python
  `Fraction`-oracle binary64 cases, plus regressions for product overflow,
  underflow, and coordinate-subtraction overflow.

## 0.1.1 (published 2026-11-09)

- Specify operation-level numerical contracts, caller-defined units, nonfinite
  behavior, overflow/underflow, cancellation, and normalization limitations.
- Add reproducible independent high-precision reference fixtures and explicit
  acceptance budgets, plus adversarial exceptional-value regression tests.
- Expand debug/release verification across a configured Linux/macOS/Windows CI
  matrix, including ARM64, and add packaged-crate/downstream API smoke checks.
- Preserve the v0.1.0 public API and Rust 1.85 minimum. Development commands use
  the already-adopted Rust 1.99.1 pin without making it the package minimum.
- Document verification evidence and limitations without claiming safety
  qualification or robust geometric predicates.
- Add a source-comment/rustdoc pass explaining affine subtraction, frame/unit
  responsibilities, direction copies, and normalization maintenance hazards;
  update the README, contributor guidance, and verification record together.
  The additional example is doctested; runtime behavior is unchanged.

## 0.1.0

- Start an independent Apache-2.0 pure-Rust CAD-kernel project with original elementary
  3D point, vector, and checked unit-direction types.
- Document floating-point behavior and explicit early-stage scope.
- Add tests, doctests, packaging metadata, and minimum-version/stable CI.

No OCCT compatibility or CAD-kernel functionality is claimed by this release.
