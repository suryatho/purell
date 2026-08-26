BUILD_DIR = build

.PHONY: all compiler runtime clean

all: compiler runtime

compiler:
	cargo build --bin purell --release
	cp target/release/purell ./purell

runtime:
	mkdir -p $(BUILD_DIR)
	# rustc --edition 2021 \
	# 	-C opt-level=z \
	# 	-C panic=abort \
	# 	-C overflow-checks=off \
	# 	--emit=obj \
	# 	runtime/purellrt.rs -o $(BUILD_DIR)/purellrt.o
	cc -g -Wall -Wextra --nostdlib runtime/purellrt.c -o $(BUILD_DIR)/purellrt.o
	cp target/release/libpurellrt.a $(BUILD_DIR)/purellrt.o

clean:
	rm -rf $(BUILD_DIR)
	rm -f purell
	cargo clean
