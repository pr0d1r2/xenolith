# The scan `scripts/guard/history-audit.sh` runs (`scripts/guard:V117`).
# Its own file rather than a quoted string inside the script: one language
# per file is this project's whole subject.
#
#   awk -v refs=N -f history-audit.awk DENYLIST CORPUS
#
# DENYLIST is `.private-names`: one pattern per line, `#` comments and
# blank lines skipped, matched as a fixed string ignoring case and as a
# WHOLE word (`scripts/guard:V23`, see `word`). CORPUS is
# what the script gathered: `ref` and `tag` section markers, then `git log
# -p` output, whose `commit <sha>` lines say which commit a hit belongs to.
#
# A hit is reported by the DENYLIST LINE NUMBER, a count and where it was
# found (a commit id, "a ref name", "an annotated tag") -- never by the
# text it matched, which would publish the name while reporting a leak.

# word(TEXT, NAME): 1 when NAME occurs in TEXT with no word character
# ([A-Za-z0-9_], the set `git grep -w` uses) directly before or after it.
# Every occurrence is tried, so "macme, then acme" matches on the second.
# A substring match found short names inside unrelated words (B3); both
# arguments are already lower case, and `index` keeps NAME a fixed string.
function word(text, name, from, at, before, after) {
  from = 1
  while ((at = index(substr(text, from), name)) > 0) {
    at += from - 1
    before = at > 1 ? substr(text, at - 1, 1) : ""
    after = substr(text, at + length(name), 1)
    if (before !~ /[A-Za-z0-9_]/ && after !~ /[A-Za-z0-9_]/) {
      return 1
    }
    from = at + 1
  }
  return 0
}

FNR == NR {
  sub(/\r$/, "")
  if ($0 ~ /^[ \t]*$/ || $0 ~ /^#/) {
    next
  }
  patterns++
  pattern[patterns] = tolower($0)
  source[patterns] = FNR
  next
}

/^ref / {
  where = "a ref name"
}

/^tag / {
  where = "an annotated tag"
}

# `-m` prints a merge once per parent as `commit <sha> (from <parent>)`;
# the commit is counted once.
/^commit [0-9a-f]+/ {
  where = substr($2, 1, 12)
  if (!($2 in seen)) {
    seen[$2] = 1
    commits++
  }
}

{
  text = tolower($0)
  for (i = 1; i <= patterns; i++) {
    if (!word(text, pattern[i])) {
      continue
    }
    hits[i]++
    total++
    if ((i, where) in found) {
      continue
    }
    found[i, where] = 1
    places[i]++
    if (places[i] <= 20) {
      list[i] = list[i] (list[i] == "" ? "" : ", ") where
    }
  }
}

END {
  for (i = 1; i <= patterns; i++) {
    if (!hits[i]) {
      continue
    }
    more = places[i] > 20 ? sprintf(" and %d more", places[i] - 20) : ""
    printf "history-audit: denylist line %d: %d matching line(s), in %s%s (V117)." \
      " The name is not repeated here on purpose.\n", \
      source[i], hits[i], list[i], more > "/dev/stderr"
  }
  printf "history-audit: read %d commits over %d ref(s); %d pattern(s), %d hits.\n", \
    commits, refs, patterns, total
  exit (total > 0)
}
