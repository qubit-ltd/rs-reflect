//! Terminal-user fixture that depends only on downstream facade packages.

use model_facade_derive::model_reflect;

#[model_reflect]
struct User {
    id: u64,
}

#[cfg(test)]
mod tests {
    use model_facade_runtime::Reflect;

    use super::User;

    #[test]
    fn facade_attribute_delegates_to_reflection_runtime() {
        let descriptor = User::type_descriptor();
        assert_eq!(descriptor.query_name(), "User");
        assert_eq!(descriptor.fields().len(), 1);
        assert_eq!(descriptor.fields()[0].query_name(), Some("id"));
    }
}
