# CMake generated Testfile for 
# Source directory: /home/doeringc/.cargo/registry/src/github.com-1ecc6299db9ec823/shaderc-sys-0.8.2/build/glslang
# Build directory: /home/doeringc/workspace/rust/vulkan-rt/renderer/target/debug/build/shaderc-sys-7f327a0350aec96e/out/build/glslang
# 
# This file includes the relevant testing commands required for 
# testing this directory and lists subdirectories to be tested as well.
add_test(glslang-testsuite "bash" "runtests" "/home/doeringc/workspace/rust/vulkan-rt/renderer/target/debug/build/shaderc-sys-7f327a0350aec96e/out/build/glslang/localResults" "/home/doeringc/workspace/rust/vulkan-rt/renderer/target/debug/build/shaderc-sys-7f327a0350aec96e/out/build/glslang/StandAlone/glslangValidator" "/home/doeringc/workspace/rust/vulkan-rt/renderer/target/debug/build/shaderc-sys-7f327a0350aec96e/out/build/glslang/StandAlone/spirv-remap")
set_tests_properties(glslang-testsuite PROPERTIES  WORKING_DIRECTORY "/home/doeringc/.cargo/registry/src/github.com-1ecc6299db9ec823/shaderc-sys-0.8.2/build/glslang/Test/" _BACKTRACE_TRIPLES "/home/doeringc/.cargo/registry/src/github.com-1ecc6299db9ec823/shaderc-sys-0.8.2/build/glslang/CMakeLists.txt;367;add_test;/home/doeringc/.cargo/registry/src/github.com-1ecc6299db9ec823/shaderc-sys-0.8.2/build/glslang/CMakeLists.txt;0;")
subdirs("External")
subdirs("glslang")
subdirs("OGLCompilersDLL")
subdirs("SPIRV")
subdirs("hlsl")
subdirs("gtests")
