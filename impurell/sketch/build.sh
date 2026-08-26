#!/bin/sh

llc test.ll --filetype=obj -o build/test.o
clang -o build/imp_rt.o lmp_rt.c
clang -o test build/test.o build/imp_rt.o

llc test.ll --filetype=obj -o build/test.o
clang -o build/imp_rt.o lmp_rt.c
clang -o test build/test.o build/imp_rt.o
