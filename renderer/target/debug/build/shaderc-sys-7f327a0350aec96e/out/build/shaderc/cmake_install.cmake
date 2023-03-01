# Install script for directory: /home/doeringc/.cargo/registry/src/github.com-1ecc6299db9ec823/shaderc-sys-0.8.2/build/shaderc

# Set the install prefix
if(NOT DEFINED CMAKE_INSTALL_PREFIX)
  set(CMAKE_INSTALL_PREFIX "/home/doeringc/workspace/rust/vulkan-rt/renderer/target/debug/build/shaderc-sys-7f327a0350aec96e/out")
endif()
string(REGEX REPLACE "/$" "" CMAKE_INSTALL_PREFIX "${CMAKE_INSTALL_PREFIX}")

# Set the install configuration name.
if(NOT DEFINED CMAKE_INSTALL_CONFIG_NAME)
  if(BUILD_TYPE)
    string(REGEX REPLACE "^[^A-Za-z0-9_]+" ""
           CMAKE_INSTALL_CONFIG_NAME "${BUILD_TYPE}")
  else()
    set(CMAKE_INSTALL_CONFIG_NAME "Release")
  endif()
  message(STATUS "Install configuration: \"${CMAKE_INSTALL_CONFIG_NAME}\"")
endif()

# Set the component getting installed.
if(NOT CMAKE_INSTALL_COMPONENT)
  if(COMPONENT)
    message(STATUS "Install component: \"${COMPONENT}\"")
    set(CMAKE_INSTALL_COMPONENT "${COMPONENT}")
  else()
    set(CMAKE_INSTALL_COMPONENT)
  endif()
endif()

# Install shared libraries without execute permission?
if(NOT DEFINED CMAKE_INSTALL_SO_NO_EXE)
  set(CMAKE_INSTALL_SO_NO_EXE "0")
endif()

# Is this installation the result of a crosscompile?
if(NOT DEFINED CMAKE_CROSSCOMPILING)
  set(CMAKE_CROSSCOMPILING "FALSE")
endif()

# Set default install directory permissions.
if(NOT DEFINED CMAKE_OBJDUMP)
  set(CMAKE_OBJDUMP "/usr/bin/objdump")
endif()

if(NOT CMAKE_INSTALL_LOCAL_ONLY)
  # Include the install script for the subdirectory.
  include("/home/doeringc/workspace/rust/vulkan-rt/renderer/target/debug/build/shaderc-sys-7f327a0350aec96e/out/build/shaderc/third_party/cmake_install.cmake")
endif()

if(NOT CMAKE_INSTALL_LOCAL_ONLY)
  # Include the install script for the subdirectory.
  include("/home/doeringc/workspace/rust/vulkan-rt/renderer/target/debug/build/shaderc-sys-7f327a0350aec96e/out/build/shaderc/libshaderc_util/cmake_install.cmake")
endif()

if(NOT CMAKE_INSTALL_LOCAL_ONLY)
  # Include the install script for the subdirectory.
  include("/home/doeringc/workspace/rust/vulkan-rt/renderer/target/debug/build/shaderc-sys-7f327a0350aec96e/out/build/shaderc/libshaderc/cmake_install.cmake")
endif()

if(NOT CMAKE_INSTALL_LOCAL_ONLY)
  # Include the install script for the subdirectory.
  include("/home/doeringc/workspace/rust/vulkan-rt/renderer/target/debug/build/shaderc-sys-7f327a0350aec96e/out/build/shaderc/glslc/cmake_install.cmake")
endif()

if(NOT CMAKE_INSTALL_LOCAL_ONLY)
  # Include the install script for the subdirectory.
  include("/home/doeringc/workspace/rust/vulkan-rt/renderer/target/debug/build/shaderc-sys-7f327a0350aec96e/out/build/shaderc/examples/cmake_install.cmake")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "Unspecified" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/pkgconfig" TYPE FILE FILES "/home/doeringc/workspace/rust/vulkan-rt/renderer/target/debug/build/shaderc-sys-7f327a0350aec96e/out/build/shaderc/shaderc.pc")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "Unspecified" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/pkgconfig" TYPE FILE FILES "/home/doeringc/workspace/rust/vulkan-rt/renderer/target/debug/build/shaderc-sys-7f327a0350aec96e/out/build/shaderc/shaderc_static.pc")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "Unspecified" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/pkgconfig" TYPE FILE FILES "/home/doeringc/workspace/rust/vulkan-rt/renderer/target/debug/build/shaderc-sys-7f327a0350aec96e/out/build/shaderc/shaderc_combined.pc")
endif()

