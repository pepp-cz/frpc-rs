
extern crate collections;

mod b64;
mod frpc;

fn main() {
    let args = std::os::args_as_bytes();
    let use_arg = args.len() > 1;
    let input = if use_arg { Vec::new() } else {std::io::stdin().read_to_end().unwrap()};
    let bytes = if use_arg {
        &args[1]
    } else {
        &input
    };
    let data = b64::decode_to_vec(bytes.as_slice());
    let strct = frpc::decode(data.as_slice());
    //let str = match str::from_utf8(data.as_slice()) {
    //    None => fail!("Decoded string is not valid utf8"),
    //    Some(s) => s
    //};
    println!("{}", strct);
}

