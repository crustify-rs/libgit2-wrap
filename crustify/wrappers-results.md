# crustify — `libgit2 / src`

## Campaign

- **target repo** — `https://github.com/libgit2/libgit2` @ `0551dfd4ad989b6a3d5683c0d4cf326c6efef929`
- **target** — `src`
- **campaign objective** — `wrap`
- **`impl_files`** — `src/`
- **`api_headers`** — `include/`
- **agent backend** — `codex`
- **model** — `openai/gpt-5.6-sol`
- **`--billing`** — `api`
- **`--max-types`** — `2`
- **`--max-syms`** — `50`
- **`--max-loc`** — `1000`
- **`--min-fields`** — `10`
- **`--parallel-max`** — `8` (reduced to `4` and `2` for audit remediation)
- **branch** — `crustify/src-gpt-5.6-sol`, tip `456a0e8fc0`
- **deps** — crustify-cli `8d8c4dff28d8cfe27e65f31b3ff756211a08c465` (`scope-refactor`), ffibox `600399ff11504332db0905cadc397aaeb15ddc02` (`main`)

## Review pass

`--objective review`, LLM-as-a-Judge over the landed waves. The user has
reserved explicit approval for the campaign-end review, so this checkpoint
records no campaign-end review run.

- **agent backend** — `codex`
- **model** — `openai/gpt-5.6-sol` (user-directed self-review)
- **`--billing`** — `api`
- **`--max-types`** — `2`
- **`--max-syms`** — `50`
- **`--max-loc`** — `1000`
- **`--min-fields`** — `10`
- **`--parallel-max`** — `8`
- **branch** — `crustify/src-gpt-5.6-sol`, tip `not run — awaiting approval`
- **agents** — `0`, over `0` session(s)

`rv`-prefixed columns below are reserved for that review pass; `—` means it
has not been authorized or run.

## Legend

- `DAG layer` — the unit's own wrap DAG layer
- `kind` — `struct` / `union` / `enum` for a type; `callback`; `function` for every symbol
- `fields` — all declared fields
- `target fields` / `target ptr` — fields touched by target-section code / pointer subset
- `wrapped fields` — distinct `type.field` paths carrying a `/// Wraps:` accessor anchor; `—` means opaque
- `newtypes` — exact `/// Wraps: <tag>` type anchors in the Rust tree
- `target fns` — distinct target-section functions using the symbol
- `deps` — type/callback dependencies recorded by the oracle plan
- `wrappers` — distinct safe functions carrying a `/// Wraps: <symbol>` anchor
- `batch` — the agent batch; pooled rows use `↖ batched` for its shared accounting
- `$` / `wall` / `loc` — computed usage cost, agent duration, and Rust insertions in its landing commit
- `rv $` / `rv wall` / `rv loc` / `verdict` — campaign-end review accounting and judgement
- Batch `wall (longest)` is the slowest agent in that layer; `serial Σ` sums agent duration

## Raw lifetime discovery

Goal: turn untyped lifecycle primitives into Rust lifetime contracts before
wrappers consume them. The raw void and string passes each received their
already-authorized immediate review; campaign-end review is separate.

| tier | symbols submitted | strategies | CDropped | CCloned | CLenDropped | CLenCloned | $ | wall |
|---|---|---|---|---|---|---|---|---|
| void | `2` | `2` | `2` | `0` | `0` | `0` | `$5.56` | `9m30s` |
| string | `1` | `1` | `1` | `1` | `0` | `0` | `$4.29` | `7m06s` |
| **Σ** | **`3`** | **`3`** | **`3`** | **`1`** | **`0`** | **`0`** | **`$9.85`** | **`16m36s`** |

## Target set

Exact first-half topological prefix: `403 / 805` closure units (`115` types,
`12` callbacks, `276` symbols). The held second half contains `402` units
and remains unscheduled and untranslated.

### Types and callbacks

| DAG layer | unit | kind | fields | target fields | target ptr | wrapped fields | newtypes | $ | wall | loc | rv $ | rv wall | rv loc | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `0` | `git_annotated_commit` | struct | `9` | `9` | `6` | `—` | `1` | `$3.26` | `6m45s` | `168` | — | — | — | pending review |
| `0` | `git_apply_location_t` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_attr_value_t` | enum | `—` | `—` | `—` | `—` | `1` | `$2.37` | `4m36s` | `76` | — | — | — | pending review |
| `0` | `git_blame` | struct | `22` | `22` | `10` | `—` | `1` | `$5.04` | `9m37s` | `75` | — | — | — | pending review |
| `1` | `git_blame_options` | struct | `7` | `6` | `0` | `7` | `1` | `$5.67` | `12m13s` | `259` | — | — | — | pending review |
| `1` | `git_blob` | struct | `7` | `6` | `2` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_branch_iterator` | struct | `0` | `0` | `0` | `—` | `1` | `$3.97` | `7m37s` | `168` | — | — | — | pending review |
| `0` | `git_branch_t` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_buf` | struct | `3` | `3` | `1` | `3` | `1` | `$5.63` | `10m21s` | `372` | — | — | — | pending review |
| `1` | `git_cached_obj` | struct | `5` | `5` | `0` | `5` | `1` | `$5.47` | `8m44s` | `320` | — | — | — | pending review |
| `1` | `git_cert` | struct | `1` | `0` | `0` | `1` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_cert_t` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_checkout_notify_t` | enum | `—` | `—` | `—` | `—` | `1` | `$2.88` | `5m58s` | `113` | — | — | — | pending review |
| `0` | `git_checkout_perfdata` | struct | `3` | `3` | `0` | `3` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `1` | `git_checkout_perfdata_cb` | callback | `—` | `—` | `—` | `—` | `1` | `$7.52` | `16m49s` | `938` | — | — | — | pending review |
| `0` | `git_checkout_progress_cb` | callback | `—` | `—` | `—` | `—` | `1` | `$7.58` | `16m07s` | `811` | — | — | — | pending review |
| `0` | `git_clone_local_t` | enum | `—` | `—` | `—` | `—` | `1` | `$4.14` | `9m03s` | `131` | — | — | — | pending review |
| `1` | `git_commit` | struct | `13` | `13` | `8` | `—` | `1` | `$4.56` | `7m52s` | `107` | — | — | — | pending review |
| `0` | `git_config` | struct | `3` | `3` | `0` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `1` | `git_config_entry` | struct | `6` | `6` | `4` | `6` | `1` | `$3.86` | `8m42s` | `277` | — | — | — | pending review |
| `0` | `git_config_level_t` | enum | `—` | `—` | `—` | `—` | `1` | `$2.32` | `5m25s` | `223` | — | — | — | pending review |
| `1` | `git_credential` | struct | `2` | `2` | `1` | `2` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_credential_t` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_delta_t` | enum | `—` | `—` | `—` | `—` | `1` | `$3.22` | `6m36s` | `76` | — | — | — | pending review |
| `0` | `git_describe_format_options` | struct | `4` | `4` | `1` | `4` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_describe_options` | struct | `6` | `6` | `1` | `6` | `1` | `$2.81` | `6m04s` | `252` | — | — | — | pending review |
| `0` | `git_describe_result` | struct | `7` | `7` | `3` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_diff` | struct | `16` | `16` | `7` | `—` | `1` | `$4.37` | `8m43s` | `109` | — | — | — | pending review |
| `1` | `git_diff_binary_file` | struct | `4` | `4` | `1` | `4` | `1` | `$3.92` | `7m38s` | `407` | — | — | — | pending review |
| `0` | `git_diff_binary_t` | enum | `—` | `—` | `—` | `—` | `1` | `$2.99` | `6m29s` | `157` | — | — | — | pending review |
| `1` | `git_diff_file` | struct | `6` | `6` | `1` | `6` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_diff_format_t` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_diff_hunk` | struct | `6` | `6` | `0` | `6` | `1` | `$3.98` | `8m08s` | `323` | — | — | — | pending review |
| `0` | `git_diff_line` | struct | `7` | `7` | `1` | `7` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_diff_patchid_options` | struct | `1` | `1` | `0` | `1` | `1` | `$3.94` | `7m10s` | `138` | — | — | — | pending review |
| `0` | `git_diff_stats` | struct | `8` | `8` | `2` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_diff_stats_format_t` | enum | `—` | `—` | `—` | `—` | `1` | `$2.28` | `4m26s` | `208` | — | — | — | pending review |
| `0` | `git_direction` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_error` | struct | `2` | `2` | `1` | `2` | `1` | `$3.32` | `7m01s` | `166` | — | — | — | pending review |
| `0` | `git_fetch_prune_t` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_filemode_t` | enum | `—` | `—` | `—` | `—` | `1` | `$3.47` | `7m34s` | `149` | — | — | — | pending review |
| `1` | `git_index` | struct | `24` | `24` | `6` | `—` | `1` | `$3.98` | `6m27s` | `54` | — | — | — | pending review |
| `0` | `git_index_conflict_iterator` | struct | `2` | `2` | `1` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `1` | `git_index_entry` | struct | `12` | `12` | `1` | `12` | `1` | `$4.65` | `8m00s` | `302` | — | — | — | pending review |
| `0` | `git_index_matched_path_cb` | callback | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_index_time` | struct | `2` | `2` | `0` | `2` | `1` | `$1.91` | `4m11s` | `96` | — | — | — | pending review |
| `0` | `git_indexer` | struct | `30` | `30` | `4` | `—` | `1` | `$3.05` | `6m48s` | `74` | — | — | — | pending review |
| `0` | `git_indexer_progress` | struct | `7` | `7` | `0` | `7` | `1` | `$2.23` | `5m34s` | `214` | — | — | — | pending review |
| `1` | `git_indexer_progress_cb` | callback | `—` | `—` | `—` | `—` | `1` | `$12.63` | `24m47s` | `807` | — | — | — | pending review |
| `0` | `git_iterator` | struct | `18` | `18` | `9` | `—` | `1` | `$3.28` | `6m31s` | `67` | — | — | — | pending review |
| `0` | `git_mailmap` | struct | `1` | `1` | `0` | `—` | `1` | `$3.06` | `5m59s` | `230` | — | — | — | pending review |
| `0` | `git_merge_analysis_t` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_merge_file_favor_t` | enum | `—` | `—` | `—` | `—` | `1` | `$3.62` | `6m55s` | `255` | — | — | — | pending review |
| `0` | `git_merge_file_input` | struct | `5` | `5` | `2` | `5` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `1` | `git_merge_file_options` | struct | `7` | `7` | `3` | `7` | `1` | `$6.66` | `14m20s` | `541` | — | — | — | pending review |
| `0` | `git_merge_file_result` | struct | `5` | `5` | `2` | `5` | `1` | `$4.56` | `9m25s` | `336` | — | — | — | pending review |
| `0` | `git_merge_preference_t` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_message_trailer` | struct | `2` | `2` | `2` | `2` | `1` | `$2.62` | `5m58s` | `187` | — | — | — | pending review |
| `1` | `git_message_trailer_array` | struct | `3` | `3` | `2` | `3` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_note` | struct | `4` | `4` | `3` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_object` | struct | `2` | `2` | `1` | `—` | `1` | `$3.91` | `7m53s` | `183` | — | — | — | pending review |
| `0` | `git_object_t` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `1` | `git_odb` | struct | `7` | `7` | `1` | `—` | `1` | `$3.69` | `10m17s` | `62` | — | — | — | pending review |
| `1` | `git_odb_foreach_cb` | callback | `—` | `—` | `—` | `—` | `1` | `$6.41` | `19m35s` | `881` | — | — | — | pending review |
| `1` | `git_odb_stream` | struct | `10` | `10` | `6` | `10` | `1` | `$4.72` | `11m45s` | `463` | — | — | — | pending review |
| `1` | `git_odb_writepack` | struct | `4` | `4` | `4` | `4` | `1` | `$5.86` | `10m24s` | `428` | — | — | — | pending review |
| `0` | `git_oid` | struct | `2` | `2` | `0` | `2` | `1` | `$3.92` | `7m34s` | `174` | — | — | — | pending review |
| `0` | `git_oid_t` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `1` | `git_oidarray` | struct | `2` | `2` | `1` | `2` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_packbuilder` | struct | `30` | `30` | `6` | `—` | `1` | `$3.15` | `5m57s` | `93` | — | — | — | pending review |
| `0` | `git_packbuilder_foreach_cb` | callback | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_packbuilder_progress` | callback | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_patch` | struct | `17` | `17` | `5` | `—` | `1` | `$2.59` | `6m27s` | `66` | — | — | — | pending review |
| `0` | `git_pathspec` | struct | `4` | `4` | `1` | `—` | `1` | `$4.46` | `6m14s` | `122` | — | — | — | pending review |
| `0` | `git_pathspec_match_list` | struct | `8` | `8` | `2` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_proxy_t` | enum | `—` | `—` | `—` | `—` | `1` | `$1.75` | `4m11s` | `76` | — | — | — | pending review |
| `0` | `git_push` | struct | `11` | `11` | `3` | `—` | `1` | `$3.78` | `6m44s` | `62` | — | — | — | pending review |
| `0` | `git_push_transfer_progress_cb` | callback | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `1` | `git_push_update` | struct | `4` | `4` | `2` | `4` | `1` | `$4.19` | `8m37s` | `227` | — | — | — | pending review |
| `0` | `git_push_update_reference_cb` | callback | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_rebase` | struct | `20` | `20` | `7` | `—` | `1` | `$2.96` | `5m17s` | `97` | — | — | — | pending review |
| `1` | `git_rebase_operation` | struct | `3` | `3` | `1` | `3` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_rebase_operation_t` | enum | `—` | `—` | `—` | `—` | `1` | `$4.82` | `7m35s` | `143` | — | — | — | pending review |
| `0` | `git_refcount` | struct | `2` | `2` | `1` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_refdb` | struct | `3` | `3` | `2` | `—` | `1` | `$3.82` | `7m23s` | `127` | — | — | — | pending review |
| `0` | `git_refdb_t` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_reference` | struct | `7` | `7` | `2` | `—` | `1` | `$7.07` | `12m55s` | `193` | — | — | — | pending review |
| `1` | `git_reference_iterator` | struct | `4` | `4` | `4` | `4` | `1` | `$7.18` | `14m40s` | `415` | — | — | — | pending review |
| `0` | `git_reference_t` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_reflog` | struct | `4` | `4` | `2` | `—` | `1` | `$4.09` | `8m45s` | `104` | — | — | — | pending review |
| `0` | `git_reflog_entry` | struct | `4` | `4` | `2` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_refspec` | struct | `7` | `7` | `3` | `—` | `1` | `$3.20` | `7m14s` | `70` | — | — | — | pending review |
| `0` | `git_remote` | struct | `17` | `17` | `6` | `—` | `1` | `$3.70` | `7m18s` | `154` | — | — | — | pending review |
| `0` | `git_remote_autotag_option_t` | enum | `—` | `—` | `—` | `—` | `1` | `$1.96` | `4m12s` | `117` | — | — | — | pending review |
| `0` | `git_remote_completion_t` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `1` | `git_remote_head` | struct | `5` | `5` | `2` | `5` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_remote_redirect_t` | enum | `—` | `—` | `—` | `—` | `1` | `$2.63` | `5m14s` | `61` | — | — | — | pending review |
| `0` | `git_repository` | struct | `29` | `28` | `17` | `—` | `1` | `$3.65` | `6m11s` | `74` | — | — | — | pending review |
| `1` | `git_repository_fetchhead_foreach_cb` | callback | `—` | `—` | `—` | `—` | `1` | `$6.14` | `11m04s` | `497` | — | — | — | pending review |
| `1` | `git_repository_init_options` | struct | `10` | `10` | `5` | `10` | `1` | `$3.89` | `9m14s` | `355` | — | — | — | pending review |
| `1` | `git_repository_mergehead_foreach_cb` | callback | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_reset_t` | enum | `—` | `—` | `—` | `—` | `1` | `$1.93` | `3m41s` | `74` | — | — | — | pending review |
| `1` | `git_revspec` | struct | `3` | `3` | `2` | `3` | `1` | `$4.98` | `12m50s` | `527` | — | — | — | pending review |
| `0` | `git_revwalk` | struct | `21` | `21` | `11` | `—` | `1` | `$2.87` | `6m22s` | `70` | — | — | — | pending review |
| `1` | `git_signature` | struct | `3` | `3` | `2` | `3` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_stash_apply_progress_t` | enum | `—` | `—` | `—` | `—` | `1` | `$3.75` | `6m56s` | `187` | — | — | — | pending review |
| `0` | `git_status_list` | struct | `4` | `4` | `2` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_status_show_t` | enum | `—` | `—` | `—` | `—` | `1` | `$1.94` | `4m13s` | `243` | — | — | — | pending review |
| `0` | `git_status_t` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_strarray` | struct | `2` | `2` | `1` | `2` | `1` | `$5.22` | `8m59s` | `203` | — | — | — | pending review |
| `0` | `git_submodule` | struct | `16` | `16` | `5` | `—` | `1` | `$3.32` | `7m12s` | `106` | — | — | — | pending review |
| `0` | `git_submodule_ignore_t` | enum | `—` | `—` | `—` | `—` | `1` | `$2.42` | `4m44s` | `162` | — | — | — | pending review |
| `0` | `git_submodule_update_t` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `1` | `git_tag` | struct | `6` | `6` | `3` | `—` | `1` | `$3.30` | `5m49s` | `146` | — | — | — | pending review |
| `0` | `git_time` | struct | `3` | `3` | `0` | `3` | `1` | `$3.76` | `7m43s` | `258` | — | — | — | pending review |
| `0` | `git_trace_level_t` | enum | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_transaction` | struct | `7` | `7` | `4` | `—` | `1` | `$4.38` | `9m40s` | `194` | — | — | — | pending review |
| `0` | `git_transport_message_cb` | callback | `—` | `—` | `—` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `1` | `git_tree` | struct | `6` | `6` | `2` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_tree_entry` | struct | `4` | `4` | `1` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `1` | `git_tree_update` | struct | `4` | `4` | `1` | `4` | `1` | `$3.26` | `5m54s` | `167` | — | — | — | pending review |
| `0` | `git_tree_update_t` | enum | `—` | `—` | `—` | `—` | `1` | `$3.02` | `6m09s` | `146` | — | — | — | pending review |
| `0` | `git_treebuilder` | struct | `3` | `3` | `1` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_treewalk_mode` | enum | `—` | `—` | `—` | `—` | `1` | `$3.26` | `6m28s` | `138` | — | — | — | pending review |
| `0` | `git_worktree` | struct | `7` | `7` | `6` | `—` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| `0` | `git_worktree_prune_options` | struct | `2` | `2` | `0` | `2` | `1` | `$4.91` | `11m21s` | `255` | — | — | — | pending review |
| `0` | `git_writestream` | struct | `3` | `3` | `3` | `3` | `1` | `↖ batched` | `↖ batched` | `↖ batched` | — | — | — | pending review |
| **Σ `127`** | | | **`580`** | **`576`** | **`215`** | **`170`** | **`127`** | **`$306.65`** | | **`17306`** | **—** | | **—** | **`0`/`127` reviewed** |

### Batches — types

| DAG layer | units | loc | $ | wall (longest) | wall (actual) | serial Σ | $/unit | $/loc |
|---|---|---|---|---|---|---|---|---|
| `0` | `88` | `8315` | `$186.53` | `12m55s` | **multi-session** | `6h13m56s` (`28.9`x) | `$2.12` | `$0.022` |
| `1` | `27` | `5057` | `$79.84` | `14m40s` | **multi-session** | `2h43m36s` (`11.1`x) | `$2.96` | `$0.016` |
| **Σ** | **`115`** | **`13372`** | **`$266.37`** | — | **multi-session** | **`8h57m32s`** | **`$2.32`** | **`$0.020`** |

### Symbols

| DAG layer | symbol | kind | target fns | deps | wrappers | batch | rv batch | verdict |
|---|---|---|---|---|---|---|---|---|
| `1` | `git_annotated_commit_free` | function | `8` | `git_annotated_commit` | `0` | `S73` | — | pending review |
| `1` | `git_annotated_commit_from_fetchhead` | function | `0` | `git_annotated_commit`, `git_oid`, `git_repository` | `1` | `S73` | — | pending review |
| `1` | `git_annotated_commit_from_ref` | function | `3` | `git_annotated_commit`, `git_reference`, `git_repository` | `1` | `S73` | — | pending review |
| `1` | `git_annotated_commit_id` | function | `7` | `git_annotated_commit`, `git_oid` | `1` | `S73` | — | pending review |
| `1` | `git_annotated_commit_lookup` | function | `2` | `git_annotated_commit`, `git_oid`, `git_repository` | `1` | `S73` | — | pending review |
| `1` | `git_annotated_commit_ref` | function | `0` | `git_annotated_commit` | `1` | `S73` | — | pending review |
| `1` | `git_attr_get` | function | `1` | `git_repository` | `1` | `S73` | — | pending review |
| `1` | `git_attr_value` | function | `5` | `git_attr_value_t` | `1` | `S73` | — | pending review |
| `1` | `git_blame_buffer` | function | `0` | `git_blame` | `1` | `S73` | — | pending review |
| `1` | `git_blame_free` | function | `2` | `git_blame` | `0` | `S73` | — | pending review |
| `1` | `git_blame_get_hunk_count` | function | `0` | `git_blame` | `1` | `S73` | — | pending review |
| `1` | `git_blob_create_frombuffer` | function | `0` | `git_oid`, `git_repository` | `1` | `S73` | — | pending review |
| `1` | `git_blob_create_fromdisk` | function | `0` | `git_oid`, `git_repository` | `1` | `S73` | — | pending review |
| `1` | `git_blob_create_fromstream` | function | `0` | `git_repository`, `git_writestream` | `1` | `S73` | — | pending review |
| `1` | `git_blob_create_fromstream_commit` | function | `0` | `git_oid`, `git_writestream` | `1` | `S73` | — | pending review |
| `1` | `git_branch_create_from_annotated` | function | `0` | `git_annotated_commit`, `git_reference`, `git_repository` | `1` | `S73` | — | pending review |
| `1` | `git_branch_delete` | function | `0` | `git_reference` | `1` | `S73` | — | pending review |
| `1` | `git_branch_is_head` | function | `2` | `git_reference` | `1` | `S73` | — | pending review |
| `1` | `git_branch_iterator_free` | function | `0` | `git_branch_iterator` | `0` | `S73` | — | pending review |
| `1` | `git_branch_iterator_new` | function | `0` | `git_branch_iterator`, `git_branch_t`, `git_repository` | `1` | `S73` | — | pending review |
| `1` | `git_branch_lookup` | function | `3` | `git_branch_t`, `git_reference`, `git_repository` | `1` | `S73` | — | pending review |
| `1` | `git_branch_move` | function | `0` | `git_reference` | `1` | `S73` | — | pending review |
| `1` | `git_branch_name` | function | `0` | `git_reference` | `1` | `S73` | — | pending review |
| `0` | `git_branch_name_is_valid` | function | `0` | — | `1` | `S55` | — | pending review |
| `1` | `git_branch_next` | function | `0` | `git_branch_iterator`, `git_branch_t`, `git_reference` | `1` | `S73` | — | pending review |
| `1` | `git_branch_remote_name` | function | `0` | `git_buf`, `git_repository` | `1` | `S73` | — | pending review |
| `1` | `git_branch_set_upstream` | function | `0` | `git_reference` | `1` | `S73` | — | pending review |
| `1` | `git_branch_upstream` | function | `1` | `git_reference` | `1` | `S73` | — | pending review |
| `1` | `git_branch_upstream_merge` | function | `0` | `git_buf`, `git_repository` | `1` | `S73` | — | pending review |
| `1` | `git_branch_upstream_name` | function | `0` | `git_buf`, `git_repository` | `1` | `S73` | — | pending review |
| `1` | `git_branch_upstream_remote` | function | `0` | `git_buf`, `git_repository` | `1` | `S73` | — | pending review |
| `1` | `git_buf_dispose` | function | `4` | `git_buf` | `0` | `S73` | — | pending review |
| `1` | `git_commit_create_with_signature` | function | `1` | `git_oid`, `git_repository` | `1` | `S73` | — | pending review |
| `1` | `git_commit_extract_signature` | function | `0` | `git_buf`, `git_oid`, `git_repository` | `1` | `S73` | — | pending review |
| `1` | `git_config_add_file_ondisk` | function | `4` | `git_config`, `git_config_level_t`, `git_repository` | `2` | `S73` | — | pending review |
| `1` | `git_config_delete_entry` | function | `8` | `git_config` | `1` | `S73` | — | pending review |
| `1` | `git_config_delete_multivar` | function | `1` | `git_config` | `1` | `S73` | — | pending review |
| `1` | `git_config_find_global` | function | `0` | `git_buf` | `1` | `S73` | — | pending review |
| `1` | `git_config_find_system` | function | `0` | `git_buf` | `1` | `S73` | — | pending review |
| `1` | `git_config_find_xdg` | function | `0` | `git_buf` | `1` | `S73` | — | pending review |
| `1` | `git_config_free` | function | `37` | `git_config` | `0` | `S73` | — | pending review |
| `1` | `git_config_get_bool` | function | `4` | `git_config` | `1` | `S73` | — | pending review |
| `1` | `git_config_get_int32` | function | `1` | `git_config` | `1` | `S73` | — | pending review |
| `1` | `git_config_get_int64` | function | `1` | `git_config` | `1` | `S73` | — | pending review |
| `1` | `git_config_get_path` | function | `0` | `git_buf`, `git_config` | `1` | `S73` | — | pending review |
| `1` | `git_config_get_string` | function | `8` | `git_config` | `1` | `S73` | — | pending review |
| `1` | `git_config_get_string_buf` | function | `0` | `git_buf`, `git_config` | `1` | `S73` | — | pending review |
| `1` | `git_config_new` | function | `5` | `git_config` | `1` | `S73` | — | pending review |
| `1` | `git_config_open_default` | function | `4` | `git_config` | `1` | `S73` | — | pending review |
| `1` | `git_config_open_global` | function | `0` | `git_config` | `1` | `S73` | — | pending review |
| `1` | `git_config_open_level` | function | `2` | `git_config`, `git_config_level_t` | `1` | `S74` | — | pending review |
| `1` | `git_config_open_ondisk` | function | `3` | `git_config` | `1` | `S74` | — | pending review |
| `0` | `git_config_parse_bool` | function | `8` | — | `1` | `S55` | — | pending review |
| `0` | `git_config_parse_int32` | function | `4` | — | `1` | `S55` | — | pending review |
| `0` | `git_config_parse_int64` | function | `2` | — | `1` | `S55` | — | pending review |
| `1` | `git_config_set_bool` | function | `4` | `git_config` | `1` | `S74` | — | pending review |
| `1` | `git_config_set_int32` | function | `2` | `git_config` | `1` | `S74` | — | pending review |
| `1` | `git_config_set_int64` | function | `1` | `git_config` | `1` | `S74` | — | pending review |
| `1` | `git_config_set_multivar` | function | `2` | `git_config` | `1` | `S74` | — | pending review |
| `1` | `git_config_set_string` | function | `13` | `git_config` | `1` | `S74` | — | pending review |
| `1` | `git_config_snapshot` | function | `2` | `git_config` | `1` | `S74` | — | pending review |
| `1` | `git_describe_commit` | function | `1` | `git_describe_options`, `git_describe_result`, `git_object` | `1` | `S74` | — | pending review |
| `1` | `git_describe_format` | function | `0` | `git_buf`, `git_describe_format_options`, `git_describe_result` | `1` | `S74` | — | pending review |
| `1` | `git_describe_result_free` | function | `2` | `git_describe_result` | `1` | `S74` | — | pending review |
| `1` | `git_describe_workdir` | function | `0` | `git_describe_options`, `git_describe_result`, `git_repository` | `1` | `S74` | — | pending review |
| `1` | `git_diff_free` | function | `30` | `git_diff` | `1` | `S74` | — | pending review |
| `1` | `git_diff_from_buffer` | function | `0` | `git_diff` | `1` | `S74` | — | pending review |
| `1` | `git_diff_get_stats` | function | `1` | `git_diff`, `git_diff_stats` | `1` | `S74` | — | pending review |
| `1` | `git_diff_is_sorted_icase` | function | `2` | `git_diff` | `1` | `S74` | — | pending review |
| `1` | `git_diff_merge` | function | `1` | `git_diff` | `1` | `S74` | — | pending review |
| `1` | `git_diff_num_deltas` | function | `14` | `git_diff` | `1` | `S74` | — | pending review |
| `1` | `git_diff_patchid` | function | `0` | `git_diff`, `git_diff_patchid_options`, `git_oid` | `1` | `S74` | — | pending review |
| `1` | `git_diff_patchid_options_init` | function | `0` | `git_diff_patchid_options` | `1` | `S74` | — | pending review |
| `1` | `git_diff_stats_deletions` | function | `0` | `git_diff_stats` | `1` | `S74` | — | pending review |
| `1` | `git_diff_stats_files_changed` | function | `0` | `git_diff_stats` | `1` | `S74` | — | pending review |
| `1` | `git_diff_stats_free` | function | `2` | `git_diff_stats` | `1` | `S74` | — | pending review |
| `1` | `git_diff_stats_insertions` | function | `0` | `git_diff_stats` | `1` | `S74` | — | pending review |
| `1` | `git_diff_stats_to_buf` | function | `0` | `git_buf`, `git_diff_stats`, `git_diff_stats_format_t` | `1` | `S74` | — | pending review |
| `0` | `git_error_clear` | function | `109` | — | `1` | `S55` | — | pending review |
| `1` | `git_error_last` | function | `6` | `git_error` | `1` | `S74` | — | pending review |
| `0` | `git_error_set_str` | function | `5` | — | `1` | `S55` | — | pending review |
| `1` | `git_graph_ahead_behind` | function | `0` | `git_oid`, `git_repository` | `1` | `S74` | — | pending review |
| `1` | `git_graph_descendant_of` | function | `1` | `git_oid`, `git_repository` | `1` | `S74` | — | pending review |
| `1` | `git_ignore_add_rule` | function | `0` | `git_repository` | `1` | `S74` | — | pending review |
| `1` | `git_ignore_clear_internal_rules` | function | `0` | `git_repository` | `1` | `S74` | — | pending review |
| `1` | `git_ignore_path_is_ignored` | function | `2` | `git_repository` | `1` | `S74` | — | pending review |
| `1` | `git_index_conflict_iterator_free` | function | `1` | `git_index_conflict_iterator` | `1` | `S74` | — | pending review |
| `1` | `git_indexer_append` | function | `3` | `git_indexer`, `git_indexer_progress` | `1` | `S74` | — | pending review |
| `1` | `git_indexer_commit` | function | `2` | `git_indexer`, `git_indexer_progress` | `1` | `S74` | — | pending review |
| `1` | `git_indexer_free` | function | `2` | `git_indexer` | `1` | `S74` | — | pending review |
| `1` | `git_indexer_name` | function | `1` | `git_indexer` | `1` | `S74` | — | pending review |
| `0` | `git_libgit2_features` | function | `0` | — | `1` | `S55` | — | pending review |
| `0` | `git_libgit2_opts` | function | `0` | — | `1` | `S55` | — | pending review |
| `0` | `git_libgit2_version` | function | `0` | — | `1` | `S55` | — | pending review |
| `1` | `git_mailmap_add_entry` | function | `0` | `git_mailmap` | `1` | `S74` | — | pending review |
| `1` | `git_mailmap_free` | function | `2` | `git_mailmap` | `1` | `S74` | — | pending review |
| `1` | `git_mailmap_from_buffer` | function | `0` | `git_mailmap` | `1` | `S74` | — | pending review |
| `1` | `git_mailmap_from_repository` | function | `1` | `git_mailmap`, `git_repository` | `1` | `S74` | — | pending review |
| `1` | `git_mailmap_new` | function | `2` | `git_mailmap` | `1` | `S74` | — | pending review |
| `1` | `git_merge_analysis` | function | `0` | `git_annotated_commit`, `git_merge_analysis_t`, `git_merge_preference_t`, `git_repository` | `1` | `S74` | — | pending review |
| `1` | `git_merge_analysis_for_ref` | function | `1` | `git_annotated_commit`, `git_merge_analysis_t`, `git_merge_preference_t`, `git_reference`, `git_repository` | `1` | `S74` | — | pending review |
| `1` | `git_merge_base` | function | `2` | `git_oid`, `git_repository` | `1` | `S75` | — | pending review |
| `1` | `git_merge_base_many` | function | `1` | `git_oid`, `git_repository` | `1` | `S75` | — | pending review |
| `1` | `git_merge_base_octopus` | function | `0` | `git_oid`, `git_repository` | `1` | `S75` | — | pending review |
| `1` | `git_merge_file_input_init` | function | `1` | `git_merge_file_input` | `1` | `S75` | — | pending review |
| `1` | `git_merge_file_result_free` | function | `4` | `git_merge_file_result` | `1` | `S75` | — | pending review |
| `1` | `git_message_prettify` | function | `0` | `git_buf` | `1` | `S75` | — | pending review |
| `1` | `git_note_default_ref` | function | `0` | `git_buf`, `git_repository` | `1` | `S75` | — | pending review |
| `1` | `git_note_free` | function | `1` | `git_note` | `1` | `S75` | — | pending review |
| `1` | `git_note_id` | function | `0` | `git_note`, `git_oid` | `1` | `S75` | — | pending review |
| `1` | `git_note_iterator_free` | function | `1` | `git_iterator` | `1` | `S75` | — | pending review |
| `1` | `git_note_iterator_new` | function | `1` | `git_iterator`, `git_repository` | `1` | `S75` | — | pending review |
| `1` | `git_note_message` | function | `1` | `git_note` | `1` | `S75` | — | pending review |
| `1` | `git_note_next` | function | `1` | `git_iterator`, `git_oid` | `1` | `S75` | — | pending review |
| `1` | `git_note_read` | function | `1` | `git_note`, `git_oid`, `git_repository` | `1` | `S75` | — | pending review |
| `1` | `git_object_dup` | function | `7` | `git_object` | `1` | `S75` | — | pending review |
| `1` | `git_object_free` | function | `39` | `git_object` | `1` | `S75` | — | pending review |
| `1` | `git_object_id` | function | `22` | `git_object`, `git_oid` | `1` | `S75` | — | pending review |
| `1` | `git_object_lookup` | function | `22` | `git_object`, `git_object_t`, `git_oid`, `git_repository` | `1` | `S75` | — | pending review |
| `1` | `git_object_lookup_prefix` | function | `6` | `git_object`, `git_object_t`, `git_oid`, `git_repository` | `1` | `S75` | — | pending review |
| `1` | `git_object_peel` | function | `15` | `git_object`, `git_object_t` | `1` | `S75` | — | pending review |
| `1` | `git_object_short_id` | function | `0` | `git_buf`, `git_object` | `1` | `S75` | — | pending review |
| `1` | `git_object_string2type` | function | `0` | `git_object_t` | `1` | `S75` | — | pending review |
| `1` | `git_object_type` | function | `11` | `git_object`, `git_object_t` | `1` | `S75` | — | pending review |
| `1` | `git_object_type2string` | function | `4` | `git_object_t` | `1` | `S75` | — | pending review |
| `1` | `git_object_typeisloose` | function | `0` | `git_object_t` | `1` | `S75` | — | pending review |
| `1` | `git_odb_hash` | function | `0` | `git_object_t`, `git_oid` | `1` | `S75` | — | pending review |
| `1` | `git_odb_hashfile` | function | `0` | `git_object_t`, `git_oid` | `1` | `S75` | — | pending review |
| `1` | `git_oid_cmp` | function | `16` | `git_oid` | `1` | `S75` | — | pending review |
| `1` | `git_oid_equal` | function | `54` | `git_oid` | `1` | `S75` | — | pending review |
| `1` | `git_oid_fromraw` | function | `0` | `git_oid` | `1` | `S75` | — | pending review |
| `1` | `git_oid_fromstrn` | function | `0` | `git_oid` | `1` | `S75` | — | pending review |
| `1` | `git_oid_is_zero` | function | `29` | `git_oid` | `1` | `S75` | — | pending review |
| `1` | `git_oid_tostr` | function | `28` | `git_oid` | `1` | `S75` | — | pending review |
| `1` | `git_packbuilder_foreach` | function | `3` | `git_packbuilder`, `git_packbuilder_foreach_cb` | `1` | `S75` | — | pending review |
| `1` | `git_packbuilder_free` | function | `4` | `git_packbuilder` | `1` | `S75` | — | pending review |
| `1` | `git_packbuilder_hash` | function | `0` | `git_oid`, `git_packbuilder` | `1` | `S75` | — | pending review |
| `1` | `git_packbuilder_insert` | function | `9` | `git_oid`, `git_packbuilder` | `1` | `S75` | — | pending review |
| `1` | `git_packbuilder_insert_commit` | function | `2` | `git_oid`, `git_packbuilder` | `1` | `S75` | — | pending review |
| `1` | `git_packbuilder_insert_recur` | function | `2` | `git_oid`, `git_packbuilder` | `1` | `S75` | — | pending review |
| `1` | `git_packbuilder_insert_tree` | function | `2` | `git_oid`, `git_packbuilder` | `1` | `S75` | — | pending review |
| `1` | `git_packbuilder_insert_walk` | function | `2` | `git_packbuilder`, `git_revwalk` | `1` | `S75` | — | pending review |
| `1` | `git_packbuilder_name` | function | `0` | `git_packbuilder` | `1` | `S75` | — | pending review |
| `1` | `git_packbuilder_new` | function | `3` | `git_packbuilder`, `git_repository` | `1` | `S75` | — | pending review |
| `1` | `git_packbuilder_object_count` | function | `1` | `git_packbuilder` | `1` | `S75` | — | pending review |
| `1` | `git_packbuilder_set_callbacks` | function | `2` | `git_packbuilder`, `git_packbuilder_progress` | `1` | `S75` | — | pending review |
| `1` | `git_packbuilder_set_threads` | function | `3` | `git_packbuilder` | `1` | `S75` | — | pending review |
| `1` | `git_packbuilder_write_buf` | function | `0` | `git_buf`, `git_packbuilder` | `1` | `S75` | — | pending review |
| `1` | `git_packbuilder_written` | function | `0` | `git_packbuilder` | `1` | `S75` | — | pending review |
| `1` | `git_patch_free` | function | `8` | `git_patch` | `1` | `S75` | — | pending review |
| `1` | `git_patch_from_diff` | function | `4` | `git_diff`, `git_patch` | `1` | `S76` | — | pending review |
| `1` | `git_patch_get_hunk` | function | `0` | `git_diff_hunk`, `git_patch` | `1` | `S76` | — | pending review |
| `1` | `git_patch_get_line_in_hunk` | function | `0` | `git_diff_line`, `git_patch` | `1` | `S76` | — | pending review |
| `1` | `git_patch_line_stats` | function | `1` | `git_patch` | `1` | `S76` | — | pending review |
| `1` | `git_patch_num_hunks` | function | `0` | `git_patch` | `1` | `S76` | — | pending review |
| `1` | `git_patch_num_lines_in_hunk` | function | `0` | `git_patch` | `1` | `S76` | — | pending review |
| `1` | `git_patch_size` | function | `0` | `git_patch` | `1` | `S76` | — | pending review |
| `1` | `git_patch_to_buf` | function | `0` | `git_buf`, `git_patch` | `1` | `S76` | — | pending review |
| `1` | `git_pathspec_free` | function | `1` | `git_pathspec` | `1` | `S76` | — | pending review |
| `1` | `git_pathspec_match_diff` | function | `0` | `git_diff`, `git_pathspec`, `git_pathspec_match_list` | `1` | `S76` | — | pending review |
| `1` | `git_pathspec_match_list_entry` | function | `0` | `git_pathspec_match_list` | `1` | `S76` | — | pending review |
| `1` | `git_pathspec_match_list_entrycount` | function | `0` | `git_pathspec_match_list` | `1` | `S76` | — | pending review |
| `1` | `git_pathspec_match_list_failed_entry` | function | `0` | `git_pathspec_match_list` | `1` | `S76` | — | pending review |
| `1` | `git_pathspec_match_list_failed_entrycount` | function | `0` | `git_pathspec_match_list` | `1` | `S76` | — | pending review |
| `1` | `git_pathspec_match_list_free` | function | `0` | `git_pathspec_match_list` | `1` | `S76` | — | pending review |
| `1` | `git_pathspec_match_workdir` | function | `0` | `git_pathspec`, `git_pathspec_match_list`, `git_repository` | `1` | `S76` | — | pending review |
| `1` | `git_pathspec_matches_path` | function | `0` | `git_pathspec` | `1` | `S76` | — | pending review |
| `1` | `git_pathspec_new` | function | `0` | `git_pathspec`, `git_strarray` | `1` | `S76` | — | pending review |
| `1` | `git_rebase_abort` | function | `0` | `git_rebase` | `1` | `S76` | — | pending review |
| `1` | `git_rebase_free` | function | `2` | `git_rebase` | `1` | `S76` | — | pending review |
| `1` | `git_rebase_operation_current` | function | `0` | `git_rebase` | `1` | `S76` | — | pending review |
| `1` | `git_rebase_operation_entrycount` | function | `0` | `git_rebase` | `1` | `S76` | — | pending review |
| `1` | `git_rebase_orig_head_id` | function | `0` | `git_oid`, `git_rebase` | `1` | `S76` | — | pending review |
| `1` | `git_rebase_orig_head_name` | function | `0` | `git_rebase` | `1` | `S76` | — | pending review |
| `1` | `git_refdb_compress` | function | `0` | `git_refdb` | `1` | `S76` | — | pending review |
| `1` | `git_refdb_free` | function | `6` | `git_refdb` | `1` | `S76` | — | pending review |
| `1` | `git_reference_cmp` | function | `1` | `git_reference` | `1` | `S76` | — | pending review |
| `1` | `git_reference_create` | function | `17` | `git_oid`, `git_reference`, `git_repository` | `1` | `S76` | — | pending review |
| `1` | `git_reference_create_matching` | function | `4` | `git_oid`, `git_reference`, `git_repository` | `1` | `S76` | — | pending review |
| `1` | `git_reference_delete` | function | `5` | `git_reference` | `1` | `S76` | — | pending review |
| `1` | `git_reference_dup` | function | `1` | `git_reference` | `1` | `S76` | — | pending review |
| `1` | `git_reference_dwim` | function | `4` | `git_reference`, `git_repository` | `1` | `S76` | — | pending review |
| `1` | `git_reference_ensure_log` | function | `1` | `git_repository` | `1` | `S76` | — | pending review |
| `1` | `git_reference_free` | function | `84` | `git_reference` | `1` | `S76` | — | pending review |
| `1` | `git_reference_has_log` | function | `0` | `git_repository` | `1` | `S76` | — | pending review |
| `1` | `git_reference_is_branch` | function | `9` | `git_reference` | `1` | `S76` | — | pending review |
| `1` | `git_reference_is_note` | function | `0` | `git_reference` | `1` | `S76` | — | pending review |
| `1` | `git_reference_is_remote` | function | `3` | `git_reference` | `1` | `S76` | — | pending review |
| `1` | `git_reference_is_tag` | function | `1` | `git_reference` | `1` | `S76` | — | pending review |
| `1` | `git_reference_lookup` | function | `28` | `git_reference`, `git_repository` | `1` | `S76` | — | pending review |
| `1` | `git_reference_name` | function | `18` | `git_reference` | `1` | `S76` | — | pending review |
| `0` | `git_reference_name_is_valid` | function | `8` | — | `1` | `S55` | — | pending review |
| `1` | `git_reference_name_to_id` | function | `13` | `git_oid`, `git_repository` | `1` | `S76` | — | pending review |
| `0` | `git_reference_normalize_name` | function | `1` | — | `1` | `S55` | — | pending review |
| `1` | `git_reference_peel` | function | `12` | `git_object`, `git_object_t`, `git_reference` | `1` | `S76` | — | pending review |
| `1` | `git_reference_rename` | function | `2` | `git_reference` | `1` | `S76` | — | pending review |
| `1` | `git_reference_resolve` | function | `4` | `git_reference` | `1` | `S76` | — | pending review |
| `1` | `git_reference_set_target` | function | `0` | `git_oid`, `git_reference` | `1` | `S76` | — | pending review |
| `1` | `git_reference_shorthand` | function | `0` | `git_reference` | `1` | `S76` | — | pending review |
| `1` | `git_reference_symbolic_create` | function | `4` | `git_reference`, `git_repository` | `1` | `S76` | — | pending review |
| `1` | `git_reference_symbolic_create_matching` | function | `2` | `git_reference`, `git_repository` | `1` | `S76` | — | pending review |
| `1` | `git_reference_symbolic_set_target` | function | `2` | `git_reference` | `1` | `S76` | — | pending review |
| `1` | `git_reference_symbolic_target` | function | `13` | `git_reference` | `1` | `S77` | — | pending review |
| `1` | `git_reference_target` | function | `21` | `git_oid`, `git_reference` | `1` | `S77` | — | pending review |
| `1` | `git_reference_target_peel` | function | `0` | `git_oid`, `git_reference` | `1` | `S77` | — | pending review |
| `1` | `git_reference_type` | function | `20` | `git_reference`, `git_reference_t` | `1` | `S77` | — | pending review |
| `1` | `git_reflog_delete` | function | `0` | `git_repository` | `1` | `S77` | — | pending review |
| `1` | `git_reflog_drop` | function | `1` | `git_reflog` | `1` | `S77` | — | pending review |
| `1` | `git_reflog_entry_byindex` | function | `8` | `git_reflog`, `git_reflog_entry` | `1` | `S77` | — | pending review |
| `1` | `git_reflog_entry_id_new` | function | `3` | `git_oid`, `git_reflog_entry` | `1` | `S77` | — | pending review |
| `1` | `git_reflog_entry_id_old` | function | `0` | `git_oid`, `git_reflog_entry` | `1` | `S77` | — | pending review |
| `1` | `git_reflog_entry_message` | function | `2` | `git_reflog_entry` | `1` | `S77` | — | pending review |
| `1` | `git_reflog_entrycount` | function | `7` | `git_reflog` | `1` | `S77` | — | pending review |
| `1` | `git_reflog_free` | function | `7` | `git_reflog` | `1` | `S77` | — | pending review |
| `1` | `git_reflog_read` | function | `5` | `git_reflog`, `git_repository` | `1` | `S77` | — | pending review |
| `1` | `git_reflog_rename` | function | `0` | `git_repository` | `1` | `S77` | — | pending review |
| `1` | `git_reflog_write` | function | `0` | `git_reflog` | `1` | `S77` | — | pending review |
| `1` | `git_refspec_direction` | function | `0` | `git_direction`, `git_refspec` | `1` | `S77` | — | pending review |
| `1` | `git_refspec_dst` | function | `0` | `git_refspec` | `1` | `S77` | — | pending review |
| `1` | `git_refspec_dst_matches` | function | `5` | `git_refspec` | `1` | `S77` | — | pending review |
| `1` | `git_refspec_force` | function | `0` | `git_refspec` | `1` | `S77` | — | pending review |
| `1` | `git_refspec_rtransform` | function | `0` | `git_buf`, `git_refspec` | `1` | `S77` | — | pending review |
| `1` | `git_refspec_src` | function | `1` | `git_refspec` | `1` | `S77` | — | pending review |
| `1` | `git_refspec_src_matches` | function | `7` | `git_refspec` | `1` | `S77` | — | pending review |
| `1` | `git_refspec_string` | function | `0` | `git_refspec` | `1` | `S77` | — | pending review |
| `1` | `git_refspec_transform` | function | `0` | `git_buf`, `git_refspec` | `1` | `S77` | — | pending review |
| `1` | `git_remote_add_fetch` | function | `0` | `git_repository` | `1` | `S77` | — | pending review |
| `1` | `git_remote_add_push` | function | `0` | `git_repository` | `1` | `S77` | — | pending review |
| `1` | `git_remote_connected` | function | `3` | `git_remote` | `1` | `S77` | — | pending review |
| `1` | `git_remote_create` | function | `2` | `git_remote`, `git_repository` | `1` | `S77` | — | pending review |
| `1` | `git_remote_create_anonymous` | function | `0` | `git_remote`, `git_repository` | `1` | `S77` | — | pending review |
| `1` | `git_remote_create_detached` | function | `0` | `git_remote` | `1` | `S77` | — | pending review |
| `1` | `git_remote_create_with_fetchspec` | function | `0` | `git_remote`, `git_repository` | `1` | `S77` | — | pending review |
| `1` | `git_remote_default_branch` | function | `0` | `git_buf`, `git_remote` | `1` | `S77` | — | pending review |
| `1` | `git_remote_delete` | function | `0` | `git_repository` | `1` | `S77` | — | pending review |
| `1` | `git_remote_disconnect` | function | `3` | `git_remote` | `1` | `S77` | — | pending review |
| `1` | `git_remote_dup` | function | `1` | `git_remote` | `1` | `S77` | — | pending review |
| `1` | `git_remote_free` | function | `14` | `git_remote` | `1` | `S77` | — | pending review |
| `1` | `git_remote_get_fetch_refspecs` | function | `0` | `git_remote`, `git_strarray` | `1` | `S77` | — | pending review |
| `1` | `git_remote_get_push_refspecs` | function | `0` | `git_remote`, `git_strarray` | `1` | `S77` | — | pending review |
| `1` | `git_remote_get_refspec` | function | `1` | `git_refspec`, `git_remote` | `1` | `S77` | — | pending review |
| `1` | `git_remote_list` | function | `1` | `git_repository`, `git_strarray` | `1` | `S77` | — | pending review |
| `1` | `git_remote_lookup` | function | `9` | `git_remote`, `git_repository` | `1` | `S77` | — | pending review |
| `1` | `git_remote_name` | function | `3` | `git_remote` | `1` | `S77` | — | pending review |
| `0` | `git_remote_name_is_valid` | function | `2` | — | `1` | `S55` | — | pending review |
| `1` | `git_remote_oid_type` | function | `2` | `git_oid_t`, `git_remote` | `1` | `S77` | — | pending review |
| `1` | `git_remote_pushurl` | function | `0` | `git_remote` | `1` | `S77` | — | pending review |
| `1` | `git_remote_refspec_count` | function | `1` | `git_remote` | `1` | `S77` | — | pending review |
| `1` | `git_remote_rename` | function | `0` | `git_repository`, `git_strarray` | `1` | `S77` | — | pending review |
| `1` | `git_remote_set_pushurl` | function | `0` | `git_repository` | `1` | `S77` | — | pending review |
| `1` | `git_remote_set_url` | function | `0` | `git_repository` | `1` | `S77` | — | pending review |
| `1` | `git_remote_stats` | function | `0` | `git_indexer_progress`, `git_remote` | `1` | `S77` | — | pending review |
| `1` | `git_remote_stop` | function | `0` | `git_remote` | `1` | `S77` | — | pending review |
| `1` | `git_remote_url` | function | `4` | `git_remote` | `1` | `S78` | — | pending review |
| `1` | `git_repository_commondir` | function | `2` | `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_config` | function | `4` | `git_config`, `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_discover` | function | `0` | `git_buf` | `1` | `S78` | — | pending review |
| `1` | `git_repository_free` | function | `17` | `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_get_namespace` | function | `0` | `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_head` | function | `16` | `git_reference`, `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_head_detached` | function | `0` | `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_is_bare` | function | `11` | `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_is_empty` | function | `3` | `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_is_shallow` | function | `0` | `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_is_worktree` | function | `10` | `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_message` | function | `0` | `git_buf`, `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_message_remove` | function | `0` | `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_oid_type` | function | `2` | `git_oid_t`, `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_open` | function | `8` | `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_open_bare` | function | `0` | `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_open_ext` | function | `2` | `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_open_from_worktree` | function | `2` | `git_repository`, `git_worktree` | `1` | `S78` | — | pending review |
| `1` | `git_repository_path` | function | `6` | `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_refdb` | function | `3` | `git_refdb`, `git_repository` | `1` | `S78` | — | pending review |
| `1` | `git_repository_set_config` | function | `0` | `git_config`, `git_repository` | `1` | `S78` | — | pending review |
| `0` | `git_tag_name_is_valid` | function | `0` | — | `1` | `S55` | — | pending review |
| **Σ `276`** | | | | | **`272`** | **`7 batches`** | **`0` batches** | **`0` held · `0` fixed · pending** |

### Batches — symbols

| DAG layer | units | loc | $ | wall | $/unit | $/loc |
|---|---|---|---|---|---|---|
| `0` | `20` | `811` | `$7.58` | `16m07s` (`1.0`x) | `$0.38` | `$0.009` |
| `1` | `268` | `4753` | `$46.60` | `24m47s` (`4.1`x) | `$0.17` | `$0.010` |
| **Σ** | **`288`** | **`5564`** | **`$54.18`** | **`24m47s`** (`4.8`x; `1h57m53s` serial) | **`$0.19`** | **`$0.010`** |

### Batches — review

The campaign-end review is waiting for explicit user approval.

| session | batch | units | rv loc | rv $ | rv wall | $/symbol | $/type |
|---|---|---|---|---|---|---|---|
| awaiting approval | — | `0` | — | — | — | — | — |
| **Σ** | **`0` agents** | **`0` types · `0` symbols** | **—** | **—** | **—** | **—** | **—** |

## Safety audit

`crustify-audit <repo> unsafe`, unseeded and tree-wide. The first-half
checkpoint is the pre-review snapshot; the after-review column is reserved
until the user authorizes the campaign-end review.

| | before review (`456a0e8fc0`) | after review (`pending`) |
|---|---|---|
| unsafe loc | `2153` | — |
| % of loc | `29.48`% | — |
| blocks | `1286` | — |
| % in `impl T` | `71.85`% | — |
| `unsafe fn` | `494` | — |
| ...of which not sanctioned | `96` | — |
| raw-ptr smell | `7` (`0` wrapped-type sites) | — |
| void-ptr smell | `0` | — |
| FFI calls | `360` | — |
| `&`/`&mut` on a wrapper | `0` | — |
| field proj outside an accessor | `0` | — |

### All metrics

| metric | before | after | Δ | reading |
|---|---|---|---|---|
| `code_lines` | `7304` | — | — | union of HIR definition spans (denominator); `cfg`-disabled items excluded |
| `total_stmts` | `1237` | — | — | statements |
| `unsafe_blocks` | `1286` | — | — | count of `unsafe { }` blocks, macro-expanded included |
| `unsafe_block_stmts` | `58` | — | — | statements inside them |
| `unsafe_block_lines` | `2154` | — | — | their lines, every outermost block |
| `unsafe_block_code_lines` | `2153` | — | — | **`29.48%`** of compiled code lines |
| `unsafe_blocks_wrapper_impl` | `924` | — | — | inside `impl <wrapper T>` |
| `unsafe_blocks_ffi_export` | `3` | — | — | inside the C-ABI gateway |
| `unsafe_fns` | `494` | — | — | `unsafe fn` declarations, post-expansion |
| `unsafe_fns_seam` | `398` | — | — | the sanctioned subset |
| **`unsafe fn` smell** | **`96`** | **—** | **—** | lifecycle hooks, owner seams, and retained-borrow setters; inspected at checkpoint |
| `unsafe_fns_pub` | `489` | — | — | exported from the crate |
| `unsafe_impls` / `unsafe_traits` | `149` / `0` | — | — | lifecycle contracts asserted once per type |
| `ffi_calls` | `360` | — | — | calls to a foreign item |
| `wrapper_newtypes` | `79` | — | — | structural layout newtypes |
| `wrapper_newtypes_declared` | `79` | — | — | `CCell`-declared count |
| `wrapper_declared_nonconformant` | `0` | — | — | declared but structurally nonconformant — **target 0** |
| `wrapper_newtypes_undeclared` | `0` | — | — | structural but undeclared |
| `raw_ptr_args` | `248` | — | — | raw-pointer argument positions |
| `raw_ptr_rets` | `316` | — | — | raw-pointer return positions |
| **total positions** | **`564`** | **—** | **—** | args + returns |
| `raw_ptr_seam` | `557` | — | — | sanctioned seam positions |
| **smell (total − seam)** | **`7`** | **—** | **—** | non-seam remainder; none points to a wrapped C type |
| `raw_ptr_wrapped` | `0` | — | — | actionable wrapped-pointee defect — **target 0** |
| `raw_ptr_in_wrapper` | `0` | — | — | non-seam pointer inside wrapper impl — **target 0** |
| `raw_ptr_derefs` | `342` | — | — | raw dereference volume |
| `ref_to_type_wrapper` | `0` | — | — | `&`/`&mut` on layout newtype — **target 0** |
| `field_proj_wrapped` | `340` | — | — | projection volume |
| `field_proj_outside_impl` | `0` | — | — | projection outside accessors — **target 0** |
| `field_ref_wrapped` | `0` | — | — | forbidden `&(*p).field` — **target 0** |
| `void_ptr_sanctioned` | `240` | — | — | sanctioned `*c_void` positions |
| `void_ptr_smell` | `0` | — | — | `*c_void` elsewhere — **target 0** |

### What the review moved

Campaign-end review has not run. Before review, the categorical targets
already hold at `0`: nonconformant wrappers, undeclared wrappers, raw pointers
to wrapped types, raw pointers inside wrapper impls, references to layout
wrappers, field references, field projections outside accessors, and void-pointer
smells. The after-review column will record any movement once approved.

## Notes

### Exact half boundary

The frozen dependency-ordered closure contains `805` units. This checkpoint
covers the exact `403`-unit topological prefix; the held `402`-unit suffix has
not been scheduled, homed, translated, reviewed, or audited as campaign work.

### Landing recovery

The first parallel session exposed a shared-session ref race after 11 landed
batches. Those forward commits were preserved, the remaining 377 units were
rescheduled, and `git_cached_obj` was closed in a final one-item batch. A
crustify-cli safeguard now requires atomic forward-only landing; the tool fix is
recorded in crustify-cli `8d8c4dff28`.

### Deterministic audit remediation

The first merged scan found `7` actionable raw-pointer
sites and `14` total non-seam positions. Two narrow
translator remediation waves (6 agents, `$22.64`,
`37m08s` serial agent time) moved those to
`0` and `7` respectively.
The two remaining raw dereference sites are C callback trampolines in
`pack_objects.rs`; each reconstructs the exact callback payload type for the
synchronous call and carries a local safety proof.

### Regression gates

Canonical Rust gates passed: `cargo fmt --check`, workspace `check`, clippy with
warnings denied, and `293` unit tests (`292` wrapper tests plus one allocator
test). The retained C sanitizer suite passed; one SSH invocation hit the known
intermittent ASan startup `DEADLYSIGNAL`, and its immediate verbose rerun passed.

### Review and UB policy

The raw void/string reviews cost `$7.54` over `14m31s` serial agent time and are already landed. The campaign-end
review has not started and requires explicit user approval. `crustify-audit ub`
has not run and also requires explicit approval.
