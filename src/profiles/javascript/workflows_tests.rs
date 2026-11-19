use super::*;

#[test]
fn collects_inline_and_block_run_commands() {
    let text = "jobs:\n  build:\n    steps:\n      - uses: actions/checkout@v4\n      - run: node tools/inline.mjs\n      - name: multi\n        run: |\n          npm ci\n          node tools/block.mjs --flag\n        env:\n          X: tools/not-a-command.mjs\n";

    assert_eq!(
        run_lines(text),
        [
            "node tools/inline.mjs",
            "npm ci",
            "node tools/block.mjs --flag"
        ]
    );
}
