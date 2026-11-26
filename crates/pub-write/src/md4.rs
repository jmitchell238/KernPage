//! MD4 (RFC 1320). Publisher uses it as the unique id of a picture (spec 4C.1.1).

pub(crate) fn md4(data: &[u8]) -> [u8; 16] {
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64).wrapping_mul(8)).to_le_bytes());
    let mut s = [0x6745_2301u32, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476];
    for chunk in msg.as_chunks::<64>().0.iter() {
        let mut x = [0u32; 16];
        for (i, w) in x.iter_mut().enumerate() {
            let b = &chunk[4 * i..4 * i + 4];
            *w = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        }
        let [mut a, mut b, mut c, mut d] = s;
        const SH1: [u32; 4] = [3, 7, 11, 19];
        const SH2: [u32; 4] = [3, 5, 9, 13];
        const SH3: [u32; 4] = [3, 9, 11, 15];
        const K2: [usize; 16] = [0, 4, 8, 12, 1, 5, 9, 13, 2, 6, 10, 14, 3, 7, 11, 15];
        const K3: [usize; 16] = [0, 8, 4, 12, 2, 10, 6, 14, 1, 9, 5, 13, 3, 11, 7, 15];
        for (i, &w) in x.iter().enumerate() {
            let f = (b & c) | (!b & d);
            let t = a
                .wrapping_add(f)
                .wrapping_add(w)
                .rotate_left(SH1[i % 4]);
            (a, b, c, d) = (d, t, b, c);
        }
        for (i, &k) in K2.iter().enumerate() {
            let f = (b & c) | (b & d) | (c & d);
            let t = a
                .wrapping_add(f)
                .wrapping_add(x[k])
                .wrapping_add(0x5a82_7999)
                .rotate_left(SH2[i % 4]);
            (a, b, c, d) = (d, t, b, c);
        }
        for (i, &k) in K3.iter().enumerate() {
            let f = b ^ c ^ d;
            let t = a
                .wrapping_add(f)
                .wrapping_add(x[k])
                .wrapping_add(0x6ed9_eba1)
                .rotate_left(SH3[i % 4]);
            (a, b, c, d) = (d, t, b, c);
        }
        s[0] = s[0].wrapping_add(a);
        s[1] = s[1].wrapping_add(b);
        s[2] = s[2].wrapping_add(c);
        s[3] = s[3].wrapping_add(d);
    }
    let mut out = [0u8; 16];
    for (i, w) in s.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&w.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::md4;

    fn hex(d: &[u8]) -> String {
        d.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn rfc_1320_vectors() {
        for (input, want) in [
            ("", "31d6cfe0d16ae931b73c59d7e0c089c0"),
            ("a", "bde52cb31de33e46245e05fbdbd6fb24"),
            ("abc", "a448017aaf21d8525fc10ae87aa6729d"),
            ("message digest", "d9130a8164549fe818874806e1c7014b"),
            (
                "abcdefghijklmnopqrstuvwxyz",
                "d79e1c308aa5bbcdeea8ed63df412da9",
            ),
            (
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
                "043f8582f241db351ce627e153e7f0e4",
            ),
            (
                "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
                "e33b4ddc9c38f2199c3e7b164fcc0536",
            ),
        ] {
            assert_eq!(hex(&md4(input.as_bytes())), want, "{input:?}");
        }
    }
}
