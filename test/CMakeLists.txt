cmake_minimum_required (VERSION 3.19)

project(${NAME}_tests)

# Avoid warning about DOWNLOAD_EXTRACT_TIMESTAMP in CMake 3.24:
if (CMAKE_VERSION VERSION_GREATER_EQUAL "3.24.0")
    cmake_policy(SET CMP0135 NEW)
endif()


##
# External Dependencies
##

include(FetchContent)

FetchContent_Declare(
        googletest
        GIT_REPOSITORY https://github.com/google/googletest.git
        GIT_SHALLOW TRUE
        GIT_PROGRESS TRUE
        GIT_TAG main
)
# For VS/Windows: Prevent overriding the parent project's compiler/linker settings
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)

FetchContent_MakeAvailable(googletest)

# Explicitly list the test source files
set(TEST_SOURCES
        BMAI3Tests.cpp
        LegacyFunctions.cpp
        PlayerTest.cpp
        SkillTest.cpp
        testutils.h
        DemoTest.cpp
)
add_executable(${NAME}_tests ${TEST_SOURCES})
target_link_libraries(${NAME}_tests PRIVATE ${NAME}_lib gtest_main)

# Copy GoogleTest DLLs to the build directory (if using shared libraries)
if(WIN32 AND BUILD_SHARED_LIBS)
    add_custom_command(TARGET ${NAME}_tests POST_BUILD
            COMMAND ${CMAKE_COMMAND} -E copy_if_different
            $<TARGET_FILE:gtest>
            $<TARGET_FILE_DIR:${NAME}_tests>)
    add_custom_command(TARGET ${NAME}_tests POST_BUILD
            COMMAND ${CMAKE_COMMAND} -E copy_if_different
            $<TARGET_FILE:gtest_main>
            $<TARGET_FILE_DIR:${NAME}_tests>)
endif()

##
# Auto-Discover Tests
##

include(GoogleTest)
gtest_discover_tests(${NAME}_tests)
