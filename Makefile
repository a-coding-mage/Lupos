# Lupos Makefile (Linux-style build front-end)
#
# Invoke natively on Linux:
#   make config
#   make kernel
#   make image
#   make test
#   make run
#
# The Rust build remains the source of truth: these targets delegate to
# `cargo xtask` for kernel builds, image builds, and QEMU runs.

SHELL := /bin/bash

.DEFAULT_GOAL := image

KCONFIG          := src/kernel/Kconfig
DEFCONFIG        := configs/lupos_defconfig
TINY_BASE_CONFIG := configs/tiny-base.config

KCONFIG_DIR      := vendor/linux/scripts/kconfig
KCONFIG_CONF     := $(KCONFIG_DIR)/conf
KCONFIG_MCONF    := $(KCONFIG_DIR)/mconf
KCONFIG_NCONF    := $(KCONFIG_DIR)/nconf

export KCONFIG_AUTOCONFIG := src/include/config/auto.conf
export KCONFIG_AUTOHEADER := src/include/generated/autoconf.h
export KCONFIG_RUSTCCFG   := src/include/generated/rustc_cfg

# Build conf/mconf/nconf out-of-tree.  Upstream kbuild ties these to its
# hostprogs machinery (vendor/linux/scripts/kconfig/Makefile L174-L199),
# which only resolves when invoked from the Linux top-level Makefile.  We
# explicitly do not invoke that, so the bare `make -C vendor/linux/scripts/kconfig <tool>`
# would fall through to make's implicit %: %.c rule and drop -I vendor/linux/scripts/include
# and ncurses linkage.  Delegate to a shared helper that mirrors the upstream
# object lists instead.
HOSTCC           ?= cc
HOSTPKG_CONFIG   ?= pkg-config
KCONFIG_TOOL_ENV := HOSTCC="$(HOSTCC)" HOSTPKG_CONFIG="$(HOSTPKG_CONFIG)"

CARGO ?= cargo

.PHONY: help
help:
	@echo "Lupos build front-end (delegates to cargo xtask)"
	@echo ""
	@echo "Config:"
	@echo "  make config              - update .config, falling back to $(DEFCONFIG)"
	@echo "  make lupos_defconfig      - write .config from $(DEFCONFIG) and sync headers"
	@echo "  make menuconfig           - edit .config via Kconfig menu UI"
	@echo "  make nconfig              - edit .config via Kconfig nconfig UI"
	@echo "  make localconfig          - configure for THIS host (probe + seed), then 'make build'"
	@echo "  make localmodconfig       - generate a device-tuned .config"
	@echo "  make allnoconfig          - answer 'n' to every config prompt"
	@echo "  make allyesconfig         - answer 'y' to every config prompt"
	@echo "  make allmodconfig         - answer 'm' where possible"
	@echo "  make alldefconfig         - reset all symbols to their Kconfig defaults"
	@echo "  make tinyconfig           - minimal config preset using $(TINY_BASE_CONFIG)"
	@echo ""
	@echo "Public build:"
	@echo "  make kernel              - build a pure Lupos kernel (ELF + bzImage); = cargo xtask build"
	@echo "  make image / make        - build the Arch userland + bootable kernel ISO; = cargo xtask build --userland --iso"
	@echo "  make build               - alias of 'make kernel' (pure kernel)"
	@echo ""
	@echo "Backend build targets:"
	@echo "  make userland             - stage the minimal Arch userland"
	@echo "  make iso                  - build kernel ELF + GRUB ISO"
	@echo "  make bzImage              - build a Linux-style kernel image artifact"
	@echo "  make modules              - stage CONFIG_*=m driver modules"
	@echo "  make modules_install      - install staged modules under INSTALL_MOD_PATH"
	@echo "  make install              - stage kernel, modules, and boot artifacts"
	@echo ""
	@echo "Run:"
	@echo "  make run                  - boot the default distro with a VGA window"
	@echo "  make run-gui              - boot into LightDM and the XFCE desktop (X11)"
	@echo "  make run-headless         - boot the default distro through the automated gate"
	@echo ""
	@echo "Tests:"
	@echo "  make test                 - run the public cargo xtask test gate"
	@echo "  make test-boot            - run cargo xtask test --boot"
	@echo "  make ping-smoke           - run cargo xtask run --ping-smoke"

.PHONY: all
all: image

.PHONY: kernel
kernel: syncconfig
	@$(CARGO) xtask build

.PHONY: image
image: syncconfig
	@$(CARGO) xtask build --userland --iso

.PHONY: build
build: kernel

$(KCONFIG_CONF):
	@$(KCONFIG_TOOL_ENV) bash src/scripts/build_kconfig_tools.sh conf

$(KCONFIG_MCONF):
	@$(KCONFIG_TOOL_ENV) bash src/scripts/build_kconfig_tools.sh mconf

$(KCONFIG_NCONF):
	@$(KCONFIG_TOOL_ENV) bash src/scripts/build_kconfig_tools.sh nconf

.PHONY: lupos_defconfig
lupos_defconfig: $(KCONFIG_CONF) $(KCONFIG) $(DEFCONFIG)
	@echo "*** Default configuration is based on '$(DEFCONFIG)'"
	@$(KCONFIG_CONF) --defconfig=$(DEFCONFIG) $(KCONFIG)
	@$(KCONFIG_CONF) --syncconfig $(KCONFIG)

.PHONY: defconfig
defconfig: lupos_defconfig

.PHONY: syncconfig
syncconfig: $(KCONFIG_CONF) $(KCONFIG)
	@if [ ! -f .config ]; then \
		echo "*** No .config found; using '$(DEFCONFIG)'"; \
		$(KCONFIG_CONF) --defconfig=$(DEFCONFIG) $(KCONFIG); \
	fi
	@$(KCONFIG_CONF) --syncconfig $(KCONFIG)

.PHONY: config
config: $(KCONFIG_CONF) $(KCONFIG)
	@if [ ! -f .config ]; then \
		echo "*** No .config found; using '$(DEFCONFIG)'"; \
		$(KCONFIG_CONF) --defconfig=$(DEFCONFIG) $(KCONFIG); \
	fi
	@$(KCONFIG_CONF) --olddefconfig $(KCONFIG)
	@$(KCONFIG_CONF) --syncconfig $(KCONFIG)

.PHONY: menuconfig
menuconfig: $(KCONFIG_MCONF) $(KCONFIG)
	@$(KCONFIG_MCONF) $(KCONFIG)
	@$(KCONFIG_CONF) --syncconfig $(KCONFIG)

.PHONY: nconfig
nconfig: $(KCONFIG_NCONF) $(KCONFIG)
	@$(KCONFIG_NCONF) $(KCONFIG)
	@$(KCONFIG_CONF) --syncconfig $(KCONFIG)

.PHONY: allnoconfig
allnoconfig: $(KCONFIG_CONF) $(KCONFIG)
	@$(KCONFIG_CONF) --allnoconfig $(KCONFIG)
	@$(KCONFIG_CONF) --syncconfig $(KCONFIG)

.PHONY: tinyconfig
tinyconfig: $(KCONFIG_CONF) $(KCONFIG) $(TINY_BASE_CONFIG)
	@KCONFIG_ALLCONFIG=$(TINY_BASE_CONFIG) $(KCONFIG_CONF) --allnoconfig $(KCONFIG)
	@$(KCONFIG_CONF) --syncconfig $(KCONFIG)

.PHONY: allyesconfig
allyesconfig: $(KCONFIG_CONF) $(KCONFIG)
	@$(KCONFIG_CONF) --allyesconfig $(KCONFIG)
	@$(KCONFIG_CONF) --syncconfig $(KCONFIG)

.PHONY: allmodconfig
allmodconfig: $(KCONFIG_CONF) $(KCONFIG)
	@$(KCONFIG_CONF) --allmodconfig $(KCONFIG)
	@$(KCONFIG_CONF) --syncconfig $(KCONFIG)

.PHONY: alldefconfig
alldefconfig: $(KCONFIG_CONF) $(KCONFIG)
	@$(KCONFIG_CONF) --alldefconfig $(KCONFIG)
	@$(KCONFIG_CONF) --syncconfig $(KCONFIG)

.PHONY: localmodconfig
localmodconfig: $(KCONFIG_CONF) $(KCONFIG) $(DEFCONFIG)
	@KCONFIG_CONF="$(KCONFIG_CONF)" KCONFIG="$(KCONFIG)" DEFCONFIG="$(DEFCONFIG)" bash src/scripts/localmodconfig.sh

.PHONY: localconfig
localconfig: $(KCONFIG_CONF) $(KCONFIG) $(DEFCONFIG)
	@KCONFIG_CONF="$(KCONFIG_CONF)" KCONFIG="$(KCONFIG)" DEFCONFIG="$(DEFCONFIG)" bash src/scripts/localconfig.sh

.PHONY: userland
userland: syncconfig
	@$(CARGO) xtask userland-build

.PHONY: iso
iso: syncconfig
	@$(CARGO) xtask iso

.PHONY: bzImage
bzImage: syncconfig
	@$(CARGO) xtask bzImage

.PHONY: modules
modules: syncconfig
	@$(CARGO) xtask modules

.PHONY: modules_install
modules_install: syncconfig
	@$(CARGO) xtask modules-install

.PHONY: install
install: syncconfig
	@$(CARGO) xtask install

.PHONY: run
run: syncconfig
	@$(CARGO) xtask run

.PHONY: run-gui
run-gui: syncconfig
	@$(CARGO) xtask run --gui

.PHONY: run-headless
run-headless: syncconfig
	@$(CARGO) xtask run --headless

.PHONY: run-display
run-display: run

.PHONY: test-boot
test-boot: syncconfig
	@$(CARGO) xtask test --boot

.PHONY: test
test: syncconfig
	@$(CARGO) xtask test

.PHONY: ping-smoke
ping-smoke: syncconfig
	@$(CARGO) xtask run --ping-smoke
