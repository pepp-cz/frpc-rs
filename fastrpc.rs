
extern crate collections;
extern crate num;

use std::str;
use collections::hashmap;
use collections::hashmap::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::fmt::Result;
use std::io::IoError;

mod b64 {
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
            |bytes| vec.extend(&mut bytes.iter().map(|x| *x))
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
}

mod frpc {
    use collections::hashmap::HashMap;
    use std::fmt;
    use std::io;

    #[deriving(Eq)]
    pub enum Value {
        Integer(i64),  // 1 = i32, 7 = +i64, 8 = -i64 
        Bool(bool),    // 2
        Double(f64),   // 3 - little endian IEEE 754
        String(~str),  // 4
        Datetime,      // 5 - TODO
        Binary(~[u8]), // 6
        Struct(HashMap<~str, Value>), // 10
        Array(Vec<Value>), // 11
        Null,          // 12
    }

    #[deriving(Eq, Show)]
    pub enum RPC {
        Call(~str, Value), // 13 method call
        Success(Value),         // 14 method reponse
        Fault(i32, ~str),  // 15 fault response
    }

    impl fmt::Show for Value {
        fn fmt(&self, fmtr : &mut fmt::Formatter) -> fmt::Result {
            match *self {
                Integer(v) => v.fmt(fmtr),
                Bool(v) => v.fmt(fmtr),
                Double(v) => v.fmt(fmtr),
                String(ref s) => {
                    try!(fmtr.buf.write_char('"'));
                    try!(s.fmt(fmtr));
                    fmtr.buf.write_char('"')
                },
                Datetime => unimplemented!(),
                Binary(_) => unimplemented!(),
                Struct(ref v) => {
                    try!(fmtr.buf.write_str("{"));
                    let mut s : &'static str = "";
                    for (key, val) in v.iter() {
                        try!(fmtr.buf.write_str(s));
                        try!(key.fmt(fmtr));
                        try!(fmtr.buf.write_str(" : "));
                        try!(val.fmt(fmtr));
                        s = ", ";
                    }
                    fmtr.buf.write_str("}")
                },
                Array(ref v) => {
                    try!(fmtr.buf.write_str("["));
                    let mut s : &'static str = "";
                    for item in v.iter() {
                        try!(fmtr.buf.write_str(s));
                        try!(item.fmt(fmtr));
                        s = ", ";
                    }
                    fmtr.buf.write_str("]")
                },
                Null => fmtr.buf.write_str("null")
            }
        }
    }

    struct ParseContext<'r>{
        pos : uint,
        data : &'r [u8]
    }

    struct ParseError {
        pos : uint,
        reason : &'static str
    }

    // TODO return error
    fn decode_u32<'r>(data : &'r [u8], len : uint) -> (u32, &'r [u8]) {
        let mut val = 0u32;
        if len > 0 { val = data[0] as u32 }
        if len > 1 { val += data[1] as u32 << 8 }
        if len > 2 { val += data[2] as u32 << 16 }
        if len > 3 { val += data[3] as u32 << 24 }
        (val, data.slice_from(len))
    }

    // TODO return error
    fn decode_u64<'r>(data : &'r [u8], len : uint) -> (u64, &'r [u8]) {
        let mut val = 0u64;
        if len > 0 { val = data[0] as u64 }
        if len > 1 { val += data[1] as u64 << 8 }
        if len > 2 { val += data[2] as u64 << 16 }
        if len > 3 { val += data[3] as u64 << 24 }
        if len > 4 { val += data[4] as u64 << 32 }
        if len > 5 { val += data[5] as u64 << 40 }
        if len > 6 { val += data[6] as u64 << 48 }
        if len > 7 { val += data[7] as u64 << 56 }
        (val, data.slice_from(len))
    }

    fn decode_name<'r>(data : &'r[u8]) -> Option<(&'r str, &'r[u8])> {
        match data {
            [len, ..rest] if (rest.len() >= (len as uint)) => {
                let len = len as uint;
                let name = ::std::str::from_utf8(rest.slice(0, len)).unwrap();
                Some((name, rest.slice_from(len)))
            },
            _ => None
        }
    }

    fn decode_value<'r>(data : &'r [u8]) -> Option<(Value, &'r [u8])> {
        match data {
            // Integer  - TODO is it legal in ver 2.0?
            [tag, ..rest] if (tag >> 3) == 1 => {
                let len = (tag & 7) as uint;
                let (val, rest) = decode_u32(rest, len);
                Some((Integer(val as i32 as i64), rest))
            },
            // Bool
            [tag, ..rest] if (tag >> 3) == 2 => {
                Some((Bool(if (tag & 1) == 1 {true} else {false}), rest))
            },
            //[tag, ..rest] if (tag >> 3) == 3 => { None }, // Double
            // String
            [tag, ..rest] if (tag >> 3) == 4 => {
                let len_size = (tag & 7) as uint + 1;
                let (len, rest) = decode_u64(rest, len_size);
                let len = len as uint;
                let str = ::str::from_utf8(rest.slice(0, len)).unwrap();
                Some((String(str.to_owned()), rest.slice_from(len)))
            },
            // [tag, ..rest] if (tag >> 3) == 5 => { None }, // Datetime
            // Binary
            [tag, ..rest] if (tag >> 3) == 6 => {
                let len_size = (tag & 7) as uint + 1;
                let (len, rest) = decode_u64(rest, len_size);
                let len = len as uint;
                Some((Binary(rest.slice(0, len).to_owned()), rest.slice_from(len)))
            },
            // positive Integer8
            [tag, ..rest] if (tag >> 3) == 7 => {
                let len = (tag & 7) as uint + 1;
                let (val, rest) = decode_u64(rest, len);
                Some((Integer(val as i64), rest))
            },
            // negative Integer8
            [tag, ..rest] if (tag >> 3) == 8 => {
                let len = (tag & 7) as uint + 1;
                let (val, rest) = decode_u64(rest, len);
                Some((Integer(-(val as i64)), rest))
            },
            // Struct
            [tag, ..rest] if (tag >> 3) == 10 => {
                let len_size = (tag & 7) as uint + 1;
                let (len, mut rest) = decode_u64(rest, len_size);
                let mut len = len as uint;
                let mut values = ::HashMap::<~str, Value>::with_capacity(len);
                while len > 0 {
                    let (name, r) = decode_name(rest).unwrap();
                    match decode_value(r) {
                        Some((v, r)) => { rest = r; values.insert(name.to_owned(), v); },
                        None => return None
                    }
                    len -= 1;
                };
                Some((Struct(values), rest))
            },
            // Array
            [tag, ..rest] if (tag >> 3) == 11 => {
                let len_size = (tag & 7) as uint + 1;
                let (len, mut rest) = decode_u64(rest, len_size);
                let mut len = len as uint;
                let mut values = Vec::<Value>::with_capacity(len);
                while len > 0 {
                    match decode_value(rest) {
                        Some((v, r)) => { rest = r; values.push(v); },
                        None => return None
                    }
                    len -= 1;
                };
                Some((Array(values), rest))
            },
            // Null
            [tag, ..rest] if (tag >> 3) == 12 => {
                Some((Null, rest))
            },
            _ => None
        }
    }

    fn decode_rpc(data : &[u8]) -> Option<RPC> {
        match data {
            // Call
            [tag, ..rest] if (tag >> 3) == 13 => {
                let (name, rest) = decode_name(rest).unwrap();
                let (value, _) = decode_value(rest).unwrap();
                Some(Call(name.to_owned(), value))
            }
            // Success
            [tag, ..rest] if (tag >> 3) == 14 => {
                let (value, _) = decode_value(rest).unwrap();
                Some(Success(value))
            },
            // Fault
            //[tag, ..rest] if (tag >> 3) == 15 => {
            //    let (val, rest) = decode_value(rest).unwrap();
            //    let (s, rest) = decode_value(rest).unwrap();
            //    Some(Fault(val, s))
            //},
            _ => None
        }
    }

    pub fn decode(data : &[u8]) -> Option<RPC> {
        match data {
            [0xCA, 0x11, 2, 0, ..rest] => decode_rpc(rest),
            [..rest] => decode_rpc(rest)
        }
    }

    #[test]
    fn test_decode() {
        assert_eq!(decode([0xca, 0x11, 2, 0]), None);
        assert_eq!(decode([0xca, 0x11, 2, 0, 104, 4, 116, 101, 115, 116, 96]), Some(Call(~"test", Null)))
    }
}

fn main() {
    let args = std::os::args_as_bytes();
    let bytes = if args.len() > 1 {args[1]} else {std::io::stdin().read_to_end().unwrap()};
    let data = b64::decode_to_vec(bytes);
    let strct = frpc::decode(data.as_slice());
    //let str = match str::from_utf8(data.as_slice()) {
    //    None => fail!("Decoded string is not valid utf8"),
    //    Some(s) => s
    //};
    println!("{}", strct);
}

