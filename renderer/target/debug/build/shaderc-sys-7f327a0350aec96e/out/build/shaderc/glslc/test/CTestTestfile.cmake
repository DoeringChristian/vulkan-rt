# CMake generated Testfile for 
# Source directory: /home/doeringc/.cargo/registry/src/github.com-1ecc6299db9ec823/shaderc-sys-0.8.2/build/shaderc/glslc/test
# Build directory: /home/doeringc/workspace/rust/vulkan-rt/renderer/target/debug/build/shaderc-sys-7f327a0350aec96e/out/build/shaderc/glslc/test
# 
# This file includes the relevant testing commands required for 
# testing this directory and lists subdirectories to be tested as well.
add_test(shaderc_expect_unittests "/usr/bin/python3" "-m" "unittest" "expect_unittest.py")
set_tests_properties(shaderc_expect_unittests PROPERTIES  WORKING_DIRECTORY "/home/doeringc/.cargo/registry/src/github.com-1ecc6299db9ec823/shaderc-sys-0.8.2/build/shaderc/glslc/test" _BACKTRACE_TRIPLES "/home/doeringc/.cargo/registry/src/github.com-1ecc6299db9ec823/shaderc-sys-0.8.2/build/shaderc/glslc/test/CMakeLists.txt;15;add_test;/home/doeringc/.cargo/registry/src/github.com-1ecc6299db9ec823/shaderc-sys-0.8.2/build/shaderc/glslc/test/CMakeLists.txt;0;")
add_test(shaderc_glslc_test_framework_unittests "/usr/bin/python3" "-m" "unittest" "glslc_test_framework_unittest.py")
set_tests_properties(shaderc_glslc_test_framework_unittests PROPERTIES  WORKING_DIRECTORY "/home/doeringc/.cargo/registry/src/github.com-1ecc6299db9ec823/shaderc-sys-0.8.2/build/shaderc/glslc/test" _BACKTRACE_TRIPLES "/home/doeringc/.cargo/registry/src/github.com-1ecc6299db9ec823/shaderc-sys-0.8.2/build/shaderc/glslc/test/CMakeLists.txt;18;add_test;/home/doeringc/.cargo/registry/src/github.com-1ecc6299db9ec823/shaderc-sys-0.8.2/build/shaderc/glslc/test/CMakeLists.txt;0;")
