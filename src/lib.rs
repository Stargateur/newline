#![cfg_attr(feature = "benchmarks", feature(test))]

use jetscii;
use std::io::{BufRead, ErrorKind};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    FromUtf8Error(std::string::FromUtf8Error),
}

pub trait NewLine: BufRead + Sized {
    /*    fn read_line_cr(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        read_line_cr(self, buf)
    }

        fn read_line_crlf(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        read_line_u8(self, buf)
    }

    fn read_line_lfcr(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        read_line_u8(self, buf)
    }

    fn lines_cr(self) -> LinesIterCr<Self> {
        LinesIterCr { inner: self }
    }

        fn lines_crlf(self) -> LinesIter<Self> {
        LinesIter { inner: self }
    }

    fn lines_lfcr(self) -> LinesIter<Self> {
        LinesIter { inner: self }
    }*/

    fn lines_all(self) -> LinesAllIter<Self> {
        LinesAllIter { inner: self }
    }

    fn read_line_all(&mut self, buf: &mut Vec<u8>) -> Result<usize, std::io::Error> {
        read_line_all(self, buf)
    }
}

impl<R: BufRead> NewLine for R {}

pub struct LinesAllIter<R> {
    inner: R,
}

impl<R> Iterator for LinesAllIter<R>
where
    R: BufRead,
{
    type Item = Result<String, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = Vec::new();
        match self.inner.read_line_all(&mut line) {
            Ok(0) => None,
            Ok(_) => Some(String::from_utf8(line).map_err(|e| Error::FromUtf8Error(e))),
            Err(e) => Some(Err(Error::Io(e))),
        }
    }
}

fn read_line_all<R: BufRead + ?Sized>(
    r: &mut R,
    buf: &mut Vec<u8>,
) -> Result<usize, std::io::Error> {
    let mut read = 0;
    let mut prev: Option<u8> = None;
    loop {
        let (done, used) = {
            let available = match r.fill_buf() {
                Ok(n) => n,
                Err(e) => {
                    if e.kind() == ErrorKind::Interrupted {
                        continue;
                    } else {
                        break Err(e);
                    }
                }
            };
            if let Some(prev) = prev {
                // prev delim was found so we are going to check if it was \r, \n, \r\n or \n\r
                // we will not copy anything in the buffer
                if let Some(current) = available.get(0) {
                    let size_delim = match current {
                        b'\n' | b'\r' => {
                            if prev == *current {
                                // \r\r and \n\n count as 2 delim
                                0 // we don't consume
                            } else {
                                // etheir \r\n or \n\r
                                1 // we consume
                            }
                        }
                        _ => 0, // was either \r or \n
                    };
                    (true, size_delim) // \r, \n, \r\n or \n\r was found
                } else {
                    (true, 0) // eof and \r or \n was the final delim
                }
            } else {
                // if previous iteration was "normal"
                if let Some(delim) = jetscii::bytes!('\n', '\r').find(available)
                // check n first speed ?
                {
                    let (done, size_delim) = match (available.get(delim), available.get(delim + 1))
                    {
                        (Some(b'\r'), Some(b'\n')) | (Some(b'\n'), Some(b'\r')) => (true, 2), // \r\n or \n\r
                        (Some(b'\n'), Some(_)) | (Some(b'\r'), Some(_)) => (true, 1), // \r or \n
                        (current @ Some(b'\n'), None) | (current @ Some(b'\r'), None) => {
                            prev = current.cloned(); // this is cheap TODO: copied
                            (false, 1) // need the next token
                        }
                        _ => unreachable!(), // if we are here `jetscii::bytes!('\n', '\r').find(available)` is buged
                    };
                    buf.extend_from_slice(&available[..delim]); // we don't copy delim
                    (done, delim + size_delim) // we find delim we don't copy it but we consume it
                } else {
                    buf.extend_from_slice(&available); // we copy the full buffer
                    (false, available.len()) // we didn't find delim or eof
                }
            }
        };

        r.consume(used);
        read += used;
        if done || used == 0 {
            // eof is used == 0, done == true is we found delim
            break Ok(read);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use std::io::BufReader;

    #[test]
    fn lines_all_iter() {
        let input = BufReader::with_capacity(
            3,
            "Heading:\r\nLine 1\rLine 2\rLine 3\r\nEnd\n\r".as_bytes(),
        );
        let output = ["Heading:", "Line 1", "Line 2", "Line 3", "End"];
        for (a, b) in input.lines_all().flatten().zip_eq(output.iter()) {
            assert_eq!(&a, b);
        }
    }
}

#[cfg(all(test, feature = "benchmarks"))]
mod bench {
    extern crate test;
    use self::test::Bencher;

    use super::*;

    #[bench]
    fn lines_all_iter(b: &mut Bencher) {
        b.iter(|| {})
    }
}
