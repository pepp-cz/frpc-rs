
fastrpc: fastrpc.rs
	rustc fastrpc.rs

fastrpc-test: fastrpc.rs b64.rs frpc.rs
	rustc --test -o fastrpc-test fastrpc.rs

all: fastrpc fastrpc-test

test: fastrpc-test
	./fastrpc-test

clean:
	@rm -Rf fastrpc-test fastrpc

.PHONE: all test clean

