//! Viridis colour map, as a 256-entry lookup table built at compile time.
//!
//! The table is filled from a degree-6 polynomial fit to matplotlib's
//! viridis (by Matt Zucker, shadertoy "viridis approximation"), which
//! matches the original to within about 1% per channel.

/// Polynomial coefficients, lowest power first; columns are R, G, B.
const COEFFS: [[f32; 3]; 7] = [
    [0.277_727_33, 0.005_407_344_5, 0.334_099_8],
    [0.105_093_04, 1.404_613_5, 1.384_590_2],
    [-0.330_861_83, 0.214_847_56, 0.095_095_16],
    [-4.634_230_5, -5.799_101, -19.332_441],
    [6.228_27, 14.179_933, 56.690_55],
    [4.776_385, -13.745_145, -65.353_03],
    [-5.435_456, 4.645_852_6, 26.312_435],
];

const fn build_lut() -> [[u8; 3]; 256] {
    let mut lut = [[0u8; 3]; 256];
    let mut i = 0;
    while i < 256 {
        let t = i as f32 / 255.0;
        let mut ch = 0;
        while ch < 3 {
            // Horner's rule, highest power first.
            let mut v = COEFFS[6][ch];
            let mut p = 6;
            while p > 0 {
                p -= 1;
                v = COEFFS[p][ch] + t * v;
            }
            let v = if v < 0.0 {
                0.0
            } else if v > 1.0 {
                1.0
            } else {
                v
            };
            lut[i][ch] = (v * 255.0 + 0.5) as u8;
            ch += 1;
        }
        i += 1;
    }
    lut
}

static VIRIDIS_LUT: [[u8; 3]; 256] = build_lut();

/// Viridis colour for `t` in `[0, 1]` (values outside are clamped; NaN maps to 0).
#[inline]
pub fn viridis(t: f32) -> [u8; 3] {
    let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
    VIRIDIS_LUT[(t * 255.0 + 0.5) as usize]
}

#[cfg(test)]
mod tests {
    use super::viridis;

    /// Reference points from matplotlib's viridis table.
    #[test]
    fn matches_matplotlib_anchor_points() {
        let anchors: [(f32, [f32; 3]); 5] = [
            (0.0, [0.267_004, 0.004_874, 0.329_415]),
            (0.25, [0.229_739, 0.322_361, 0.545_706]),
            (0.5, [0.127_568, 0.566_949, 0.550_556]),
            (0.75, [0.369_214, 0.788_888, 0.382_914]),
            (1.0, [0.993_248, 0.906_157, 0.143_936]),
        ];
        for (t, want) in anchors {
            let got = viridis(t);
            for ch in 0..3 {
                let diff = (f32::from(got[ch]) / 255.0 - want[ch]).abs();
                assert!(
                    diff < 0.025,
                    "viridis({t}) channel {ch}: got {got:?}, want {want:?}"
                );
            }
        }
    }

    #[test]
    fn clamps_out_of_range() {
        assert_eq!(viridis(-1.0), viridis(0.0));
        assert_eq!(viridis(2.0), viridis(1.0));
        assert_eq!(viridis(f32::NAN), viridis(0.0));
    }
}
