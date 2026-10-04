#!/usr/bin/env bats

@test "hk hooks use member comments accepted by the Pkl evaluator" {
  hk_file="${BATS_TEST_DIRNAME}/../../hk.pkl"
  hooks_body="$(sed -n '/^hooks {/,$p' "$hk_file")"

  ! grep -q '^  ///' <<<"$hooks_body"
}
