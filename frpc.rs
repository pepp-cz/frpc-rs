
use collections::hashmap::HashMap;
use std::fmt;
use std::str;

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
            let name = str::from_utf8(rest.slice(0, len)).unwrap();
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
            let str = str::from_utf8(rest.slice(0, len)).unwrap();
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
            let mut values = HashMap::<~str, Value>::with_capacity(len);
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

