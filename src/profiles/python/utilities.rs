//! Hand-written Python helpers that the standard library or common packages already provide.

use crate::profiles::dependencies::{shape, KnownUtility, Provider, ProviderSource};

const fn stdlib(module: &'static str, function: &'static str) -> Provider {
    Provider {
        source: ProviderSource::StandardLibrary { module },
        function,
    }
}

const fn package(name: &'static str, module: &'static str, function: &'static str) -> Provider {
    Provider {
        source: ProviderSource::Package { name, module },
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

const MORE_ITERTOOLS: &str = "more-itertools";

#[rustfmt::skip]
pub static UTILITIES: &[KnownUtility] = &[
    utility(&["chunk", "chunks", "chunked", "batched"], (2, 2, 8), &[stdlib("itertools", "batched"), package(MORE_ITERTOOLS, "more_itertools", "chunked")]),
    utility(&["flatten"], (1, 1, 5), &[stdlib("itertools", "chain.from_iterable"), package(MORE_ITERTOOLS, "more_itertools", "flatten")]),
    utility(&["unique", "uniq", "dedupe", "deduplicate"], (1, 1, 6), &[stdlib("builtins", "dict.fromkeys"), package(MORE_ITERTOOLS, "more_itertools", "unique_everseen")]),
    utility(&["pairwise"], (1, 1, 6), &[stdlib("itertools", "pairwise")]),
    utility(&["take"], (2, 2, 3), &[stdlib("itertools", "islice"), package(MORE_ITERTOOLS, "more_itertools", "take")]),
    utility(&["partition"], (2, 2, 10), &[package(MORE_ITERTOOLS, "more_itertools", "partition"), package("toolz", "toolz", "partition")]),
    utility(&["sliding_window", "windowed"], (2, 2, 8), &[package(MORE_ITERTOOLS, "more_itertools", "windowed"), package("toolz", "toolz", "sliding_window")]),
    utility(&["group_by", "groupby"], (2, 2, 10), &[package("toolz", "toolz", "groupby"), package(MORE_ITERTOOLS, "more_itertools", "map_reduce")]),
    utility(&["memoize"], (1, 1, 12), &[stdlib("functools", "cache")]),
    utility(&["truncate"], (2, 3, 6), &[stdlib("textwrap", "shorten")]),
    utility(&["mean", "average"], (1, 1, 5), &[stdlib("statistics", "mean")]),
    utility(&["median"], (1, 1, 8), &[stdlib("statistics", "median")]),
    utility(&["clamp"], (3, 3, 5), &[package("numpy", "numpy", "clip")]),
    utility(&["retry", "with_retry", "with_retries"], (1, 3, 12), &[package("tenacity", "tenacity", "retry"), package("backoff", "backoff", "on_exception")]),
    utility(&["slugify"], (1, 1, 10), &[package("python-slugify", "slugify", "slugify"), package("django", "django.utils.text", "slugify")]),
];
