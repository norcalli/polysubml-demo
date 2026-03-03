fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let src = format!("{}/src", manifest_dir);
    println!("cargo::rerun-if-changed={}/grammar.lalrpop", src);
    lalrpop::Configuration::new()
        .set_in_dir(&src)
        .set_out_dir(&src)
        .process()
        .unwrap();
}
