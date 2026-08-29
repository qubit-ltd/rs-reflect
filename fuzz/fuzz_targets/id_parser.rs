#![no_main]

use libfuzzer_sys::fuzz_target;
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::identity::ExternalTraitId;

// Exercises public identifier parsing with arbitrary UTF-8 input.
fuzz_target!(|data: String| {
    let _ = CapabilityId::new(&data);
    let _ = ExternalTraitId::new(&data);
});
