PREFIX ?= /usr/local/kappa_processor
BINDIR ?= $(PREFIX)/bin
LIBDIR ?= $(PREFIX)/lib
MODULESDIR ?= $(PREFIX)/modules

EXECUTABLE = kappa_processor
LIBS = libconnectors_macro.so libstreamer_block_macro.so
MODULES = libfilter.so

all: deploy-debug

deploy-debug: debug install_debug

deploy: release install

debug:
	cargo build
release:
	cargo build --release
test:
	cargo test
clean:
	cargo clean

${LIBS}: %.so:
	install -m 755 target/debug/$@ $(LIBDIR)/

${MODULES}: %.so:
	install -m 755 target/debug/$@ $(MODULESDIR)/

deploy-folder-tree:
	if [ -d ${PREFIX} ]; then \
	    rm -rf ${PREFIX}; \
	fi
	mkdir -p $(BINDIR)
	mkdir -p $(LIBDIR)
	mkdir -p $(MODULESDIR)

install_debug: deploy-folder-tree $(MODULES) $(LIBS)
	install -m 755 target/debug/$(EXECUTABLE) $(BINDIR)/

install: deploy-folder-tree $(MODULES) $(LIBS)
	install -m 755 target/release/$(EXECUTABLE) $(BINDIR)/
	
.PHONY: all build release test clean install