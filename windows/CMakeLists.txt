cmake_minimum_required(VERSION 3.14)
project(paste LANGUAGES CXX)

# The name of the executable created for the application.
set(BINARY_NAME "paste")

cmake_policy(SET CMP0063 NEW)

set(CMAKE_INSTALL_PREFIX "${CMAKE_BINARY_DIR}/run" CACHE PATH "..." FORCE)

if(CMAKE_INSTALL_PREFIX_INITIALIZED_TO_DEFAULT)
  set(CMAKE_INSTALL_PREFIX "${CMAKE_BINARY_DIR}/run" CACHE PATH "..." FORCE)
endif()

set(BUILD_BUNDLE_DIR "$<TARGET_FILE_DIR:${BINARY_NAME}>")

# Set fallback configurations.
if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
  set(CMAKE_BUILD_TYPE "Debug" CACHE STRING "Flutter build mode" FORCE)
  set_property(CACHE CMAKE_BUILD_TYPE PROPERTY STRINGS "Debug" "Profile" "Release")
endif()

# Compilation settings that should be applied to most targets.
#
# Be cautious about adding new options here, as plugins use this function by
# default. In most cases, you should add new options to specific targets instead
# of all targets.
function(apply_standard_settings TARGET)
  target_compile_options(${TARGET} PRIVATE /W4 /WX /wd"4100" /wd"4305" /wd"4996")
  target_compile_options(${TARGET} PRIVATE /EHsc)
  target_compile_definitions(${TARGET} PRIVATE "_HAS_EXCEPTIONS=0")
  target_compile_definitions(${TARGET} PRIVATE "$<$<CONFIG:Debug>:_DEBUG>")
endfunction()

# Flutter library and tool build rules.
set(FLUTTER_MANAGED_DIR "${CMAKE_CURRENT_SOURCE_DIR}/flutter")
add_subdirectory(${FLUTTER_MANAGED_DIR})

# Application build; see runner/CMakeLists.txt.
add_subdirectory("runner")

# Generated plugin build rules, which manage building the plugins and adding
# them to the application.
include(flutter/generated_plugins.cmake)

# === Installation ===
# Support files are copied into place next to the executable, so that it can
# run in place.
install(CODE "
  file(REMOVE_RECURSE \"${CMAKE_INSTALL_PREFIX}/data\")
  file(COPY \"${BUILD_BUNDLE_DIR}/data\" DESTINATION \"${CMAKE_INSTALL_PREFIX}\")
  file(COPY \"${BUILD_BUNDLE_DIR}/\" DESTINATION \"${CMAKE_INSTALL_PREFIX}\"
    FILES_MATCHING PATTERN \"*.dll\"
    PATTERN \"data\" EXCLUDE)
  " COMPONENT Runtime)
