// build.rs
fn main() {
    #[cfg(feature = "uniffi")]
    {
        let udl_path = "src/uniffi/secular.udl";
        if std::path::Path::new(udl_path).exists() {
            uniffi::generate_scaffolding(udl_path)
                .expect("UniFFI scaffolding generation failed");
        }
    }
}
