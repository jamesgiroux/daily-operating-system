#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${STAKEHOLDER_LINT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
SRC_DIR="$ROOT_DIR/src-tauri/src"

if [[ ! -d "$SRC_DIR" ]]; then
  echo "stakeholder signal lint: missing source dir: $SRC_DIR" >&2
  exit 2
fi

is_allowlisted_writer_file() {
  case "$1" in
    src-tauri/src/db/accounts.rs) return 0 ;;
    src-tauri/src/db/core.rs) return 0 ;;
    src-tauri/src/db/entity_linking.rs) return 0 ;;
    src-tauri/src/db/mod_tests.rs) return 0 ;;
    src-tauri/src/db/people.rs) return 0 ;;
    src-tauri/src/db/projects.rs) return 0 ;;
  esac
  return 1
}

failures=0

while IFS= read -r -d '' file; do
  rel="${file#"$ROOT_DIR"/}"
  case "$rel" in
    # derived_state is the downstream cache/projection subscriber this lint protects.
    src-tauri/src/services/derived_state.rs) continue ;;
    # demo fixtures can seed stakeholder examples without participating in production cache semantics.
    src-tauri/src/demo.rs) continue ;;
    # migrations perform one-time schema/data shaping before service-layer writers are available.
    src-tauri/src/migrations.rs) continue ;;
    # devtools are operator/debug utilities, not production service-layer mutation paths.
    src-tauri/src/devtools/*) continue ;;
  esac

  allowlisted_writer=0
  if is_allowlisted_writer_file "$rel"; then
    allowlisted_writer=1
  fi

  helper_file=0
  if [[ "$rel" == "src-tauri/src/services/stakeholder_writer.rs" ]]; then
    helper_file=1
  fi

  awk -v file="$rel" -v allowlisted_writer="$allowlisted_writer" -v helper_file="$helper_file" '
    function min(a, b) { return a < b ? a : b }
    function max(a, b) { return a > b ? a : b }
    function scan_line(input,    i, ch, next_ch) {
      clean_line = ""
      syntax_line = ""
      for (i = 1; i <= length(input); i++) {
        ch = substr(input, i, 1)
        next_ch = substr(input, i + 1, 1)

        if (scan_block_comment) {
          if (ch == "*" && next_ch == "/") {
            scan_block_comment = 0
            i++
          }
          continue
        }

        if (scan_string) {
          clean_line = clean_line ch
          if (scan_escape) {
            scan_escape = 0
          } else if (ch == "\\") {
            scan_escape = 1
          } else if (ch == "\"") {
            scan_string = 0
          }
          continue
        }

        if (ch == "/" && next_ch == "/") {
          break
        }
        if (ch == "/" && next_ch == "*") {
          scan_block_comment = 1
          i++
          continue
        }
        if (ch == "\"") {
          scan_string = 1
          scan_escape = 0
          clean_line = clean_line ch
          continue
        }

        clean_line = clean_line ch
        syntax_line = syntax_line ch
      }
    }
    function count_char(text, needle,    i, count) {
      count = 0
      for (i = 1; i <= length(text); i++) {
        if (substr(text, i, 1) == needle) {
          count++
        }
      }
      return count
    }
    function is_function_decl(text) {
      return text ~ /(^|[^[:alnum:]_])((pub(\([^)]*\))?|async|unsafe|extern|const)[[:space:]]+)*fn[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*(<|\()/
    }
    function is_stakeholder_write_wrapper_call(line) {
      return syntax_lines[line] ~ /(^|[^[:alnum:]_:])((crate::services::)?stakeholder_writer::)?write_with_stakeholders_changed(_for_entities)?[[:space:]]*\(/
    }
    function is_stakeholder_emit_helper_call(line) {
      return syntax_lines[line] ~ /(^|[^[:alnum:]_:])((crate::services::)?stakeholder_writer::)?emit_stakeholders_changed(_for_entities)?[[:space:]]*\(/
    }
    function is_direct_actiondb_graph_method_call(line) {
      return syntax_lines[line] ~ /(^|[^[:alnum:]_])[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\.[[:space:]]*(link_person_to_entity|unlink_person_from_entity|merge_people|add_team_member|add_account_team_member)[[:space:]]*\(/ ||
        syntax_lines[line] ~ /(^|[^[:alnum:]_:])ActionDb::(link_person_to_entity|unlink_person_from_entity|merge_people|add_team_member|add_account_team_member)[[:space:]]*\(/
    }
    function call_span_stop(start,    j, depth) {
      depth = 0
      for (j = start; j <= n; j++) {
        depth += count_char(syntax_lines[j], "(")
        depth -= count_char(syntax_lines[j], ")")
        if (depth <= 0) {
          return j
        }
      }
      return n
    }
    function index_stakeholder_signal_calls(    j) {
      for (j = 1; j <= n; j++) {
        if (is_stakeholder_write_wrapper_call(j)) {
          write_wrapper_count++
          write_wrapper_start[write_wrapper_count] = j
          write_wrapper_end[write_wrapper_count] = call_span_stop(j)
        } else if (is_stakeholder_emit_helper_call(j)) {
          emit_helper_count++
          emit_helper_start[emit_helper_count] = j
          emit_helper_end[emit_helper_count] = call_span_stop(j)
        }
      }
    }
    function is_inside_stakeholder_write_wrapper(line,    k) {
      for (k = 1; k <= write_wrapper_count; k++) {
        if (write_wrapper_start[k] <= line && line <= write_wrapper_end[k]) {
          return 1
        }
      }
      return 0
    }
    function has_nearby_stakeholder_emit(line, fn_idx,    k, window_start, window_end) {
      if (fn_idx <= 0) {
        return 0
      }
      window_start = max(fn_start[fn_idx], line - 30)
      window_end = min(fn_end[fn_idx], line + 30)
      for (k = 1; k <= emit_helper_count; k++) {
        if ((window_start <= emit_helper_start[k] && emit_helper_start[k] <= window_end) ||
            (window_start <= emit_helper_end[k] && emit_helper_end[k] <= window_end)) {
          return 1
        }
      }
      return 0
    }
    function has_write_level_stakeholder_signal(line, fn_idx) {
      return is_inside_stakeholder_write_wrapper(line) ||
        has_nearby_stakeholder_emit(line, fn_idx)
    }
    function enclosing_function(line,    k, best) {
      best = 0
      for (k = 1; k <= fn_count; k++) {
        if (fn_start[k] <= line && line <= fn_end[k] &&
            (best == 0 || fn_start[k] > fn_start[best])) {
          best = k
        }
      }
      return best
    }
    function is_non_graph_update(i, stop,    j, text) {
      if (tolower(code_lines[i]) !~ /update[^\"]*account_stakeholders/) {
        return 0
      }
      text = ""
      for (j = i; j <= stop; j++) {
        text = text "\n" tolower(code_lines[j])
      }
      if (text !~ /(set|,)[[:space:]]*(account_id|person_id|status|relationship_type)[[:space:]]*=/) {
        return 1
      }
      return 0
    }
    function has_direct_stakeholder_emit(i, stop,    j, saw_emit, saw_signal) {
      saw_emit = 0
      saw_signal = 0
      for (j = i; j <= stop; j++) {
        if (code_lines[j] ~ /emit_in_transaction[[:space:]]*\(/) {
          saw_emit = 1
        }
        if (code_lines[j] ~ /STAKEHOLDERS_CHANGED_SIGNAL|stakeholders_changed/) {
          saw_signal = 1
        }
      }
      return saw_emit && saw_signal
    }
    {
      lines[NR] = $0
      scan_line($0)
      code_lines[NR] = clean_line
      syntax_lines[NR] = syntax_line
      in_cfg_test_line[NR] = test_stack_len > 0 || pending_cfg_test

      if (syntax_line ~ /^[[:space:]]*#\[[^]]*cfg[[:space:]]*\([[:space:]]*test[[:space:]]*\)/) {
        pending_cfg_test = 1
        in_cfg_test_line[NR] = 1
      }

      if (!pending_fn && is_function_decl(syntax_line)) {
        pending_fn = 1
        pending_fn_line = NR
      }

      opens = count_char(syntax_line, "{")
      closes = count_char(syntax_line, "}")
      if (pending_fn && opens > 0) {
        fn_count++
        fn_start[fn_count] = NR
        fn_decl[fn_count] = pending_fn_line
        fn_depth[fn_count] = brace_depth + 1
        stack_len++
        fn_stack[stack_len] = fn_count
        pending_fn = 0
      } else if (pending_fn && syntax_line ~ /;/) {
        pending_fn = 0
      }

      if (pending_cfg_test && syntax_line ~ /(^|[^[:alnum:]_])mod[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\{/ && opens > 0) {
        test_stack_len++
        test_depth[test_stack_len] = brace_depth + 1
        pending_cfg_test = 0
      } else if (pending_cfg_test && syntax_line !~ /^[[:space:]]*#\[/ && syntax_line !~ /^[[:space:]]*$/ && syntax_line !~ /(^|[^[:alnum:]_])mod[[:space:]]+/) {
        pending_cfg_test = 0
      }

      brace_depth += opens - closes
      while (stack_len > 0 && brace_depth < fn_depth[fn_stack[stack_len]]) {
        fn_end[fn_stack[stack_len]] = NR
        stack_len--
      }
      while (test_stack_len > 0 && brace_depth < test_depth[test_stack_len]) {
        test_stack_len--
      }
    }
    END {
      n = NR
      while (stack_len > 0) {
        fn_end[fn_stack[stack_len]] = n
        stack_len--
      }
      index_stakeholder_signal_calls()

      write_re = "(insert([[:space:]]+or[[:space:]]+(ignore|replace))?[[:space:]]+into|update|delete[[:space:]]+from)[^\"]*(account_stakeholders|entity_members)([^[:alnum:]_]|$)"
      for (i = 1; i <= n; i++) {
        line = tolower(code_lines[i])
        if (line !~ write_re) {
          continue
        }
        stop = min(n, i + 30)
        fn_idx = enclosing_function(i)
        if (allowlisted_writer || is_non_graph_update(i, stop) ||
            has_write_level_stakeholder_signal(i, fn_idx)) {
          continue
        }
        printf "%s:%d: stakeholder graph write must be covered at the write site by stakeholder_writer::write_with_stakeholders_changed/emit_stakeholders_changed or use an allowlisted DB writer\n", file, i
        failures++
      }

      for (i = 1; i <= n; i++) {
        if (!is_direct_actiondb_graph_method_call(i)) {
          continue
        }
        fn_idx = enclosing_function(i)
        if (in_cfg_test_line[i] || allowlisted_writer || helper_file ||
            has_write_level_stakeholder_signal(i, fn_idx)) {
          continue
        }
        printf "%s:%d: direct ActionDb stakeholder graph method call must be covered at the call site by stakeholder_writer::write_with_stakeholders_changed/emit_stakeholders_changed or use an allowlisted DB writer\n", file, i
        failures++
      }

      if (!helper_file) {
        for (i = 1; i <= n; i++) {
          if (code_lines[i] !~ /emit_in_transaction[[:space:]]*\(|STAKEHOLDERS_CHANGED_SIGNAL|stakeholders_changed/) {
            continue
          }
          stop = min(n, i + 12)
          if (has_direct_stakeholder_emit(i, stop)) {
            printf "%s:%d: direct stakeholders_changed emit must use services::stakeholder_writer helper\n", file, i
            failures++
          }
        }
      }

      if (failures > 0) {
        exit 1
      }
    }
  ' "$file" || failures=$((failures + 1))
done < <(find "$SRC_DIR" -type f -name '*.rs' -print0)

if [[ "$failures" -ne 0 ]]; then
  echo "stakeholder signal lint failed with $failures violation(s)" >&2
  exit 1
fi

echo "stakeholder signal lint passed"
