// Required to emit the N-API linker flags for the `cdylib` crate.
extern crate napi_build;

fn main() {
    napi_build::setup();
}
