# Exact raw-input 3D orientation

`spatial::orient3d(a, b, c, d)` returns the exact sign of
`((b-a) × (c-a)) · (d-a)` for the finite binary64 coordinates stored in four
`FinitePoint3` values. The origin followed by the positive x, y and z unit
points produces `Positive`. Duplicate or exactly coplanar points produce
`Coplanar`. Signed zeros are the same exact input value. No subtraction or
multiplication is performed in floating point and no tolerance is applied.

Each binary64 input is embedded as an integer multiple of `2^-1074`. Its
magnitude is below `2^2098`, and an exact coordinate difference is below
`2^2099`. A cross-product component, formed as a difference of two products,
is below `2^4199`; multiplying by a coordinate difference is below `2^6298`.
The sum of three such terms is below `2^6300`. The implementation uses the
existing 33-limb coordinate and 66-limb product arithmetic and a 99-limb
(6336-bit) signed determinant. The signed 66-by-33 product must XOR operand
signs and canonicalize zero. Its bound is specific to raw finite inputs.

At most six 33-by-33 and three 66-by-33 limb products are required, or 13,068
scalar limb multiplications, plus fixed-width addition and carry work. The
implementation allocates no heap, but this count does not bound wall time, stack
usage or all instructions. The independent Python `Fraction` fixtures use a
different anchor and determinant expansion; they cover every finite exponent
field, near-coplanar and extreme cases, and mixed signs. The finite corpus and
this bound are evidence, not a formal proof or safety qualification.

This predicate describes the represented raw coordinates only. It does not
accept exact constructed rational or algebraic inputs, supply a plane or
intersection witness, validate B-rep topology, or solve repeated-cut closure.
An exact result on rounded constructed coordinates describes those rounded
coordinates, not necessarily the intended mathematical construction.
