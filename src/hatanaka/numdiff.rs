//! Y. Hatanaka lossless numerical compression algorithm

/// This module implements the Hatanaka scheme for numerical (de)-compression.
/// Values are multiplied by a factor of 10^N (N is the number of decimals in the
/// original data) to turn them into integers.
/// These integers are then put through a higher-order delta-compression scheme.
///
/// There are two tunable parameters:
/// - M: the maximum order accepted by the decompressor. The historical CRX2RNX
///   program supports M=5, but we support M=6.
/// - level (m): the actual order used for compression. It must be <= M. The
///   historical RNX2CRX program uses m=3, which is a good choice for 1-30s
///   sampling rates.
///
/// A higher m does not necessarily mean better compression, since at a high-enough
/// level random noise dominates over any differential trend.
///
/// If you want to produce compatible data, you should respect M = 5.
/// If your remain within our application, you can use higher compression order
/// (m <= M = 6).
///
/// Currently compressor.rs uses level = 3 ([CompressorExpert::format()]).
#[derive(Debug, Clone)]
pub struct NumDiff<const M: usize> {
    /// iteration counter
    m: usize,
    /// compression level, within M maximal range
    level: usize,
    /// internal data history
    buf: [i64; M],
}

impl<const M: usize> NumDiff<M> {
    /// Builds a [NumDiff] structure dedicated to numerical (de-)compression.
    /// Level must not exceed 6 otherwise this will panic.
    /// ## Inputs
    ///  - data: initial point
    ///  - level: compression level / range.
    pub fn new(data: i64, level: usize) -> Self {
        if level > 6 {
            panic!("M=6 is the compression limit");
        }
        let mut buf = [0; M]; // reset
        buf[0] = data;
        Self { buf, m: 0, level }
    }
    /// [NumDiff] needs to be reinit   when ???
    pub fn force_init(&mut self, data: i64, level: usize) {
        if level > 6 {
            panic!("M=6 is the compression limit");
        }
        self.m = 0;
        self.level = level;
        self.rotate_history(data);
    }

    /// Rotate internal buffer, take new sample into account.
    fn rotate_history(&mut self, data: i64) {
        self.buf.copy_within(0..M - 2, 1);
        self.buf[0] = data;
    }

    /// Decompresses input data point, returns recovered data point.
    pub fn decompress(&mut self, data: i64) -> i64 {
        if self.m < self.level {
            self.m += 1;
        }

        let new: i64 = match self.m {
            1 => data + self.buf[0],
            2 => data + 2 * self.buf[0] - self.buf[1],
            3 => data + 3 * self.buf[0] - 3 * self.buf[1] + self.buf[2],
            4 => data + 4 * self.buf[0] - 6 * self.buf[1] + 4 * self.buf[2] - self.buf[3],
            5 => {
                data + 5 * self.buf[0] - 10 * self.buf[1] + 10 * self.buf[2] - 5 * self.buf[3]
                    + self.buf[4]
            },
            6 => {
                data + 6 * self.buf[0] - 15 * self.buf[1] + 20 * self.buf[2] - 15 * self.buf[3]
                    + 6 * self.buf[4]
                    - self.buf[5]
            },
            _ => panic!("numdiff is limited to M < 7"),
        };

        self.rotate_history(new);
        new
    }

    /// Compresses input data point, returns "compressed" data point.
    pub fn compress(&mut self, data: i64) -> i64 {
        if self.m < self.level {
            self.m += 1;
        }
        self.rotate_history(data);

        match self.m {
            1 => self.buf[0] - self.buf[1],
            2 => self.buf[0] - 2 * self.buf[1] + self.buf[2],
            3 => self.buf[0] - 3 * self.buf[1] + 3 * self.buf[2] - self.buf[3],
            4 => self.buf[0] - 4 * self.buf[1] + 6 * self.buf[2] - 4 * self.buf[3] + self.buf[4],
            5 => {
                self.buf[0] - 5 * self.buf[1] + 10 * self.buf[2] - 10 * self.buf[3]
                    + 5 * self.buf[4]
                    - self.buf[5]
            },
            6 => {
                self.buf[0] - 6 * self.buf[1] + 15 * self.buf[2] - 20 * self.buf[3]
                    + 15 * self.buf[4]
                    - 6 * self.buf[5]
                    + self.buf[6]
            },
            _ => panic!("numdiff is limited to M < 7"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_decompression() {
        let mut diff = NumDiff::<6>::new(126298057858, 3);
        assert_eq!(diff.decompress(-15603288), 126282454570);
        assert_eq!(diff.decompress(521089), 126267372371);
        assert_eq!(diff.decompress(-752), 126252810509);
        assert_eq!(diff.decompress(1575419284), 127814188268);
        assert_eq!(diff.decompress(-3150848707), 127800656941);
        assert_eq!(diff.decompress(1575424909), 127787641437);
        assert_eq!(diff.decompress(-135), 127775141621);

        // test re-init
        diff.force_init(111982965979, 3);
        assert_eq!(diff.decompress(-16266911), 111966699068);
        assert_eq!(diff.decompress(609858), 111951042015);
        assert_eq!(diff.decompress(-213), 111935994607);
        assert_eq!(diff.decompress(1575419307), 113496976151);
        assert_eq!(diff.decompress(-3150848442), 113483138205);
        assert_eq!(diff.decompress(1575425367), 113469906136);
        assert_eq!(diff.decompress(146), 113457280090);
    }

    #[test]
    fn test_compression() {
        let mut diff = NumDiff::<6>::new(126298057858, 3);
        assert_eq!(diff.compress(126282454570), -15603288);
        assert_eq!(diff.compress(126267372371), 521089);
        assert_eq!(diff.compress(126252810509), -752);
        assert_eq!(diff.compress(127814188268), 1575419284);
        assert_eq!(diff.compress(127800656941), -3150848707);
        assert_eq!(diff.compress(127787641437), 1575424909);
        assert_eq!(diff.compress(127775141621), -135);

        diff.force_init(111982965979, 3);
        assert_eq!(diff.compress(111966699068), -16266911);
        assert_eq!(diff.compress(111951042015), 609858);
        assert_eq!(diff.compress(111935994607), -213);
        assert_eq!(diff.compress(113496976151), 1575419307);
        assert_eq!(diff.compress(113483138205), -3150848442);
        assert_eq!(diff.compress(113469906136), 1575425367);
        assert_eq!(diff.compress(113457280090), 146);
    }
}
