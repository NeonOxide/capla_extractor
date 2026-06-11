// SHA-256 over `input[0..len]`, writing 32 bytes of digest to `output`.
fun sha256(len: u64, input: [u8; len], output: mut [u8; 32]) {
    // --- Round constants K[0..64] ---------------------------------------
    let k: [u32; 64] = alloc u32, 64;
    k[0]  = 1116352408u32; k[1]  = 1899447441u32; k[2]  = 3049323471u32; k[3]  = 3921009573u32;
    k[4]  = 961987163u32;  k[5]  = 1508970993u32; k[6]  = 2453635748u32; k[7]  = 2870763221u32;
    k[8]  = 3624381080u32; k[9]  = 310598401u32;  k[10] = 607225278u32;  k[11] = 1426881987u32;
    k[12] = 1925078388u32; k[13] = 2162078206u32; k[14] = 2614888103u32; k[15] = 3248222580u32;
    k[16] = 3835390401u32; k[17] = 4022224774u32; k[18] = 264347078u32;  k[19] = 604807628u32;
    k[20] = 770255983u32;  k[21] = 1249150122u32; k[22] = 1555081692u32; k[23] = 1996064986u32;
    k[24] = 2554220882u32; k[25] = 2821834349u32; k[26] = 2952996808u32; k[27] = 3210313671u32;
    k[28] = 3336571891u32; k[29] = 3584528711u32; k[30] = 113926993u32;  k[31] = 338241895u32;
    k[32] = 666307205u32;  k[33] = 773529912u32;  k[34] = 1294757372u32; k[35] = 1396182291u32;
    k[36] = 1695183700u32; k[37] = 1986661051u32; k[38] = 2177026350u32; k[39] = 2456956037u32;
    k[40] = 2730485921u32; k[41] = 2820302411u32; k[42] = 3259730800u32; k[43] = 3345764771u32;
    k[44] = 3516065817u32; k[45] = 3600352804u32; k[46] = 4094571909u32; k[47] = 275423344u32;
    k[48] = 430227734u32;  k[49] = 506948616u32;  k[50] = 659060556u32;  k[51] = 883997877u32;
    k[52] = 958139571u32;  k[53] = 1322822218u32; k[54] = 1537002063u32; k[55] = 1747873779u32;
    k[56] = 1955562222u32; k[57] = 2024104815u32; k[58] = 2227730452u32; k[59] = 2361852424u32;
    k[60] = 2428436474u32; k[61] = 2756734187u32; k[62] = 3204031479u32; k[63] = 3329325298u32;

    // --- Initial hash state H[0..8] -------------------------------------
    let h: [u32; 8] = alloc u32, 8;
    h[0] = 1779033703u32;
    h[1] = 3144134277u32;
    h[2] = 1013904242u32;
    h[3] = 2773480762u32;
    h[4] = 1359893119u32;
    h[5] = 2600822924u32;
    h[6] = 528734635u32;
    h[7] = 1541459225u32;

    // --- Total number of 64-byte blocks after padding -------------------
    // Padding: one 128 (0x80) byte, zero or more 0 bytes, then 8-byte length.
    // Padded length = ((len + 9 + 63) / 64) * 64.
    let nblocks: u64 = (len + 9u64 + 63u64) / 64u64;

    // --- Bit length, big-endian, computed once --------------------------
    // bitlen = len * 8 (mod 2^64). Caller is trusted not to overflow.
    let bitlen: u64 = len << 3;

    // --- Working buffers ------------------------------------------------
    let w: [u32; 64] = alloc u32, 64;     // message schedule
    let blk: [u8; 64] = alloc u8, 64;     // current 64-byte block

    let b: u64 = 0u64;
    while b < nblocks {
        // Block `b` covers byte offsets [b*64 .. b*64 + 64) of the padded stream.
        let base: u64 = b * 64u64;

        // Per-byte fill of blk:
        //   - real input byte:                 off <  len
        //   - 128 terminator byte:             off == len
        //   - zero pad byte:                   len < off < (nblocks*64 - 8)
        //   - one of 8 length bytes:           off >= nblocks*64 - 8
        let len_start: u64 = nblocks * 64u64 - 8u64;

        let i: u64 = 0u64;
        while i < 64u64 {
            let off: u64 = base + i;
            if off < len {
                blk[i] = input[off];
            } else {
                if off == len {
                    blk[i] = 128u8;
                } else {
                    if off < len_start {
                        blk[i] = 0u8;
                    } else {
                        // Length byte: off - len_start is in 0..8, big-endian.
                        let lpos: u32 = (u32) off - (u32)len_start;
                        let shift: u32 = (7u32 - lpos) * 8u32;
                        blk[i] = (u8) ((bitlen >> shift) & 255u64);
                    }
                }
            }
            i = i + 1u64;
        }

        // --- Prepare message schedule w[0..64] --------------------------
        // w[0..16]: big-endian load from blk
        let j: u64 = 0u64;
        while j < 16u64 {
            let p: u64 = j * 4u64;
            let b0: u32 = (u32) blk[p];
            let b1: u32 = (u32) blk[p + 1u64];
            let b2: u32 = (u32) blk[p + 2u64];
            let b3: u32 = (u32) blk[p + 3u64];
            w[j] = (b0 << 24u32) | (b1 << 16u32) | (b2 << 8u32) | b3;
            j = j + 1u64;
        }

        // w[16..64]:
        //   sigma0(x) = rotr(x,7)  ^ rotr(x,18) ^ (x >> 3)
        //   sigma1(x) = rotr(x,17) ^ rotr(x,19) ^ (x >> 10)
        //   w[t] = sigma1(w[t-2]) + w[t-7] + sigma0(w[t-15]) + w[t-16]
        j = 16u64;
        while j < 64u64 {
            let x15: u32 = w[j - 15u64];
            let s0: u32 = ((x15 >> 7u32)  | (x15 << 25u32))
                       ^ ((x15 >> 18u32) | (x15 << 14u32))
                       ^ (x15 >> 3u32);

            let x2: u32 = w[j - 2u64];
            let s1: u32 = ((x2 >> 17u32) | (x2 << 15u32))
                       ^ ((x2 >> 19u32) | (x2 << 13u32))
                       ^ (x2 >> 10u32);

            w[j] = w[j - 16u64] + s0 + w[j - 7u64] + s1;
            j = j + 1u64;
        }

        // --- Compression: working variables a..hh -----------------------
        let a: u32 = h[0];
        let bb: u32 = h[1];
        let c: u32 = h[2];
        let d: u32 = h[3];
        let e: u32 = h[4];
        let f: u32 = h[5];
        let g: u32 = h[6];
        let hh: u32 = h[7];

        let t: u64 = 0u64;
        while t < 64u64 {
            // Sigma1(e) = rotr(e,6) ^ rotr(e,11) ^ rotr(e,25)
            let S1: u32 = ((e >> 6u32)  | (e << 26u32))
                       ^ ((e >> 11u32) | (e << 21u32))
                       ^ ((e >> 25u32) | (e << 7u32));
            // Ch(e,f,g) = (e & f) ^ (~e & g)
            let ch: u32 = (e & f) ^ ((~e) & g);
            let temp1: u32 = hh + S1 + ch + k[t] + w[t];

            // Sigma0(a) = rotr(a,2) ^ rotr(a,13) ^ rotr(a,22)
            let S0: u32 = ((a >> 2u32)  | (a << 30u32))
                       ^ ((a >> 13u32) | (a << 19u32))
                       ^ ((a >> 22u32) | (a << 10u32));
            // Maj(a,bb,c) = (a & bb) ^ (a & c) ^ (bb & c)
            let mj: u32 = (a & bb) ^ (a & c) ^ (bb & c);
            let temp2: u32 = S0 + mj;

            hh = g;
            g = f;
            f = e;
            e = d + temp1;
            d = c;
            c = bb;
            bb = a;
            a = temp1 + temp2;

            t = t + 1u64;
        }

        // --- Add this chunk's compressed result to current hash value ---
        h[0] = h[0] + a;
        h[1] = h[1] + bb;
        h[2] = h[2] + c;
        h[3] = h[3] + d;
        h[4] = h[4] + e;
        h[5] = h[5] + f;
        h[6] = h[6] + g;
        h[7] = h[7] + hh;

        b = b + 1u64;
    }

    // --- Serialize H[0..8] as 32 big-endian bytes into output -----------
    let m: u64 = 0u64;
    while m < 8u64 {
        let v: u32 = h[m];
        let q: u64 = m * 4u64;
        output[q]        = (u8) ((v >> 24u32) & 255u32);
        output[q + 1u64] = (u8) ((v >> 16u32) & 255u32);
        output[q + 2u64] = (u8) ((v >> 8u32)  & 255u32);
        output[q + 3u64] = (u8) (v & 255u32);
        m = m + 1u64;
    }

    // --- Release the locals we allocated --------------------------------
    free w;
    free blk;
    free h;
    free k;

    return;
}