
use std::mem::transmute;
use std::raw::Slice;

#[inline]
fn decode_byte(byte : u8) -> u8 {
    match byte {
        65...90  => byte-65,   // A-Z
        97...122 => byte-97+26, // a-z
        48...57  => byte-48+52,  // 0-9
        43 => 62, // +
        47 => 63, // /
        61 => 0, // =
        _ => 64 // signals invalid char
    }
}

#[inline]
fn decode_quartet(input : [u8, ..4]) -> Option<[u8, ..3]> {
        let b1 = decode_byte(input[0]);
        let b2 = decode_byte(input[1]);
        let b3 = decode_byte(input[2]);
        let b4 = decode_byte(input[3]);
        
        let buffer : [u8, ..3] = [
            (b1 << 2) | (b2 >> 4),
            (b2 << 4) | (b3 >> 2),
            (b3 << 6) | b4
        ];

        if (64u8 & (b1 | b2 | b3 | b4)) != 0 { return None }

        Some(buffer)
}

#[inline]
fn decode_octet(input : [u8, ..8]) -> Option<[u8, ..6]> {
        let b1 = decode_byte(input[0]);
        let b2 = decode_byte(input[1]);
        let b3 = decode_byte(input[2]);
        let b4 = decode_byte(input[3]);
        let b5 = decode_byte(input[4]);
        let b6 = decode_byte(input[5]);
        let b7 = decode_byte(input[6]);
        let b8 = decode_byte(input[7]);
        
        let buffer : [u8, ..6] = [
            (b1 << 2) | (b2 >> 4),
            (b2 << 4) | (b3 >> 2),
            (b3 << 6) | (b4),
            (b5 << 2) | (b6 >> 4),
            (b6 << 4) | (b7 >> 2),
            (b7 << 6) | (b8)
        ];

        if (64u8 & (b1 | b2 | b3 | b4 | b5 | b6 | b7 | b8)) != 0 { return None }

        Some(buffer)
}

pub fn decode_with_callback(input : &[u8], cb : |&[u8]|) {
    let mut to_decode = input;

    while to_decode.last().map_or(false, |c| *c == 61u8) {
        to_decode = to_decode.init()
    }

    while to_decode.len() >= 4 { // decode four bytes to three
        let res = decode_quartet(unsafe {
            let s : Slice<u8> = transmute(to_decode);
            let a : &[u8, ..4] = transmute(s.data);
            *a}).expect("Decoding failed");
        cb(res);
        to_decode = to_decode.slice_from(4);
    }

    let (len, res) = match to_decode {
        [] => (0, Some([0, 0, 0])),
        [_] => panic!("Input of invalid length"),
        [a, b] => (1, decode_quartet([a, b, 61, 61])),
        [a, b, c] => (2, decode_quartet([a, b, c, 61])),
        _ => panic!("impossible")
    };
    if len > 0 {
        cb(res.expect("Decoding of trailing bytes failed").slice(0, len))
    }
}

pub fn decode_to_vec(input : &[u8]) -> Vec<u8> {
    let mut vec = Vec::<u8>::new();
    decode_with_callback(input,
        |bytes| {
            println!("adding {} bytes", bytes.len());
            vec.extend(bytes.iter().map(|x| *x))
        }
    );
    return vec;
}

#[test]
fn test_decode_with_callback() {
    let mut vec = Vec::new();
    decode_with_callback(
        [97, 71, 86, 115, 98, 71, 56, 104],
        |bytes| vec.extend(bytes.iter().map(|x| *x))
    );
    assert_eq!(vec, vec!(104, 101, 108, 108, 111, 33))
}

#[test]
fn test_decode_to_vec() {
    assert_eq!(decode_to_vec([97, 71, 86, 115, 98, 71, 56, 104]),
               vec!(104, 101, 108, 108, 111, 33));
    assert_eq!(decode_to_vec([86, 50, 86, 115, 89, 50, 57, 116, 90, 81]),
               vec!(87, 101, 108, 99, 111, 109, 101));
    assert_eq!(decode_to_vec([86, 50, 86, 115, 89, 50, 57, 116, 90, 81, 61, 61]),
               vec!(87, 101, 108, 99, 111, 109, 101));
    assert_eq!(decode_to_vec([86, 50, 86, 115, 89, 50, 57, 116, 90, 83, 66, 48, 98,
                              121, 66, 75, 89, 87, 49, 104, 97, 87, 78, 104, 73, 71,
                              70, 117, 90, 67, 66, 108, 98, 109, 112, 118, 101, 83,
                              66, 53, 98, 51, 86, 121, 73, 71, 104, 118, 98, 71,
                              120, 112, 90, 71, 70, 53]), 
               vec!(87, 101, 108, 99, 111, 109, 101, 32, 116, 111, 32, 74, 97, 109,
                    97, 105, 99, 97, 32, 97, 110, 100, 32, 101, 110, 106, 111, 121,
                    32, 121, 111, 117, 114, 32, 104, 111, 108, 108, 105, 100, 97, 121));
}

