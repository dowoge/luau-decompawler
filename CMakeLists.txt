# luau_check and luau_run: the real Luau compiler and VM, which the test harness
# compares the decompiler against. These are C++ because Luau is. The decompiler
# and the harness are the Rust crate at the root.
#
#   git submodule update --init
#   cmake -S . -B build
#   cmake --build build --config Release --target luau_check luau_run
cmake_minimum_required(VERSION 3.16)
project(luau_decompiler_tools LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(NOT DEFINED LUAU_DIR)
    set(LUAU_DIR "${CMAKE_CURRENT_SOURCE_DIR}/third_party/luau")
endif()
if(NOT EXISTS "${LUAU_DIR}/CMakeLists.txt")
    message(FATAL_ERROR "Luau not found at ${LUAU_DIR}: run git submodule update --init")
endif()

# Only the libraries linked below. These must be cache entries: Luau's own
# cmake_minimum_required would otherwise let its option() override them.
set(LUAU_BUILD_CLI OFF CACHE BOOL "Build Luau CLI" FORCE)
set(LUAU_BUILD_TESTS OFF CACHE BOOL "Build Luau tests" FORCE)
add_subdirectory("${LUAU_DIR}" "${CMAKE_CURRENT_BINARY_DIR}/luau" EXCLUDE_FROM_ALL)

find_package(Threads REQUIRED)

add_executable(luau_check tools/luau_check.cpp)
target_link_libraries(luau_check PRIVATE Luau.Compiler Luau.Ast Luau.Common Threads::Threads)

add_executable(luau_run tools/luau_run.cpp)
target_link_libraries(luau_run PRIVATE Luau.VM Luau.Compiler Luau.Ast Luau.Common)
