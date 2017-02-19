BUILDDIR ?= $(OUT_DIR)/build
ifdef NVFANCONTROL_MULTILIB
CC = gcc -m32
else
CC = gcc
endif
AR = ar
LIBTOOL = libtool
ifdef NVFANCONTROL_MULTILIB
NVCTRL = /usr/lib32/libXNVCtrl.a
else
NVCTRL = /usr/lib/libXNVCtrl.a
endif

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


