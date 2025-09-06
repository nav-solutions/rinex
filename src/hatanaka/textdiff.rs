//! Y. Hatanaka lossless TextDiff algorithm

/// [TextDiff] is a structure that implements the Text diff. algorithm
/// designed by Y. Hatanaka, which is a lossless text compression algorithm.
/// [TextDiff] in its current form does not allow compressing & decompressing (back & forth)
/// at the same time: you need two dedicated objects.
/// This does not bother our application because it only operates in one way,
/// but it is one aspect to keep in mind.
#[derive(Debug)]
pub struct TextDiff {
    buffer: String,
    compressed: String,
}

impl Default for TextDiff {
    fn default() -> Self {
        Self::new("")
    }
}

impl TextDiff {
    /// Creates a new [TextDiff] structure to apply the
    /// differential algorithm developped by Y. Hatanaka.
    pub fn new(data: &str) -> Self {
        Self {
            buffer: data.to_string(),
            compressed: data.to_string(),
        }
    }

    /// Force kernel reset using new content
    pub fn force_init(&mut self, data: &str) {
        self.buffer = data.to_string();
        self.compressed = data.to_string();
    }

    /// Decompresses given data. Returns recovered value.
    pub fn decompress(&mut self, data: &str) -> &str {
        let size = data.len();
        let buf_len = self.buffer.len();

        if size > buf_len {
            // extend new bytes as is
            self.buffer.push_str(&data[buf_len..]);
        }

        // browse all bytes and save only new content
        let bytes = data.bytes();
        unsafe {
            let buf = self.buffer.as_bytes_mut();

            // browse all new bytes
            for (i, byte) in bytes.enumerate() {
                if let Some(buf) = buf.get_mut(i) {
                    if byte != b' ' {
                        if byte == b'&' {
                            *buf = b' ';
                        } else {
                            *buf = byte;
                        }
                    }
                }
            }
            &self.buffer
        }
    }

    /// Compress given data using the Textdiff algorithm.
    /// Returns compressed text.
    pub fn compress(&mut self, data: &str) -> &str {
        let len = data.len();
        let mut buf_len = self.buffer.len();

        if buf_len > len {
            // special case: shrink history
            self.buffer = self.buffer[..len].to_string();
            buf_len = len; // update size
        }

        self.compressed = self.buffer.to_string();

        unsafe {
            let mut bytes = data.as_bytes().iter();
            let mut buffered = self.buffer.as_bytes().iter();
            let mut compressed = self.compressed.as_bytes_mut().iter_mut();

            while let Some(buffered) = buffered.next() {
                if let Some(byte) = bytes.next() {
                    if let Some(compressed) = compressed.next() {
                        if byte == buffered {
                            *compressed = b' ';
                        } else {
                            if *byte == b' ' {
                                *compressed = b'&';
                            } else {
                                *compressed = *byte;
                            }
                        }
                    }
                }
            }
        }

        // update internal memory
        self.buffer = data.to_string();

        // possible new string termination
        if len > buf_len {
            self.compressed.push_str(&data[buf_len..]);

            unsafe {
                let mut compressed = self.compressed.as_bytes_mut().iter_mut().skip(buf_len);
                while let Some(byte) = compressed.next() {
                    if *byte == b' ' {
                        *byte = b'&';
                    }
                }
            }
        }

        &self.compressed
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn textdiff_decompression() {
        const INIT: &str = "ABCDEFG 12 000 33 XXACQmpLf";
        let mut diff = TextDiff::new(INIT);

        let compressed: Vec<&str> = vec![
            "         3   1 44 xxACq   F",
            "        4 ",
            " 11 22   x   0 4  y     p  ",
            "              1     ",
            "                   z",
            " ",
            "                           &",
            "&                           ",
            " ",
        ];

        let expected: Vec<&str> = vec![
            "ABCDEFG 13 001 44 xxACqmpLF",
            "ABCDEFG 43 001 44 xxACqmpLF",
            "A11D22G 4x 000 44 yxACqmpLF",
            "A11D22G 4x 000144 yxACqmpLF",
            "A11D22G 4x 000144 yzACqmpLF",
            "A11D22G 4x 000144 yzACqmpLF",
            "A11D22G 4x 000144 yzACqmpLF ",
            " 11D22G 4x 000144 yzACqmpLF ",
            " 11D22G 4x 000144 yzACqmpLF ",
        ];

        for i in 0..compressed.len() {
            let decompressed = diff.decompress(compressed[i]);

            assert_eq!(
                decompressed,
                expected[i],
                "failed for {}th \"{}\"",
                i + 1,
                compressed[i]
            );
        }

        // test re-init
        diff.force_init(" 2200 123      G 07G08G09G   XX XX");

        let masks: Vec<&str> = vec![
            "        F       1  3",
            " x    1 f  f   p",
            " ",
            "  3       4       ",
        ];
        let expected: Vec<&str> = vec![
            " 2200 12F      G107308G09G   XX XX",
            " x200 12f  f   p107308G09G   XX XX",
            " x200 12f  f   p107308G09G   XX XX",
            " x300 12f 4f   p107308G09G   XX XX",
        ];
        for i in 0..masks.len() {
            let mask = masks[i];
            let result = diff.decompress(mask);
            assert_eq!(
                result,
                expected[i].to_string(),
                "failed for [{}]{}",
                i,
                mask
            );
        }
    }

    #[test]
    fn textdiff_compression() {
        let mut diff = TextDiff::new("0");

        assert_eq!(diff.compress("0"), " ");
        assert_eq!(diff.compress("4"), "4");
        assert_eq!(diff.compress("4"), " ");
        assert_eq!(diff.compress("44"), " 4");
        assert_eq!(diff.compress("4 "), " &");
        assert_eq!(diff.compress("4 "), "  ");
        assert_eq!(diff.compress("4  "), "  &");
        assert_eq!(diff.compress("0"), "0");
        assert_eq!(diff.compress("0"), " ");
        assert_eq!(diff.compress("   "), "&&&");

        diff.force_init("Default 1234");
        assert_eq!(diff.compress("DEfault 1234"), " E          ");
        assert_eq!(diff.compress("DEfault 1234"), "            ");
        assert_eq!(diff.compress("             "), "&&&&&&& &&&&&");
    }
}
