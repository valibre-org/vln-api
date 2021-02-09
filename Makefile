.PHONY: valor_bin default clean build_plugins pack

PLUGINS=blockchain wallet
OUT_DIR=.build
NATIVE_PLUGINS=$(PLUGINS:%=${OUT_DIR}/plugins/%)
CODEDEPLOY_FILES=$(shell find -L .codedeploy -type f)
VALOR_BIN=~/.cargo/bin/valor_bin
VALOR_VER ?= 0.4.7-beta.0

default: build_plugins

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

target/release/lib%.so:
	cargo build -p $* --release

$(OUT_DIR)/valor: valor_bin
	@mkdir -p $(@D); cp $(VALOR_BIN) $@

$(OUT_DIR)/plugins/%: target/release/lib%.so plugins/%/src/lib.rs plugins/%/Cargo.toml
	@mkdir -p $(@D)
	mv $< $@ 
