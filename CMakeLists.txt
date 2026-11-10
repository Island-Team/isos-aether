cmake_minimum_required(VERSION 3.22)
project(IsOS VERSION 0.2.1 LANGUAGES NONE)

# Development build convenience only. Product installation is owned by install.sh.
option(ISLAND_BUILD_EOS "Build the EOS Rust v2 runtime" ON)
option(ISLAND_BUILD_EI "Build the EI Python distribution" ON)
option(ISLAND_BUILD_ISOSUI "Build the IsOS UI" OFF)
option(ISLAND_BUILD_TESTS "Register repository build tests" OFF)

if(ISLAND_BUILD_TESTS)
  enable_testing()
endif()

if(ISLAND_BUILD_EOS)
  find_program(ISLAND_CARGO_EXECUTABLE NAMES cargo
    HINTS "$ENV{HOME}/.cargo/bin" REQUIRED)
  add_custom_target(eos_rust ALL
    COMMAND "${ISLAND_CARGO_EXECUTABLE}" build --locked --release --workspace
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
    COMMENT "Building the IsOS Rust workspace"
    USES_TERMINAL VERBATIM)
endif()

if(ISLAND_BUILD_EI)
  add_custom_target(ei_dist
    COMMAND "${CMAKE_SOURCE_DIR}/ei/scripts/platform/dispatch.sh" build
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}/ei"
    COMMENT "Building the EI distribution"
    USES_TERMINAL VERBATIM)
endif()

if(ISLAND_BUILD_ISOSUI)
  find_program(ISLAND_NPM_EXECUTABLE NAMES npm REQUIRED)
  add_custom_target(isosui
    COMMAND "${ISLAND_NPM_EXECUTABLE}" run build
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}/ui"
    COMMENT "Building both IsOS UI clients"
    USES_TERMINAL VERBATIM)
endif()
