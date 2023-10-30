fn main() {
    #[cfg(feature = "serialize-protobuf")]
    {
        println!("cargo:rerun-if-changed=src/serializer/data_producer.proto");
        prost_build::compile_protos(&["data_producer.proto"], &["src/serializer"]).unwrap();
    }
}
