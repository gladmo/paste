# This file controls Flutter-level build steps. It should not be edited.
cmake_minimum_required(VERSION 3.14)

set(EPHEMERAL_DIR "${CMAKE_CURRENT_SOURCE_DIR}/ephemeral")

# Configuration provided via flutter tool.
include(${EPHEMERAL_DIR}/generated_config.cmake)

# TODO: Determine the correct output directory for non-monolithic builds.
set(FLUTTER_LIBRARY "${EPHEMERAL_DIR}/flutter_windows.dll")

# List of asset files.
set(FLUTTER_ASSET_DIR_NAME "flutter_assets")

# === Flutter Library ===
# System-level dependencies.
find_package(PkgConfig)

add_library(flutter INTERFACE)
add_dependencies(flutter flutter_assemble)

# Enable the Windows target as a valid option.
target_compile_definitions(flutter INTERFACE
  FLUTTER_ENABLE_WINDOWS_SHELL_INTEGRATION
)

target_include_directories(flutter INTERFACE
  "${EPHEMERAL_DIR}"
)
target_link_libraries(flutter INTERFACE "${FLUTTER_LIBRARY}.lib")
add_custom_target(flutter_assemble DEPENDS
  "${FLUTTER_LIBRARY}"
  "${EPHEMERAL_DIR}/icudtl.dat"
)
