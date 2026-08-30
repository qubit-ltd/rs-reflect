#![no_main]

use libfuzzer_sys::fuzz_target;
use qubit_reflect::expression::ConcreteTypeExpression;
use qubit_reflect::expression::DiagnosticText;
use qubit_reflect::expression::GenericArgument;
use qubit_reflect::expression::TypeExpression;

// Exercises public structural type-expression construction with arbitrary text.
fuzz_target!(|data: String| {
    let expression = TypeExpression::Concrete(ConcreteTypeExpression {
        path: Box::new([data.clone().into_boxed_str()]),
        arguments: Box::new([GenericArgument::Type(TypeExpression::Parameter(data.into_boxed_str()))]),
        diagnostic: DiagnosticText::default(),
    });
    let _ = expression == expression.clone();
    let _ = format!("{expression:?}");
});
