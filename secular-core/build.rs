// build.rs
fn main() {
    #[cfg(feature = "uniffi")]
    uniffi::generate_scaffolding("src/uniffi/secular.udl")
        .expect("UniFFI scaffolding generation failed");
}
