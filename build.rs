extern crate protoc_rust;

use std::fs;

fn main() {
    let output_proto_directory = "src/data_transformer";
    fs::create_dir_all(output_proto_directory)
        .expect("Error on creating output directory");

    protoc_rust::run(protoc_rust::Args {
        out_dir: output_proto_directory,
        input: &["resources/proto/proto_api.proto"],
        includes: &["resources/proto"],
    })
    .expect("Error on protobuf generation");
}
