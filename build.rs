extern crate protoc_rust;

use std::fs;

fn main() {
    let output_proto_directory = "src/lib/data_transformer";
    fs::create_dir_all(output_proto_directory)
        .expect("Error on creating output directory");

    protoc_rust::run(protoc_rust::Args {
        out_dir: output_proto_directory,
        input: &["resources/proto/proto_api.proto"],
        includes: &["resources/proto"],
    }).expect("Error on protobuf generation");

    println!(
        "cargo:rustc-cfg={}",
        if cfg!(feature = "use-graph") {
            "use_graph"
        } else if cfg!(feature = "use-black-hole") {
            "use_black_hole"
        } else {
            "no_payload_handler"
        }
    );

    println!(
        "cargo:rustc-cfg={}",
        if cfg!(feature = "use-tcp") {
            "use_tcp"
        } else {
            "no_global_server"
        }
    );

    println!(
        "cargo:rustc-cfg={}",
        if cfg!(feature = "use-unix-socket") {
            "use_unix_socket"
        } else {
            "no_local_server"
        }
    );

    println!(
        "cargo:rustc-cfg={}",
        if cfg!(feature = "use-protobuf") {
            "use_protobuf"
        } else {
            "no_data_handler"
        }
    );
}
