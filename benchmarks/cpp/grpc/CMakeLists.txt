cmake_minimum_required(VERSION 3.15)
project(grpc_server CXX)

set(CMAKE_CXX_STANDARD 17)
if(CMAKE_BUILD_TYPE STREQUAL "Release")
  set(CMAKE_INTERPROCEDURAL_OPTIMIZATION TRUE)
  add_compile_options(-O3 -march=native -DNDEBUG)
endif()

find_package(Protobuf REQUIRED)
find_package(gRPC REQUIRED)
find_package(absl REQUIRED)


# Proto file generation
# Note: In a real environment we might want to use built-in cmake protobuf support better, 
# but for simplicity in Docker we often just invoke protoc or rely on find_package vars.
# Here we assume generated files are handled or we generate them.
# For simplicity, we will run protoc in Dockerfile or custom command.
# Let's use the standard way if possible.

get_target_property(grpc_cpp_plugin gRPC::grpc_cpp_plugin LOCATION)

function(grpc_generate_cpp sr_srcs hdr_srcs)
  if(NOT ARGN)
    message(SEND_ERROR "Error: grpc_generate_cpp() called without any proto files")
    return()
  endif()

  if(PROTOBUF_GENERATE_CPP_APPEND_PATH)
    # Create an include path for each file specified
    foreach(fil ${ARGN})
      get_filename_component(abs_fil ${fil} ABSOLUTE)
      get_filename_component(abs_path ${abs_fil} PATH)
      list(FIND _protobuf_include_path ${abs_path} _contains_already)
      if(${_contains_already} EQUAL -1)
        list(APPEND _protobuf_include_path -I ${abs_path})
      endif()
    endforeach()
  else()
    set(_protobuf_include_path -I ${CMAKE_CURRENT_SOURCE_DIR})
  endif()

  if(DEFINED PROTOBUF_IMPORT_DIRS)
    foreach(dir ${PROTOBUF_IMPORT_DIRS})
      get_filename_component(abs_path ${dir} ABSOLUTE)
      list(FIND _protobuf_include_path ${abs_path} _contains_already)
      if(${_contains_already} EQUAL -1)
        list(APPEND _protobuf_include_path -I ${abs_path})
      endif()
    endforeach()
  endif()

  set(${sr_srcs})
  set(${hdr_srcs})
  foreach(fil ${ARGN})
    get_filename_component(abs_fil ${fil} ABSOLUTE)
    get_filename_component(fil_we ${fil} NAME_WE)
    get_filename_component(fil_path ${fil} PATH)
    
    # We will generate files in current binary dir
    set(_out_path ${CMAKE_CURRENT_BINARY_DIR})
    
    list(APPEND ${sr_srcs} "${_out_path}/${fil_we}.pb.cc")
    list(APPEND ${sr_srcs} "${_out_path}/${fil_we}.grpc.pb.cc")
    list(APPEND ${hdr_srcs} "${_out_path}/${fil_we}.pb.h")
    list(APPEND ${hdr_srcs} "${_out_path}/${fil_we}.grpc.pb.h")

    add_custom_command(
      OUTPUT "${_out_path}/${fil_we}.pb.cc"
             "${_out_path}/${fil_we}.pb.h"
             "${_out_path}/${fil_we}.grpc.pb.cc"
             "${_out_path}/${fil_we}.grpc.pb.h"
      COMMAND ${Protobuf_PROTOC_EXECUTABLE}
      ARGS --grpc_out=${_out_path}
           --cpp_out=${_out_path}
           --plugin=protoc-gen-grpc=${grpc_cpp_plugin}
           ${_protobuf_include_path}
           ${abs_fil}
      DEPENDS ${abs_fil}
      COMMENT "Running C++ gRPC protocol buffer compiler on ${fil}"
      VERBATIM)
  endforeach()

  set_source_files_properties(${${sr_srcs}} ${${hdr_srcs}} PROPERTIES GENERATED TRUE)
  set(${sr_srcs} ${${sr_srcs}} PARENT_SCOPE)
  set(${hdr_srcs} ${${hdr_srcs}} PARENT_SCOPE)
endfunction()

set(PROTO_FILES proto/analytics.proto)
grpc_generate_cpp(PROTO_SRCS PROTO_HDRS ${PROTO_FILES})

add_executable(server 
  src/main.cc 
  src/server_impl.cc 
  src/call_data.cc 
  ${PROTO_SRCS} 
  ${PROTO_HDRS}
)

target_link_libraries(server
  gRPC::grpc++
  gRPC::grpc++_reflection
  protobuf::libprotobuf
  absl::flat_hash_map
  -ljemalloc
)

target_include_directories(server PRIVATE 
  ${CMAKE_CURRENT_SOURCE_DIR}/src
  ${CMAKE_CURRENT_BINARY_DIR}
)
