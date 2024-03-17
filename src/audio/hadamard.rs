use const_soft_float::soft_f32::SoftF32;

#[allow(unused_imports)]
use micromath::F32Ext;

/// An orthogonal hadamard matrix transform
///
/// Creates a maximum amount of channel mixing where the energy is preserved. The
/// signal is evenly spread across the channels without introducing phase issues
/// or coloration. The mixer is often used in diffusion steps.
///
/// ```text
///              ┌──────────────────┐
/// a ─────────▶ │                  │ ─────────▶ w
///              │                  │
/// b ─────────▶ │     hadamard     │ ─────────▶ x
///              │      matrix      │
/// c ─────────▶ │                  │ ─────────▶ y
///              │                  │
/// d ─────────▶ │                  │ ─────────▶ z
///              └──────────────────┘
///
/// Orthogonal matrix means preserving the energy:
///
/// a^2 + b^2 + d^2 + c^2   =   w^2 + x^2 + y^2 + z^2
/// ```
///
/// The input must be a power of 2.
pub fn transform_hadamard<const C: usize>(samples: &mut [f32; C]) {
    let mut stride = 1;

    while stride < C {
        for i in (0..C).step_by(stride * 2) {
            for j in 0..stride {
                let a = samples[i + j];
                let b = samples[i + j + stride];
                // Instead of scaling down as a final step, we can multiply by 1/sqrt(2) here.
                // buf[i + j] = (a + b) * FRAC_1_SQRT_2;
                // buf[i + j + stride] = (a - b) * FRAC_1_SQRT_2;
                samples[i + j] = a + b;
                samples[i + j + stride] = a - b;
            }
        }
        stride *= 2;
    }

    const fn factor<const C: usize>() -> f32 {
        // Soft32 here is while we wait for this to land:
        // https://github.com/rust-lang/rust/issues/57241
        (SoftF32(1.0).div(SoftF32(C as f32))).sqrt().to_f32()
    }

    // Scale down to orthogonal
    for b in samples {
        *b *= factor::<C>();
    }
}

#[cfg(test)]
mod test {
    use crate::f32cmp::F32Cmp;

    use super::*;

    #[test]
    fn test_transform_hadamard() {
        let mut input = [1., 1., 1., 1.];
        transform_hadamard(&mut input);
        assert_eq!(input, [2., 0., 0., 0.].map(F32Cmp));

        let mut input = [1., 2., 3., 4.];
        transform_hadamard(&mut input);
        assert_eq!(input, [5.0, -1.0, -2.0, 0.0].map(F32Cmp));
    }
}
