.PHONY: run valor_bin default clean build_plugins pack

PLUGINS=blockchain wallet transfers capture_url
OUT_DIR=.build
NATIVE_PLUGINS=$(PLUGINS:%=${OUT_DIR}/plugins/%)
CODEDEPLOY_FILES=$(shell find -L .codedeploy -type f)
VALOR_BIN=~/.cargo/bin/valor_bin
VALOR_VER ?= 0.4.7-beta.0

ifeq ($(OS),Windows_NT)
    uname_S := Windows
else
    uname_S := $(shell uname -s)
endif

ifeq ($(uname_S), Windows)
    LIB_EXT = .dll
endif
ifeq ($(uname_S), Darwin)
    LIB_EXT = .dylib
endif
ifeq ($(uname_S), Linux)
    LIB_EXT = .so
endif

default: build_plugins

run: clean $(OUT_DIR)/valor $(NATIVE_PLUGINS)
	LD_LIBRARY_PATH=$(OUT_DIR)/plugins $(OUT_DIR)/valor -p plugins.json

pack: app.zip

build_plugins: $(NATIVE_PLUGINS) 

valor_bin:
	cargo install -f valor_bin --version $(VALOR_VER) --target-dir target

clean: 
	rm -f $(NATIVE_PLUGINS) app.zip 

app.zip: $(OUT_DIR)/valor $(NATIVE_PLUGINS)
	@zip app -j $(CODEDEPLOY_FILES)
	@zip app $<
	@zip app plugins.json
	@zip app $(filter-out $<,$^)

target/release/lib%$(LIB_EXT):
	cargo build -p $* --release

$(OUT_DIR)/valor: valor_bin
	@mkdir -p $(@D); cp $(VALOR_BIN) $@

$(OUT_DIR)/plugins/%: target/release/lib%$(LIB_EXT) plugins/%/src/lib.rs plugins/%/Cargo.toml
	@mkdir -p $(@D)
	mv $< $@ 
