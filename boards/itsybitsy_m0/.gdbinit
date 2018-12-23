# print demangled symbols by default
set print asm-demangle on

# JLink
target extended-remote :2331
monitor flash breakpoints 1
# allow hprints to show up in gdb
monitor semihosting enable
monitor semihosting IOClient 3

# reset *after* loading so that we run through the bootloader 
# and correctly bootstrap the loaded code
monitor halt
load
monitor reset

break HardFault

# OpenOCD
#target extended-remote :3333
#monitor arm semihosting enable
#load
#step
