.PHONY: run valor default clean plugins pack

UNAME:=$(shell uname -s)
ifeq ($(UNAME), Darwin)
	LIB_PRE=lib
	LIB_EXT=.dylib
endif
ifeq ($(UNAME), Linux)
	LIB_PRE=lib
	LIB_EXT=.so
endif

PLUGINS=blockchain capture_url html_renderer
OUT_DIR ?= .build
NATIVE_PLUGINS_PATTERN=${OUT_DIR}/plugins/${LIB_PRE}%${LIB_EXT}
NATIVE_PLUGINS=$(PLUGINS:%=${NATIVE_PLUGINS_PATTERN})
CODEDEPLOY_FILES=$(shell find -L .codedeploy -type f)
VALOR_VER ?= 0.5.2-beta.0
VALOR_GIT=https://github.com/valibre-org/valor.git

default: valor plugins

run: valor plugins
	LD_LIBRARY_PATH=$(OUT_DIR)/plugins $(OUT_DIR)/$< -p plugins.json

pack: app.zip

plugins: $(NATIVE_PLUGINS)

valor: $(OUT_DIR)/valor

clean: 
	rm -f $(OUT_DIR)/valor
	rm -f app.zip
	rm -f $(NATIVE_PLUGINS) 

app.zip: $(OUT_DIR)/valor $(NATIVE_PLUGINS)
	@zip app -j $(CODEDEPLOY_FILES)
	@zip app $<
	@zip app plugins.json
	@zip app $(filter-out $<,$^)

target/release/$(LIB_PRE)%$(LIB_EXT):
	cargo build -p $* --release

$(OUT_DIR)/valor:
	#cargo install -f valor_bin --version $(VALOR_VER) --target-dir target
	cargo install -f --target-dir target --git $(VALOR_GIT) --branch main valor_bin
	@mkdir -p $(@D); cp `which valor_bin` $@

$(NATIVE_PLUGINS_PATTERN): target/release/$(LIB_PRE)%$(LIB_EXT)
	@mkdir -p $(@D)
	mv $< $@ 
	strip $@
