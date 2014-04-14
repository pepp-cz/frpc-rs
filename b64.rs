
#[inline]
fn decode_byte(byte : u8) -> (bool, u8) {
    match byte {
        65..90 => (true, byte-65),   // A-Z
        97..122 => (true, byte-97+26), // a-z
        48..57 => (true, byte-48+52),  // 0-9
        43 => (true, 62), // +
        47 => (true, 63), // /
        61 => (true, 0), // =
        _ => (false, 0)
    }
}

#[inline]
fn decode_quartet(input : [u8, ..4], cb : |&[u8]|) {
        let mut buffer = [0u8, ..3];
        let pos = 0;
        let (mut valid, mut tmp) = decode_byte(input[pos]);
        
        let (byte_valid, value) = decode_byte(input[pos+1]);
        valid = valid && byte_valid;
        buffer[0] = (tmp << 2) | (value >> 4);
        tmp = value & 0xF;

        let (byte_valid, value) = decode_byte(input[pos+2]);
        valid = valid && byte_valid;
        buffer[1] = (tmp << 4) | (value >> 2);
        tmp = value & 0x3;

        let (byte_valid, value) = decode_byte(input[pos+3]);
        valid = valid && byte_valid;
        buffer[2] = (tmp << 6) | value;

        if !valid { fail!("Illegal characters") }

        cb(buffer);
}

pub fn decode_with_callback(input : &[u8], cb : |&[u8]|) {
    let mut to_decode = input.len();
    // remove trailing '='
    while to_decode > 0 && (input[to_decode - 1] == 61) { to_decode -= 1 }

    let mut pos = 0u;

    while to_decode >= 4 { // decode four bytes to three
        decode_quartet([input[pos], input[pos+1], input[pos+2], input[pos+3]], |bytes| cb(bytes));
        pos += 4;
        to_decode -= 4;
    }

    match to_decode {
        0 => (),
        1 => fail!("Input of invalid length"),
        2 => decode_quartet([input[pos], input[pos+1], 61, 61], |bytes| cb(bytes.slice(0,1))),
        3 => decode_quartet([input[pos], input[pos+1], input[pos+2], 61], |bytes| cb(bytes.slice(0,2))),
        _ => fail!("impossible")
    }
}

pub fn decode_to_vec(input : &[u8]) -> Vec<u8> {
    let mut vec = Vec::<u8>::new();
    decode_with_callback(input,
        |bytes| vec.extend(bytes.iter().map(|x| *x))
    );
    return vec;
}

#[test]
fn test_decode_with_callback() {
    let mut vec = Vec::<u8>::new();
    decode_with_callback(
        [97, 71, 86, 115, 98, 71, 56, 104],
        |bytes| vec.extend(&mut bytes.iter().map(|x| *x))
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

