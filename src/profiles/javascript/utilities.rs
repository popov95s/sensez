//! Hand-written JS/TS helpers that common utility packages or the platform already provide.

use crate::profiles::dependencies::{shape, KnownUtility, Provider, ProviderSource};

const fn npm(name: &'static str, function: &'static str) -> Provider {
    Provider {
        source: ProviderSource::Package { name, module: name },
        function,
    }
}

const fn builtin(module: &'static str, function: &'static str) -> Provider {
    Provider {
        source: ProviderSource::StandardLibrary { module },
        function,
    }
}

const fn utility(
    names: &'static [&'static str],
    (min_arity, max_arity, max_lines): (usize, usize, usize),
    providers: &'static [Provider],
) -> KnownUtility {
    KnownUtility {
        names,
        shape: shape(min_arity, max_arity, max_lines),
        providers,
    }
}

/// Utilities provided by the same-named function of every listed package.
macro_rules! same_name {
    ($function:literal: $($package:literal),+) => {
        &[$(npm($package, $function)),+]
    };
}

#[rustfmt::skip]
pub static UTILITIES: &[KnownUtility] = &[
    utility(&["sleep", "delay", "wait"], (1, 1, 3), &[npm("es-toolkit", "delay"), npm("radash", "sleep"), npm("delay", "delay")]),
    utility(&["clamp"], (3, 3, 5), same_name!("clamp": "es-toolkit", "remeda", "lodash-es", "lodash", "just-clamp")),
    utility(&["chunk", "chunks"], (2, 2, 10), &[npm("es-toolkit", "chunk"), npm("remeda", "chunk"), npm("radash", "cluster"), npm("lodash-es", "chunk"), npm("lodash", "chunk")]),
    utility(&["unique", "uniq", "dedupe"], (1, 1, 5), &[npm("es-toolkit", "uniq"), npm("remeda", "unique"), npm("radash", "unique"), npm("lodash-es", "uniq"), npm("lodash", "uniq"), npm("just-unique", "unique")]),
    utility(&["uniqueBy", "uniqBy", "dedupeBy"], (2, 2, 10), &[npm("es-toolkit", "uniqBy"), npm("remeda", "uniqueBy"), npm("radash", "unique"), npm("lodash-es", "uniqBy"), npm("lodash", "uniqBy")]),
    utility(&["groupBy"], (2, 2, 12), &[npm("es-toolkit", "groupBy"), npm("remeda", "groupBy"), npm("radash", "group"), npm("lodash-es", "groupBy"), npm("lodash", "groupBy"), npm("just-group-by", "groupBy")]),
    utility(&["keyBy", "indexBy"], (2, 2, 8), &[npm("es-toolkit", "keyBy"), npm("remeda", "indexBy"), npm("radash", "objectify"), npm("lodash-es", "keyBy"), npm("lodash", "keyBy")]),
    utility(&["partition"], (2, 2, 10), &[npm("es-toolkit", "partition"), npm("remeda", "partition"), npm("radash", "fork"), npm("lodash-es", "partition"), npm("lodash", "partition")]),
    utility(&["sum"], (1, 1, 3), same_name!("sum": "es-toolkit", "remeda", "radash", "lodash-es", "lodash")),
    utility(&["mean", "average"], (1, 1, 5), same_name!("mean": "es-toolkit", "remeda", "lodash-es", "lodash")),
    utility(&["range"], (1, 3, 6), same_name!("range": "es-toolkit", "remeda", "radash", "lodash-es", "lodash")),
    utility(&["shuffle"], (1, 1, 10), same_name!("shuffle": "es-toolkit", "remeda", "radash", "lodash-es", "lodash")),
    utility(&["difference"], (2, 2, 5), same_name!("difference": "es-toolkit", "remeda", "lodash-es", "lodash")),
    utility(&["intersection"], (2, 2, 5), same_name!("intersection": "es-toolkit", "remeda", "lodash-es", "lodash")),
    utility(&["flatten"], (1, 1, 5), &[builtin("Array.prototype", "flat")]),
    utility(&["mapValues"], (2, 2, 8), same_name!("mapValues": "es-toolkit", "remeda", "radash", "lodash-es", "lodash")),
    utility(&["omit"], (2, 2, 8), same_name!("omit": "es-toolkit", "remeda", "radash", "lodash-es", "lodash", "just-omit")),
    utility(&["debounce"], (2, 3, 12), same_name!("debounce": "es-toolkit", "radash", "lodash-es", "lodash", "just-debounce-it")),
    utility(&["throttle"], (2, 3, 12), same_name!("throttle": "es-toolkit", "radash", "lodash-es", "lodash", "just-throttle")),
    utility(&["once"], (1, 1, 10), same_name!("once": "es-toolkit", "remeda", "lodash-es", "lodash", "just-once")),
    utility(&["memoize"], (1, 2, 12), &[npm("es-toolkit", "memoize"), npm("radash", "memo"), npm("lodash-es", "memoize"), npm("lodash", "memoize"), npm("memoize-one", "memoizeOne")]),
    utility(&["retry", "withRetry", "withRetries"], (1, 3, 12), &[npm("p-retry", "pRetry"), npm("es-toolkit", "retry"), npm("radash", "retry"), npm("async-retry", "retry")]),
    utility(&["truncate"], (2, 3, 5), same_name!("truncate": "remeda", "lodash-es", "lodash", "just-truncate")),
    utility(&["capitalize"], (1, 1, 3), same_name!("capitalize": "es-toolkit", "remeda", "radash", "lodash-es", "lodash")),
    utility(&["camelCase", "toCamelCase"], (1, 1, 10), &[npm("es-toolkit", "camelCase"), npm("remeda", "toCamelCase"), npm("radash", "camel"), npm("lodash-es", "camelCase"), npm("lodash", "camelCase")]),
    utility(&["kebabCase", "toKebabCase"], (1, 1, 10), &[npm("es-toolkit", "kebabCase"), npm("remeda", "toKebabCase"), npm("radash", "dash"), npm("lodash-es", "kebabCase"), npm("lodash", "kebabCase")]),
    utility(&["snakeCase", "toSnakeCase"], (1, 1, 10), &[npm("es-toolkit", "snakeCase"), npm("remeda", "toSnakeCase"), npm("radash", "snake"), npm("lodash-es", "snakeCase"), npm("lodash", "snakeCase")]),
    utility(&["escapeRegExp", "escapeRegex", "escapeRegExpChars"], (1, 1, 3), same_name!("escapeRegExp": "es-toolkit", "lodash-es", "lodash")),
    utility(&["deepEqual", "isDeepEqual", "isEqual"], (2, 2, 12), &[npm("es-toolkit", "isEqual"), npm("remeda", "isDeepEqual"), npm("radash", "isEqual"), npm("lodash-es", "isEqual"), npm("lodash", "isEqual"), npm("fast-deep-equal", "equal"), npm("dequal", "dequal")]),
    utility(&["cloneDeep", "deepClone"], (1, 1, 12), &[builtin("globalThis", "structuredClone"), npm("es-toolkit", "cloneDeep"), npm("lodash-es", "cloneDeep"), npm("lodash", "cloneDeep")]),
    utility(&["slugify"], (1, 1, 10), &[npm("slugify", "slugify"), npm("@sindresorhus/slugify", "slugify"), npm("es-toolkit", "kebabCase")]),
    utility(&["noop"], (0, 0, 1), same_name!("noop": "es-toolkit", "lodash-es", "lodash")),
    utility(&["identity"], (1, 1, 3), same_name!("identity": "es-toolkit", "remeda", "lodash-es", "lodash")),
];
