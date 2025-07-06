// build.rs - Build script for gRPC proto compilation
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile GhostChain gRPC protocol definitions
    tonic_build::configure()
        .build_server(false)  // We're only a client
        .compile(
            &[
                "proto/ghostchain/blockchain.proto",
                "proto/ghostchain/transaction.proto", 
                "proto/ghostchain/network.proto",
            ],
            &["proto/ghostchain"],
        )?;

    // Set up to rebuild if proto files change
    println!("cargo:rerun-if-changed=proto/ghostchain/");
    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}
