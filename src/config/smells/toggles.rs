//! Per-rule on/off state: an optional flat toggle knob plus the `disabled` list.

use super::knobs::Smells;
use crate::report::SmellKind;

macro_rules! toggles {
    ($($kind:ident => $field:ident),* $(,)?) => {
        impl Smells {
            fn toggle(&self, kind: SmellKind) -> Option<bool> {
                match kind {
                    $(SmellKind::$kind => Some(self.$field),)*
                    _ => None,
                }
            }

            fn toggle_mut(&mut self, kind: SmellKind) -> Option<&mut bool> {
                match kind {
                    $(SmellKind::$kind => Some(&mut self.$field),)*
                    _ => None,
                }
            }
        }

        /// The flat `[smells]` key that switches `kind` on or off.
        pub(super) fn toggle_key(kind: SmellKind) -> Option<&'static str> {
            match kind {
                $(SmellKind::$kind => Some(stringify!($field)),)*
                _ => None,
            }
        }
    };
}

toggles! {
    MagicNumbers => magic_numbers,
    NarratingCode => narrating_code,
    HighComplexity => cyclomatic_complexity,
    LongFunction => long_function,
    LargeClass => large_class,
    LongParameterList => long_parameter_list,
    TooManyReturns => too_many_returns,
    SplitVariable => split_variable,
    LooseTyping => loose_typing,
    MagicStringDefault => magic_string_default,
    TuplePacking => tuple_packing,
    MutatedParameter => param_mutation,
    ReassignedParameter => param_reassignment,
    LiteralMembership => literal_membership,
}

impl Smells {
    /// Whether `kind` runs: its toggle (when it has one) is on and it is not in `disabled`.
    pub fn is_enabled(&self, kind: SmellKind) -> bool {
        self.toggle(kind).unwrap_or(true) && !self.disabled.contains(&kind)
    }

    /// Switch `kind` on or off in both its toggle and the `disabled` list.
    pub(super) fn set_enabled(&mut self, kind: SmellKind, enabled: bool) {
        if let Some(toggle) = self.toggle_mut(kind) {
            *toggle = enabled;
        }
        if enabled {
            self.disabled.retain(|disabled| *disabled != kind);
        } else if !self.disabled.contains(&kind) {
            self.disabled.push(kind);
        }
    }
}
