# Raw-input segment and three-point plane

`RawPlane3` retains three finite binary64 points in their original order. Its
constructor accepts them only when the exact cross product of `(b-a)` and
`(c-a)` is nonzero. This establishes a unique oriented plane of the represented
raw dyadic points; there is no tolerance or floating normalization. A degenerate
xy projection can still define a plane through a nonzero xz or yz projection.

`RawPlane3::side` returns the exact sign of `((b-a) × (c-a)) · (p-a)` for a
finite raw point, using the separately documented fixed-width `orient3d` path.
Reversing two plane points reverses the sign. This is not a predicate on an
implicit, rational, algebraic or rounded intended construction.

For a checked nonzero finite `Segment3`, `intersect_segment` classifies the
closed sets using the exact two endpoint signs:

| Start side | End side | Event |
| --- | --- | --- |
| zero | zero | entire segment contained |
| zero | nonzero | start endpoint |
| nonzero | zero | end endpoint |
| equal nonzero signs | equal nonzero signs | disjoint |
| opposite nonzero signs | opposite nonzero signs | unique proper crossing |

Endpoint events return the original endpoint bit patterns, including signed
zero. A proper crossing retains the original plane and directed segment in an
opaque witness. Because the exact signed plane expression is affine along the
segment, opposite signs establish one interior root `t = s0 / (s0 - s1)`.
The API does not compute `t`, construct coordinates, compare witnesses, or
accept the witness as input to a later exact operation. Its mathematical
identity is not an exported equality/hash contract. An approximation or box
would not substitute for that retained definition.

Plane admission uses six 33-by-33 limb multiplications at most. Each exact side
uses six 33-by-33 and three 66-by-33 multiplications at most; classification
uses two side evaluations. These fixed counts bound scalar limb multiplication
only, not all instructions, stack bytes or wall time. The implementations use
no heap. See [the determinant bounds](orientation3.md): the 66-limb and
99-limb proofs apply to raw finite binary64 inputs only.

The independently generated Python `Fraction` fixtures classify the same
finite inputs and check the exact crossing parameter and incidence for strict
crossings. Their finite corpus is evidence, not a universal proof. This slice
does not construct intersections of constructed segments, provide an owned
B-rep, validate solids, or establish a Boolean operation.
