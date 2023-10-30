fn main() {
    #[cfg(feature = "serialize-protobuf")]
    {
        println!("cargo:rerun-if-changed=src/serializer/venom_data_producer.proto");
        prost_build::compile_protos(&["venom_data_producer.proto"], &["src/serializer"]).unwrap();
    }
}
