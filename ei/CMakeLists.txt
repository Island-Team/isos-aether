cmake_minimum_required(VERSION 3.24)

project(IsOSEI VERSION 0.2.1 LANGUAGES NONE)

option(ISLAND_BUILD_EI "Expose the EI Python distribution target" ON)
option(ISLAND_BUILD_TESTS "Expose the repository verification target" OFF)

find_program(ISLAND_UV_EXECUTABLE NAMES uv)

if(ISLAND_BUILD_EI)
  if(ISLAND_UV_EXECUTABLE)
    add_custom_target(
      ei-dist
      COMMAND "${CMAKE_COMMAND}" -E make_directory "${CMAKE_BINARY_DIR}/dist"
      COMMAND
        "${ISLAND_UV_EXECUTABLE}" build --out-dir "${CMAKE_BINARY_DIR}/dist"
        "${CMAKE_SOURCE_DIR}"
      WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
      COMMENT "Build the EI wheel and source distribution"
      USES_TERMINAL
      VERBATIM
    )
  else()
    message(STATUS "uv was not found; the ei-dist target is unavailable")
  endif()
endif()

if(ISLAND_BUILD_TESTS)
  if(NOT ISLAND_UV_EXECUTABLE)
    message(FATAL_ERROR "ISLAND_BUILD_TESTS requires uv")
  endif()
  enable_testing()
  add_custom_target(
    check
    COMMAND "${ISLAND_UV_EXECUTABLE}" run pytest -q
    COMMAND "${ISLAND_UV_EXECUTABLE}" run ruff check .
    COMMAND "${ISLAND_UV_EXECUTABLE}" run pyright
    COMMAND
      "${CMAKE_COMMAND}" -E env
      "PYTHONPATH=${CMAKE_SOURCE_DIR}/src:${CMAKE_SOURCE_DIR}/libexec"
      "${ISLAND_UV_EXECUTABLE}" run lint-imports --cache-dir "${CMAKE_SOURCE_DIR}/../data/cache/tools/import-linter-ei"
    COMMAND "${CMAKE_CTEST_COMMAND}" --test-dir "${CMAKE_BINARY_DIR}" --output-on-failure
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
    COMMENT "Run the Island checkout verification gates"
    USES_TERMINAL
    VERBATIM
  )
endif()

# Product installation is owned by the repository-root install.sh.
