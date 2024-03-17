use const_soft_float::soft_f32::SoftF32;

/// Householder transform function
///
/// Each element in the array is a sample. The mixer outputs some amount
/// of each input into all outputs. The interpolation is smooth between
/// the channels. The algorithm is often used in delays feedback loops.
pub fn transform_householder<const C: usize>(samples: &mut [f32; C]) {
    const fn multiplier<const C: usize>() -> f32 {
        SoftF32(-2.0).div(SoftF32(C as f32)).to_f32()
    }

    let k = samples.iter().sum::<f32>() * multiplier::<C>();

    for b in samples {
        *b += k;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_transform_householder() {
        use crate::f32cmp::F32Cmp;

        let mut input = [1.0, 2.0, 3.0, 4.0];
        transform_householder(&mut input);
        assert_eq!(input, [-4.0, -3.0, -2.0, -1.0].map(F32Cmp));

        let mut input = [1.0, -2.0, 3.0, -4.0];
        transform_householder(&mut input);
        assert_eq!(input, [2.0, -1.0, 4.0, -3.0].map(F32Cmp));
    }
}
