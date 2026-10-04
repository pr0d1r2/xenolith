# The comparison `scripts/guard/github-settings.sh` applies to one `gh api`
# reply (`scripts:V115`). Its own file rather than a quoted string inside
# the script: one language per file is this project's whole subject.
#
#   jq -r --arg section S -f github-settings.jq --args CHECK...
#
# `section` names the reply: `repo` (repos/{repo}), `protection`
# (branches/main/protection) or `actions` (actions/permissions/workflow).
# The positional args are the status checks main must require. Emits one
# line per mismatch, nothing when the reply matches the intended settings.

def cite: " (scripts:V115).";

if $section == "repo" then
  # Pull requests land by rebase merge only (`scripts/guard:B4`): a squash
  # collapses a RED and a GREEN commit into one, and a merge commit hides
  # them behind a second parent. GitHub omits these fields for a token
  # without admin rights, so a missing one is a finding, never a match.
  (if .allow_rebase_merge == true then empty
    else "github-settings: rebase merging is not allowed (or not readable);"
      + " pull requests land by rebase merge only" + cite end),
  (if .allow_squash_merge == false then empty
    else "github-settings: squash merging is allowed (or not readable); a"
      + " squash collapses RED and GREEN into one commit" + cite end),
  (if .allow_merge_commit == false then empty
    else "github-settings: merge commits are allowed (or not readable);"
      + " pull requests land by rebase merge only" + cite end)
elif $section == "protection" then
  (if .enforce_admins.enabled == true then empty
    else "github-settings: main's protection does not include administrators"
      + " (enforce_admins)" + cite end),
  # Both shapes GitHub has used: `checks[].context` and the older
  # `contexts[]`.
  (([.required_status_checks.checks[]?.context]
      + [.required_status_checks.contexts[]?]) as $have
    | $ARGS.positional[] | . as $want | select(any($have[]; . == $want) | not)
    | "github-settings: status check `\(.)` is not required on main" + cite)
elif $section == "actions" then
  if .can_approve_pull_request_reviews == false then empty
  else "github-settings: GitHub Actions may create and approve pull requests;"
    + " no bot here needs that" + cite end
else
  error("github-settings.jq: unknown section \($section)")
end
