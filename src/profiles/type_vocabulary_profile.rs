use super::typevocab;
use crate::spine::ir::MutatorFamily;

/// Language-owned interpretation of type annotations used by smell detectors.
pub trait TypeVocabularyProfile: Send + Sync {
    fn loose_kind(&self, annotation: &str) -> Option<typevocab::LooseTypeKind>;
    fn is_bool(&self, annotation: &str) -> bool;
    fn is_dictish(&self, annotation: &str) -> bool;
    /// A dict-ish annotation whose keys are typed or declared open, so literal keys are not a hidden schema.
    fn is_open_map(&self, _annotation: &str) -> bool {
        false
    }
    fn has_domain_model(&self, annotation: &str) -> bool;
    fn is_primitive_scalar_alias(&self, annotation: &str) -> bool;
    /// Param a type-guard or assertion return annotation narrows (`p is T`).
    fn guarded_param<'a>(&self, _return_annotation: &'a str) -> Option<&'a str> {
        None
    }
    /// Annotation that already restricts values to a closed set of string literals.
    fn is_literal_union(&self, _annotation: &str) -> bool {
        false
    }
    /// Whether a value of this annotation is mutated by a `family` method call.
    fn is_mutable_collection(&self, annotation: &str, family: MutatorFamily) -> bool;
    fn is_void(&self, annotation: &str) -> bool;
    /// The annotation admits a missing value (`T | null`, `Optional[T]`).
    fn admits_missing(&self, _annotation: &str) -> bool {
        false
    }
    /// A named type (not an inline literal, union or intersection) that behaviour could move onto.
    fn is_nominal_type(&self, annotation: &str) -> bool {
        typevocab::is_nominal(annotation, &[])
    }
}
