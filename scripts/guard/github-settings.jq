# The comparison `scripts/guard/github-settings.sh` applies to one `gh api`
# reply (`scripts:V115`). Its own file rather than a quoted string inside
# the script: one language per file is this project's whole subject.
#
#   jq -r --arg section S -f github-settings.jq --args CHECK...
#
# `section` names the reply: `protection` (branches/main/protection) or
# `actions` (actions/permissions/workflow). The positional args are the
# status checks main must require. Emits one line per mismatch, nothing
# when the reply matches the intended settings.

def cite: " (scripts:V115).";

if $section == "protection" then
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
