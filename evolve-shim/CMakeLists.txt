cmake_minimum_required(VERSION 3.16)
project(evolve_shim CXX)

# Cross-compilation settings are in the toolchain file.
# Build with: cmake -B build -DCMAKE_TOOLCHAIN_FILE=../mingw-w64-x86_64.cmake
# Or directly: x86_64-w64-mingw32-g++ -O2 -Wall -shared -o build/evolve_shim.dll shim.cpp shim.def

add_library(evolve_shim SHARED shim.cpp shim.def)
set_target_properties(evolve_shim PROPERTIES PREFIX "" SUFFIX ".dll")
target_compile_options(evolve_shim PRIVATE -O2 -Wall)
