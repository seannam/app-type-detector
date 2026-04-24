// Required to emit the N-API linker flags for `cdylib` crates.
extern crate napi_build;

fn main() {
    napi_build::setup();
}
