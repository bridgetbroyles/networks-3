fn main(){
 println!("cargo:rerun-if-env-changed=A3_CLAIM_CONFIG");
 let config=std::env::var("A3_CLAIM_CONFIG").expect("A3_CLAIM_CONFIG is required");
 std::fs::write(std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("config.rs"),config).unwrap();
}
