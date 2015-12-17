use byteorder::{ByteOrder, LittleEndian};
use typenum::consts::{U16, U64, U128};

use digest::Digest;
use utils::buffer::{FixedBuffer64, FixedBuf, StandardPadding};

#[cfg(feature = "asm-md5")]
mod compress {
    extern {
        fn OCTAVO_md5_compress(state: *mut u32, data: *const u8);
    }

    pub fn compress(state: &mut [u32], data: &[u8]) {
        unsafe { OCTAVO_md5_compress(state.as_mut_ptr(), data.as_ptr()) }
    }
}

#[cfg(not(feature = "asm-md5"))]
mod compress {
    use byteorder::{ByteOrder, LittleEndian};

    // Round 1 constants
    const C1: [u32; 16] = [0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a,
                           0xa8304613, 0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
                           0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821];

    // Round 2 constants
    const C2: [u32; 16] = [0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453,
                           0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
                           0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a];

    // Round 3 constants
    const C3: [u32; 16] = [0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9,
                           0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
                           0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665];

    // Round 4 constants
    const C4: [u32; 16] = [0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92,
                           0xffeff47d, 0x85845dd1, 0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
                           0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391];

    pub fn compress(state: &mut [u32], input: &[u8]) {
        fn f(u: u32, v: u32, w: u32) -> u32 {
            (u & v) | (!u & w)
        }

        fn g(u: u32, v: u32, w: u32) -> u32 {
            (u & w) | (v & !w)
        }

        fn h(u: u32, v: u32, w: u32) -> u32 {
            u ^ v ^ w
        }

        fn i(u: u32, v: u32, w: u32) -> u32 {
            v ^ (u | !w)
        }

        fn op_f(w: u32, x: u32, y: u32, z: u32, m: u32, s: u32) -> u32 {
            w.wrapping_add(f(x, y, z)).wrapping_add(m).rotate_left(s).wrapping_add(x)
        }

        fn op_g(w: u32, x: u32, y: u32, z: u32, m: u32, s: u32) -> u32 {
            w.wrapping_add(g(x, y, z)).wrapping_add(m).rotate_left(s).wrapping_add(x)
        }

        fn op_h(w: u32, x: u32, y: u32, z: u32, m: u32, s: u32) -> u32 {
            w.wrapping_add(h(x, y, z)).wrapping_add(m).rotate_left(s).wrapping_add(x)
        }

        fn op_i(w: u32, x: u32, y: u32, z: u32, m: u32, s: u32) -> u32 {
            w.wrapping_add(i(x, y, z)).wrapping_add(m).rotate_left(s).wrapping_add(x)
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];

        let mut data = [0u32; 16];

        for (v, c) in data.iter_mut().zip(input.chunks(4)) {
            *v = LittleEndian::read_u32(c);
        }

        // round 1
        for i in 0..4 {
            let i = i * 4;
            a = op_f(a, b, c, d, data[i].wrapping_add(C1[i]), 7);
            d = op_f(d, a, b, c, data[i + 1].wrapping_add(C1[i + 1]), 12);
            c = op_f(c, d, a, b, data[i + 2].wrapping_add(C1[i + 2]), 17);
            b = op_f(b, c, d, a, data[i + 3].wrapping_add(C1[i + 3]), 22);
        }

        // round 2
        let mut t = 1;
        for i in 0..4 {
            let i = i * 4;
            a = op_g(a, b, c, d, data[t & 0x0f].wrapping_add(C2[i]), 5);
            d = op_g(d, a, b, c, data[(t + 5) & 0x0f].wrapping_add(C2[i + 1]), 9);
            c = op_g(c,
                     d,
                     a,
                     b,
                     data[(t + 10) & 0x0f].wrapping_add(C2[i + 2]),
                     14);
            b = op_g(b,
                     c,
                     d,
                     a,
                     data[(t + 15) & 0x0f].wrapping_add(C2[i + 3]),
                     20);
            t += 20;
        }

        // round 3
        t = 5;
        for i in 0..4 {
            let i = i * 4;
            a = op_h(a, b, c, d, data[t & 0x0f].wrapping_add(C3[i]), 4);
            d = op_h(d, a, b, c, data[(t + 3) & 0x0f].wrapping_add(C3[i + 1]), 11);
            c = op_h(c, d, a, b, data[(t + 6) & 0x0f].wrapping_add(C3[i + 2]), 16);
            b = op_h(b, c, d, a, data[(t + 9) & 0x0f].wrapping_add(C3[i + 3]), 23);
            t += 12;
        }

        // round 4
        t = 0;
        for i in 0..4 {
            let i = i * 4;
            a = op_i(a, b, c, d, data[t & 0x0f].wrapping_add(C4[i]), 6);
            d = op_i(d, a, b, c, data[(t + 7) & 0x0f].wrapping_add(C4[i + 1]), 10);
            c = op_i(c,
                     d,
                     a,
                     b,
                     data[(t + 14) & 0x0f].wrapping_add(C4[i + 2]),
                     15);
            b = op_i(b,
                     c,
                     d,
                     a,
                     data[(t + 21) & 0x0f].wrapping_add(C4[i + 3]),
                     21);
            t += 28;
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }
}

#[derive(Copy, Clone, Debug)]
struct State {
    state: [u32; 4],
}

impl State {
    fn new() -> Self {
        State { state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476] }
    }

    fn process_block(&mut self, input: &[u8]) {
        assert_eq!(input.len(), 64);

        compress::compress(&mut self.state, input);
    }
}

#[derive(Clone)]
pub struct Md5 {
    state: State,
    length: u64,
    buffer: FixedBuffer64,
}

impl Default for Md5 {
    fn default() -> Self {
        Md5 {
            state: State::new(),
            length: 0,
            buffer: FixedBuffer64::new(),
        }
    }
}

impl Digest for Md5 {
    type OutputBits = U128;
    type OutputBytes = U16;

    type BlockSize = U64;

    fn update<T>(&mut self, input: T)
        where T: AsRef<[u8]>
    {
        let input = input.as_ref();
        self.length += input.len() as u64;

        let state = &mut self.state;
        self.buffer.input(&input[..], |d| state.process_block(d));
    }

    fn result<T: AsMut<[u8]>>(mut self, mut out: T) {
        let state = &mut self.state;

        self.buffer.standard_padding(8, |d| state.process_block(d));
        LittleEndian::write_u64(self.buffer.next(8), self.length << 3);
        state.process_block(self.buffer.full_buffer());

        let mut out = out.as_mut();
        assert!(out.len() >= Self::output_bytes());
        for (b, s) in out.chunks_mut(4).zip(state.state.iter()) {
            LittleEndian::write_u32(b, *s);
        }
    }
}
