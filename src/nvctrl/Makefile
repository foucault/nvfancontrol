BUILDDIR ?= $(OUT_DIR)/build
CC = gcc
AR = ar
LIBTOOL = libtool
NVCTRL = /usr/lib/libXNVCtrl.a
CFLAGS ?= -static -O2 -fPIC -Wall -pedantic -Wl,-Bdynamic -lX11 -lXext -Wl,-Bstatic -lXNVCtrl
BASENAME = nvctrl_c
SRC = $(BASENAME).c
OBJ = $(BUILDDIR)/lib$(BASENAME).o
OUT = $(BUILDDIR)/lib$(BASENAME).a
SHELL := /bin/bash

.PHONY: $(BUILDDIR)

default: library

library: $(OBJ)
	echo -e "CREATE $(OUT)\nADDLIB $(NVCTRL)\nADDMOD $(OBJ)\nSAVE\nEND" | $(AR) -M


$(OBJ): $(BUILDDIR)
	$(CC) -c -o $(OBJ) $(CFLAGS) $(SRC)

$(BUILDDIR):
	mkdir -p $(BUILDDIR)


