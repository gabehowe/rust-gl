# CMAKE generated file: DO NOT EDIT!
# Generated by "Unix Makefiles" Generator, CMake Version 3.28

# Delete rule output on recipe failure.
.DELETE_ON_ERROR:

#=============================================================================
# Special targets provided by cmake.

# Disable implicit rules so canonical targets will work.
.SUFFIXES:

# Disable VCS-based implicit rules.
% : %,v

# Disable VCS-based implicit rules.
% : RCS/%

# Disable VCS-based implicit rules.
% : RCS/%,v

# Disable VCS-based implicit rules.
% : SCCS/s.%

# Disable VCS-based implicit rules.
% : s.%

.SUFFIXES: .hpux_make_needs_suffix_list

# Command-line flag to silence nested $(MAKE).
$(VERBOSE)MAKESILENT = -s

#Suppress display of executed commands.
$(VERBOSE).SILENT:

# A target that is always out of date.
cmake_force:
.PHONY : cmake_force

#=============================================================================
# Set environment variables for the build.

# The shell in which to execute make rules.
SHELL = /bin/sh

# The CMake executable.
CMAKE_COMMAND = /usr/bin/cmake

# The command to remove a file.
RM = /usr/bin/cmake -E rm -f

# Escaping for special characters.
EQUALS = =

# The top-level source directory on which CMake was run.
CMAKE_SOURCE_DIR = /home/gabri/.cargo/registry/src/index.crates.io-6f17d22bba15001f/glfw-sys-5.0.0+3.3.9/glfw

# The top-level build directory on which CMake was run.
CMAKE_BINARY_DIR = /home/gabri/dev/projects/5c19b154-4adf-492c-b7d8-cc5da2b1fbc3/target/debug/build/glfw-sys-1400c0cbb2edcdcd/out/build

# Utility rule file for uninstall.

# Include any custom commands dependencies for this target.
include CMakeFiles/uninstall.dir/compiler_depend.make

# Include the progress variables for this target.
include CMakeFiles/uninstall.dir/progress.make

CMakeFiles/uninstall:
	/usr/bin/cmake -P /home/gabri/dev/projects/5c19b154-4adf-492c-b7d8-cc5da2b1fbc3/target/debug/build/glfw-sys-1400c0cbb2edcdcd/out/build/cmake_uninstall.cmake

uninstall: CMakeFiles/uninstall
uninstall: CMakeFiles/uninstall.dir/build.make
.PHONY : uninstall

# Rule to build all files generated by this target.
CMakeFiles/uninstall.dir/build: uninstall
.PHONY : CMakeFiles/uninstall.dir/build

CMakeFiles/uninstall.dir/clean:
	$(CMAKE_COMMAND) -P CMakeFiles/uninstall.dir/cmake_clean.cmake
.PHONY : CMakeFiles/uninstall.dir/clean

CMakeFiles/uninstall.dir/depend:
	cd /home/gabri/dev/projects/5c19b154-4adf-492c-b7d8-cc5da2b1fbc3/target/debug/build/glfw-sys-1400c0cbb2edcdcd/out/build && $(CMAKE_COMMAND) -E cmake_depends "Unix Makefiles" /home/gabri/.cargo/registry/src/index.crates.io-6f17d22bba15001f/glfw-sys-5.0.0+3.3.9/glfw /home/gabri/.cargo/registry/src/index.crates.io-6f17d22bba15001f/glfw-sys-5.0.0+3.3.9/glfw /home/gabri/dev/projects/5c19b154-4adf-492c-b7d8-cc5da2b1fbc3/target/debug/build/glfw-sys-1400c0cbb2edcdcd/out/build /home/gabri/dev/projects/5c19b154-4adf-492c-b7d8-cc5da2b1fbc3/target/debug/build/glfw-sys-1400c0cbb2edcdcd/out/build /home/gabri/dev/projects/5c19b154-4adf-492c-b7d8-cc5da2b1fbc3/target/debug/build/glfw-sys-1400c0cbb2edcdcd/out/build/CMakeFiles/uninstall.dir/DependInfo.cmake "--color=$(COLOR)"
.PHONY : CMakeFiles/uninstall.dir/depend

