#[cfg(feature = "use-protobuf")]
extern crate protoc_rust;
#[cfg(feature = "use-protobuf")]
use std::fs;

#[cfg(feature = "use-protobuf")]
fn main() {
    let output_proto_directory = "src/lib/data_transformer";
    fs::create_dir_all(output_proto_directory)
        .expect("Error on creating output directory");

    protoc_rust::run(protoc_rust::Args {
        out_dir: output_proto_directory,
        input: &["resources/proto/proto_api.proto"],
        includes: &["resources/proto"]
    }).expect("Error on protobuf generation");
}

