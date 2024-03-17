const TOLERANCE_NEG: f32 = -0.0001;
const TOLERANCE_POS: f32 = 0.0001;

#[derive(Debug, Clone, Copy)]
pub struct F32Cmp(pub f32);

fn eq(a: F32Cmp, b: F32Cmp) -> bool {
    let n = a.0 - b.0;
    n > TOLERANCE_NEG && n < TOLERANCE_POS
}

impl PartialEq for F32Cmp {
    fn eq(&self, other: &Self) -> bool {
        eq(*self, *other)
    }
}

impl PartialEq<f32> for F32Cmp {
    fn eq(&self, other: &f32) -> bool {
        eq(*self, F32Cmp(*other))
    }
}

impl PartialEq<F32Cmp> for f32 {
    fn eq(&self, other: &F32Cmp) -> bool {
        eq(F32Cmp(*self), *other)
    }
}

impl Eq for F32Cmp {}
