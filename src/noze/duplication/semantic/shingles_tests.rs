use super::*;

fn stream(lexemes: impl Iterator<Item = u64>) -> Vec<(StructuralToken, u64)> {
    lexemes
        .map(|lex| (StructuralToken::GenericIdentifier, lex))
        .collect()
}

#[test]
fn consistent_rename_scores_one() {
    let left = stream([1, 2, 3, 1, 4, 2, 5, 3].into_iter());
    let right = stream([9, 8, 7, 9, 6, 8, 5, 7].into_iter());
    assert_eq!(jaccard(&shingles(&left), &shingles(&right)), 1.0);
}

#[test]
fn early_insertion_only_disturbs_nearby_shingles() {
    let base: Vec<(StructuralToken, u64)> = (0..40)
        .map(|i| match i % 2 {
            0 => (StructuralToken::Call, i),
            _ => (StructuralToken::GenericIdentifier, i % 5),
        })
        .collect();
    let mut edited = base.clone();
    edited.insert(2, (StructuralToken::Call, 999));
    let score = jaccard(&shingles(&base), &shingles(&edited));
    assert!(score >= 0.5, "scored {score}");
}

#[test]
fn streams_shorter_than_a_shingle_score_zero() {
    let short = stream([1, 2].into_iter());
    assert_eq!(jaccard(&shingles(&short), &shingles(&short)), 0.0);
}

#[test]
fn words_ignore_case_and_punctuation() {
    assert_eq!(words("Sum the values."), words("sum, the VALUES"));
}
