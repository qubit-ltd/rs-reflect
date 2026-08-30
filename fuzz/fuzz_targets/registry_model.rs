#![no_main]

use libfuzzer_sys::fuzz_target;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::identity::ExternalTraitId;

// Exercises public registry-adjacent identity and descriptor queries without
// fabricating pointers or lifetimes.
fuzz_target!(|data: String| {
    let _ = CapabilityId::new(&data);
    let _ = ExternalTraitId::new(&data);
    let descriptor = TypeDescriptor::of::<Option<String>>();
    let _ = descriptor.type_id();
});
