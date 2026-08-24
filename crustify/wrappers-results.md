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
- **`--parallel-max`** — `8` (`4` then `2` for the two remediation waves)
- **branch** — `crustify/src-gpt-5.6-sol`, tip `c5e2ec28d`
- **deps** — crustify-cli `51d44d1` (`docs/results-template-ub`), ffibox `600399f` (`main`)

## Review pass

`--objective review`, LLM-as-a-Judge over the landed waves.

- **agent backend** — `claude`
- **model** — `anthropic/claude-opus-5`
- **`--billing`** — `subscription`
- **`--max-types`** — `15`
- **`--max-syms`** — `150`
- **`--max-loc`** — `3000`
- **`--min-fields`** — `30`
- **`--parallel-max`** — `16`
- **branch** — `crustify/src-gpt-5.6-sol`, tip `c5e2ec28d`
- **agents** — `23`, over `2` session(s)

`rv`-prefixed columns below carry the review pass; the unprefixed ones remain
the campaign's.

## UB pass

`crustify-audit ub`, an agentic hunt for undefined behaviour reachable from the
crate's SAFE APIs.

- **agent backend** — not run
- **model** — not run
- **`--billing`** — not run
- **`--timeout`** — not run
- **subject** — not run
- **agents** — `0`, `—` wall, `—`
- **advisories** — `0`; `crustify/audit/advisories/` does not exist
- **patch** — none; the pass requires separate explicit approval, which has not been given

`ub`-prefixed columns carry this pass.

## Legend


- `objective` — what the batch's agents were told to do: `wrap`, `port`, or
  `raw lifetime`. The type tables are split by it, so it appears as a column
  only in `Batches — symbols`, which mixes the two
- `types` / `symbols` — scheduler units in the batch. Callbacks are scheduled
  in symbol batches and counted there
- `fields` — in-scope fields: the field accessors the oracle assigned to that
  type batch, not the type's full declared field count
- `lifecycle prims` — deleters, disposers and cloners the ownership store binds
  to that batch's types; raw-tier primitives that belong to no type are counted
  in `Raw lifetime discovery` instead
- `$` / `wall` / `loc` — that agent's computed cost, its elapsed time, and the
  `.rs` insertions of its landing commit. `wall` is `ended_at − started_at` from
  the agent's own `usage.json`, so it INCLUDES the per-worktree C rebuild
- `$/type` / `$/symbol` / `$/field` / `$/loc` — that row's `$` over its units,
  its in-scope fields, or its `loc`
- `$/type` / `$/sym` — in the Overview, a sub-campaign's cost over the types or
  symbols it was scheduled for; `—` where it was scheduled for none
- `rv $` / `rv wall` / `rv loc` — the REVIEW agent's cost, elapsed time, and net
  `.rs` line delta (`+ins/-del`) of its landing commit. Under subscription
  billing `rv $` is an API-equivalent comparison value, not a charged amount
- `ub $` / `ub wall` — the UB pass's cost and elapsed time; `—` where the
  optional pass did not run

Every table below is a heading, a model line and the table. All prose belongs
in Notes.

## Overview

- **Rust LoC** — `7,423`
- **C LoC** — `171,084`
- **ported types** — `0`
- **ported symbols** — `0`
- **wrapped types** — `115`
- **wrapped symbols** — `288` (`276` functions + `12` callbacks)

Implementation `openai/gpt-5.6-sol` via `codex`; review `anthropic/claude-opus-5`
via `claude`. Each row names the model that produced it.

| sub-campaign | objective | nr types | nr symbols | session wall | total | $/type | $/sym | ub wall | ub $ |
|---|---|---:|---:|---|---:|---:|---:|---|---:|
| `2-raw-lifetime` | raw lifetime | `0` | `3` | `16m37s` | `$9.85` (`gpt-5.6-sol`) | — | `$3.28` | — | — |
| `2-raw-lifetime-review` | review | `0` | `3` | `14m32s` | `$7.54` (`gpt-5.6-sol`) | — | `$2.51` | — | — |
| `6-first-half` | wrap | `115` | `288` | `3h19m28s` | `$365.66` (`gpt-5.6-sol`) | `$2.71` | `$0.19` | — | — |
| `3-review-first-half` | review | `115` | `288` | `1h15m16s` | `$254.37` (`claude-opus-5`) | `$1.89` | `$0.13` | — | — |
| orchestrator | orchestration | `—` | `—` | — | `not metered`+ (`claude-opus-5`) | — | — | — | — |
| **Σ recorded agents** | | **`115`** | **`288`** | **`5h05m53s`** | **`$637.43`** | **`$4.59`** | **`$0.32`** | | **—** |

## Raw lifetime discovery

`openai/gpt-5.6-sol` via `codex`.

| tier | symbols submitted | strategies | CDropped | CCloned | CLenDropped | CLenCloned | $ | wall |
|---|---|---|---|---|---|---|---|---|
| void | `2` | `2` | `2` | `0` | `0` | `0` | `$5.56` | `9m30s` |
| string | `1` | `1` | `1` | `1` | `0` | `0` | `$4.29` | `7m06s` |
| **Σ** | **`3`** | **`3`** | **`3`** | **`1`** | **`0`** | **`0`** | **`$9.85`** | **`16m36s`** |

### Review, in-model

`openai/gpt-5.6-sol` via `codex`.

| tier | symbols | batches | $ | wall |
|---|---|---|---|---|
| void | `2` | `1` | `$3.69` | `0h07m` |
| string | `1` | `1` | `$3.85` | `0h07m` |
| **Σ** | **`3`** | **`2`** | **`$7.54`** | **`0h14m`** |

### Review, independent

Not run; the raw lifetime tiers were judged in-model only.

| symbols | rv loc | rv $ | rv wall | rv $/symbol |
|---|---|---|---|---|
| `0` | — | — | — | — |
| **Σ `0`** | **—** | **—** | — | **—** |

## Target set

### Batches — types, wrap

`openai/gpt-5.6-sol` via `codex`. One row per batch, in execution order.

| types | fields | lifecycle prims | $ | wall | $/type | $/field |
|---|---|---|---|---|---|---|
| `2` | `0` | `1` | `$3.26` | `6m45s` | `$1.63` | `—` |
| `1` | `0` | `0` | `$2.37` | `4m37s` | `$2.37` | `—` |
| `1` | `0` | `1` | `$5.04` | `9m37s` | `$5.04` | `—` |
| `2` | `0` | `1` | `$3.97` | `7m38s` | `$1.99` | `—` |
| `2` | `3` | `2` | `$5.63` | `10m22s` | `$2.82` | `$1.88` |
| `2` | `3` | `0` | `$2.88` | `5m58s` | `$1.44` | `$0.96` |
| `2` | `0` | `0` | `$4.14` | `9m04s` | `$2.07` | `—` |
| `2` | `0` | `0` | `$2.32` | `5m26s` | `$1.16` | `—` |
| `2` | `4` | `0` | `$3.22` | `6m36s` | `$1.61` | `$0.80` |
| `2` | `6` | `0` | `$3.82` | `8m36s` | `$1.91` | `$0.64` |
| `1` | `0` | `2` | `$4.25` | `8m04s` | `$4.25` | `—` |
| `2` | `0` | `0` | `$2.99` | `6m29s` | `$1.49` | `—` |
| `2` | `13` | `0` | `$2.93` | `6m26s` | `$1.47` | `$0.23` |
| `2` | `1` | `0` | `$2.58` | `4m59s` | `$1.29` | `$2.58` |
| `2` | `0` | `0` | `$2.28` | `4m26s` | `$1.14` | `—` |
| `2` | `2` | `0` | `$1.49` | `3m42s` | `$0.74` | `$0.74` |
| `2` | `0` | `0` | `$1.03` | `2m51s` | `$0.52` | `—` |
| `1` | `2` | `0` | `$0.39` | `0m49s` | `$0.39` | `$0.19` |
| `1` | `0` | `1` | `$0.00` | `0m02s` | `$0.00` | `—` |
| `1` | `7` | `0` | `$0.00` | `0m02s` | `$0.00` | `$0.00` |
| `1` | `0` | `1` | `$0.00` | `0m01s` | `$0.00` | `—` |
| `2` | `0` | `1` | `$0.00` | `0m01s` | `$0.00` | `—` |
| `2` | `6` | `0` | `$2.81` | `6m05s` | `$1.41` | `$0.47` |
| `1` | `0` | `2` | `$4.37` | `8m44s` | `$4.37` | `—` |
| `2` | `13` | `0` | `$3.98` | `8m09s` | `$1.99` | `$0.31` |
| `2` | `1` | `0` | `$3.94` | `7m11s` | `$1.97` | `$3.94` |
| `2` | `2` | `0` | `$3.32` | `7m01s` | `$1.66` | `$1.66` |
| `2` | `0` | `0` | `$3.47` | `7m35s` | `$1.73` | `—` |
| `1` | `2` | `0` | `$1.91` | `4m12s` | `$1.91` | `$0.95` |
| `1` | `0` | `1` | `$3.05` | `6m49s` | `$3.05` | `—` |
| `1` | `7` | `0` | `$2.23` | `5m35s` | `$2.23` | `$0.32` |
| `1` | `0` | `1` | `$3.28` | `6m31s` | `$3.28` | `—` |
| `2` | `0` | `1` | `$3.06` | `6m00s` | `$1.53` | `—` |
| `2` | `5` | `0` | `$3.62` | `6m56s` | `$1.81` | `$0.72` |
| `2` | `5` | `1` | `$4.56` | `9m26s` | `$2.28` | `$0.91` |
| `2` | `2` | `0` | `$2.62` | `5m58s` | `$1.31` | `$1.31` |
| `2` | `0` | `2` | `$3.91` | `7m53s` | `$1.96` | `—` |
| `2` | `2` | `0` | `$3.92` | `7m35s` | `$1.96` | `$1.96` |
| `1` | `0` | `1` | `$3.15` | `5m57s` | `$3.15` | `—` |
| `1` | `0` | `1` | `$2.59` | `6m28s` | `$2.59` | `—` |
| `2` | `0` | `1` | `$4.46` | `6m14s` | `$2.23` | `—` |
| `1` | `0` | `0` | `$1.75` | `4m12s` | `$1.75` | `—` |
| `1` | `0` | `1` | `$3.78` | `6m45s` | `$3.78` | `—` |
| `1` | `0` | `1` | `$2.96` | `5m18s` | `$2.96` | `—` |
| `2` | `0` | `0` | `$4.82` | `7m36s` | `$2.41` | `—` |
| `2` | `0` | `1` | `$3.82` | `7m23s` | `$1.91` | `—` |
| `2` | `0` | `2` | `$7.07` | `12m56s` | `$3.53` | `—` |
| `2` | `0` | `1` | `$4.09` | `8m46s` | `$2.04` | `—` |
| `1` | `0` | `2` | `$3.20` | `7m15s` | `$3.20` | `—` |
| `1` | `0` | `2` | `$3.70` | `7m18s` | `$3.70` | `—` |
| `2` | `0` | `0` | `$1.96` | `4m13s` | `$0.98` | `—` |
| `1` | `0` | `0` | `$2.63` | `5m15s` | `$2.63` | `—` |
| `1` | `0` | `1` | `$3.65` | `6m11s` | `$3.65` | `—` |
| `1` | `0` | `0` | `$1.93` | `3m41s` | `$1.93` | `—` |
| `1` | `0` | `1` | `$2.87` | `6m23s` | `$2.87` | `—` |
| `2` | `0` | `0` | `$3.75` | `6m56s` | `$1.87` | `—` |
| `2` | `0` | `0` | `$1.94` | `4m14s` | `$0.97` | `—` |
| `1` | `2` | `3` | `$5.22` | `9m00s` | `$5.22` | `$2.61` |
| `1` | `0` | `2` | `$3.32` | `7m12s` | `$3.32` | `—` |
| `2` | `0` | `0` | `$2.42` | `4m45s` | `$1.21` | `—` |
| `2` | `3` | `0` | `$3.76` | `7m44s` | `$1.88` | `$1.25` |
| `2` | `0` | `1` | `$4.38` | `9m41s` | `$2.19` | `—` |
| `2` | `0` | `0` | `$3.02` | `6m09s` | `$1.51` | `—` |
| `2` | `0` | `0` | `$3.26` | `6m28s` | `$1.63` | `—` |
| `2` | `5` | `0` | `$4.91` | `11m22s` | `$2.45` | `$0.98` |
| `2` | `7` | `0` | `$5.67` | `12m13s` | `$2.83` | `$0.81` |
| `1` | `1` | `0` | `$2.78` | `5m19s` | `$2.78` | `$2.78` |
| `1` | `0` | `2` | `$4.56` | `7m52s` | `$4.56` | `—` |
| `2` | `8` | `1` | `$3.86` | `8m43s` | `$1.93` | `$0.48` |
| `2` | `10` | `0` | `$3.92` | `7m38s` | `$1.96` | `$0.39` |
| `1` | `0` | `1` | `$3.98` | `6m27s` | `$3.98` | `—` |
| `1` | `12` | `0` | `$4.65` | `8m00s` | `$4.65` | `$0.39` |
| `2` | `10` | `0` | `$6.66` | `14m21s` | `$3.33` | `$0.67` |
| `1` | `0` | `1` | `$3.69` | `10m17s` | `$3.69` | `—` |
| `1` | `10` | `1` | `$4.72` | `11m45s` | `$4.72` | `$0.47` |
| `2` | `6` | `0` | `$5.86` | `10m25s` | `$2.93` | `$0.98` |
| `2` | `7` | `0` | `$4.19` | `8m37s` | `$2.09` | `$0.60` |
| `2` | `9` | `1` | `$7.18` | `14m40s` | `$3.59` | `$0.80` |
| `1` | `10` | `0` | `$3.89` | `9m15s` | `$3.89` | `$0.39` |
| `2` | `6` | `0` | `$4.98` | `12m50s` | `$2.49` | `$0.83` |
| `2` | `0` | `2` | `$3.30` | `5m50s` | `$1.65` | `—` |
| `1` | `4` | `0` | `$3.26` | `5m54s` | `$3.26` | `$0.82` |
| `1` | `5` | `2` | `$5.47` | `8m45s` | `$5.47` | `$1.09` |
| `2` | `0` | `1` | `$4.07` | `6m52s` | `$2.04` | `—` |
| `1` | `0` | `1` | `$2.60` | `3m42s` | `$2.60` | `—` |
| `1` | `0` | `2` | `$4.49` | `6m16s` | `$4.49` | `—` |
| `1` | `0` | `1` | `$3.89` | `5m02s` | `$3.89` | `—` |
| `1` | `0` | `1` | `$4.12` | `7m55s` | `$4.12` | `—` |
| `1` | `0` | `1` | `$3.47` | `7m22s` | `$3.47` | `—` |
| `1` | `2` | `0` | `$3.21` | `5m28s` | `$3.21` | `$1.61` |
| **Σ `140`** | **`203`** | **`57`** | **`$311.48`** | — | **`$2.22`** | **`$1.53`** |

### Batches — types, port

No port wave ran; this is a `wrap` campaign.

| types | fields | lifecycle prims | $ | wall | $/type | $/field |
|---|---|---|---|---|---|---|
| `0` | `0` | `0` | — | — | — | — |
| **Σ `0`** | **`0`** | **`0`** | **—** | — | **—** | **—** |

### Batches — review types

`anthropic/claude-opus-5` via `claude`. One row per batch, in execution order.

| types | rv loc | rv $ | rv wall | rv $/type |
|---|---|---|---|---|
| `4` | `+111/-21` | `$9.27` | `15m08s` | `$2.32` |
| `15` | `+181/-11` | `$18.08` | `24m38s` | `$1.21` |
| `13` | `+213/-18` | `$20.79` | `24m43s` | `$1.60` |
| `1` | `+104/-32` | `$8.62` | `14m43s` | `$8.62` |
| `6` | `+153/-9` | `$16.18` | `22m38s` | `$2.70` |
| `8` | `+127/-22` | `$14.56` | `26m22s` | `$1.82` |
| `1` | `+49/-6` | `$9.52` | `16m06s` | `$9.52` |
| `5` | `+117/-18` | `$10.23` | `16m59s` | `$2.05` |
| `5` | `+102/-16` | `$10.94` | `17m07s` | `$2.19` |
| `5` | `+128/-55` | `$13.90` | `19m00s` | `$2.78` |
| `6` | `+73/-2` | `$12.00` | `16m51s` | `$2.00` |
| `11` | `+123/-57` | `$9.54` | `15m22s` | `$0.87` |
| `7` | `+137/-6` | `$10.91` | `16m34s` | `$1.56` |
| `5` | `+116/-35` | `$6.06` | `13m31s` | `$1.21` |
| `4` | — | — | `3m41s` | — |
| `5` | `+19/-3` | `$12.17` | `16m00s` | `$2.43` |
| `7` | `+162/-4` | `$8.52` | `12m37s` | `$1.22` |
| `5` | `+169/-20` | `$10.43` | `16m22s` | `$2.09` |
| `4` | `+103/-19` | `$8.14` | `12m23s` | `$2.04` |
| `2` | `+195/-9` | `$7.04` | `12m44s` | `$3.52` |
| **Σ `119`** | **`+2382/-363`** | **`$216.91`** | — | **`$1.82`** |

### Batches — symbols

`openai/gpt-5.6-sol` via `codex` for `wrap`. One row per batch, in execution order.

| objective | symbols | loc | $ | wall | $/symbol | $/loc |
|---|---|---|---|---|---|---|
| wrap | `20` | `811` | `$7.58` | `16m07s` | `$0.38` | `$0.01` |
| wrap | `50` | `938` | `$7.52` | `16m49s` | `$0.15` | `$0.01` |
| wrap | `41` | `807` | `$12.63` | `24m47s` | `$0.31` | `$0.02` |
| wrap | `50` | `881` | `$6.41` | `19m35s` | `$0.13` | `$0.01` |
| wrap | `50` | `915` | `$7.06` | `14m54s` | `$0.14` | `$0.01` |
| wrap | `50` | `497` | `$6.84` | `14m36s` | `$0.14` | `$0.01` |
| wrap | `22` | `715` | `$6.14` | `11m04s` | `$0.28` | `$0.01` |
| **Σ** | **`283`** | **`5,564`** | **`$54.18`** | | **`$0.19`** | **`$0.01`** |

### Batches — review symbols

`anthropic/claude-opus-5` via `claude`. One row per batch, in execution order.

| symbols | rv loc | rv $ | rv wall | rv $/symbol |
|---|---|---|---|---|
| `20` | `+83/-27` | `$7.02` | `11m03s` | `$0.35` |
| `150` | — | — | `3m35s` | — |
| `90` | `+39/-1` | `$10.63` | `13m35s` | `$0.12` |
| `150` | `+170/-25` | `$19.81` | `19m43s` | `$0.13` |
| **Σ `410`** | **`+292/-53`** | **`$37.46`** | — | **`$0.09`** |

## Safety audit

Deterministic `crustify-audit unsafe`; no model.

### Snapshots

| | before review (`456a0e8fc0`) | after review (`d71102b84`) |
|---|---|---|
| unsafe loc | `2153` | `2164` |
| % of loc | `29.48`% | `29.15`% |
| blocks | `1286` | `1303` |
| % in `impl T` | `71.85`% | `71.99`% |
| `unsafe fn` | `494` | `498` |
| ...of which not sanctioned | `96` | `100` |
| raw-ptr smell | `7` | `8` |
| void-ptr smell | `0` | `0` |
| FFI calls | `360` | `366` |
| `&`/`&mut` on a wrapper | `0` | `0` |
| field proj outside an accessor | `0` | `0` |

### All metrics

| metric | before | after | Δ | reading |
|---|---|---|---|---|
| `code_lines` | `7304` | `7423` | `+119` | union of HIR definition spans (denominator); `cfg`-disabled items excluded |
| `total_stmts` | `1237` | `1266` | `+29` | statements |
| `unsafe_blocks` | `1286` | `1303` | `+17` | count of `unsafe { }` blocks, macro-expanded included |
| `unsafe_block_stmts` | `58` | `62` | `+4` | statements inside them |
| `unsafe_block_lines` | `2154` | `2165` | `+11` | their lines, every outermost block |
| `unsafe_block_code_lines` | `2153` | `2164` | `+11` | **`29.48`% → `29.15`%** |
| `unsafe_blocks_wrapper_impl` | `924` | `938` | `+14` | inside `impl <wrapper T>` |
| `unsafe_blocks_ffi_export` | `3` | `3` | `0` | inside the C-ABI gateway |
| `unsafe_fns` | `494` | `498` | `+4` | `unsafe fn` declarations, post-expansion |
| `unsafe_fns_seam` | `398` | `398` | `0` | ...the sanctioned subset |
| **`unsafe fn` smell** | **`96`** | **`100`** | **`+4`** | the remainder — read each and accept or fix it |
| `unsafe_fns_pub` | `489` | `492` | `+3` | ...of `unsafe_fns`, exported from the crate |
| `unsafe_impls` / `unsafe_traits` | `149` / `0` | `149` / `0` | `0` | lifecycle contracts asserted once per type |
| `ffi_calls` | `360` | `366` | `+6` | calls to a foreign item — the unsafe-FFI-call surface |
| `wrapper_newtypes` | `79` | `79` | `0` | LAYOUT newtypes — `repr(transparent)` over a `repr(C)` type by value, detected structurally |
| `wrapper_newtypes_declared` | `79` | `79` | `0` | the `CCell`-declared count, for comparison |
| `wrapper_declared_nonconformant` | `0` | `0` | `0` | declared but failing the structural test — **target 0** |
| `wrapper_newtypes_undeclared` | `0` | `0` | `0` | structural but undeclared — a hand-written layout newtype |
| `raw_ptr_args` | `248` | `248` | `0` | raw-ptr positions in arguments |
| `raw_ptr_rets` | `316` | `317` | `+1` | raw-ptr positions in returns |
| **total positions** | **`564`** | **`565`** | `+1` | args + rets; disjoint, so this is the surface |
| `raw_ptr_seam` | `557` | `557` | `0` | sanctioned: seam fn / `mod ffi_export` / `extern "C"` / ptr-to-own-`Self` |
| **smell (total − seam)** | **`7`** | **`8`** | `+1` | the non-seam remainder |
| `raw_ptr_wrapped` | `0` | `0` | `0` | **of the smell**: pointee is a C type that HAS a wrapper — the actionable defect |
| `raw_ptr_in_wrapper` | `0` | `0` | `0` | **of the smell**: inside a wrapper impl — the least excusable placement |
| `raw_ptr_derefs` | `342` | `345` | `+3` | `*p` on a raw pointer (volume) |
| `ref_to_type_wrapper` | `0` | `0` | `0` | `&`/`&mut` on a layout newtype — **target 0** |
| `field_proj_wrapped` | `340` | `343` | `+3` | projection VOLUME — shares one HIR shape with `addr_of!`, not a violation |
| `field_proj_outside_impl` | `0` | `0` | `0` | projections outside any accessor — **target 0** |
| `field_ref_wrapped` | `0` | `0` | `0` | `&(*p).field` — forbidden by the translator playbook — **target 0** |
| `void_ptr_sanctioned` | `240` | `240` | `0` | `*c_void` in a seam / `ffi_export` / `extern "C"` signature |
| `void_ptr_smell` | `0` | `0` | `0` | `*c_void` elsewhere; `void_ptr_sites` names each one |

## Notes

### Review is a sub-campaign, not a column

The oracle re-batches whatever it judges under the review pass's own budgets, so
review rows never line up with the wave underneath them. The first half was
emitted by `97` implementation agents under `--max-types 2`; the same units were
judged by `24` review agents under `--max-types 15 --max-syms 150`. That is why
`Batches — review types` and `Batches — review symbols` are their own tables
rather than `rv` columns on the wrap tables, and why no row-for-row mapping
between them exists.

### What the review schedule dropped

The review wave was seeded with the exact `403` landed first-half units. `28` of
them are lifecycle primitives — `git_annotated_commit_free`, `git_buf_dispose`,
`git_config_free` and the like — which the oracle drops from a schedule because
it emits them through their owning type rather than as standalone units. Their
Rust is still judged, inside the owning type's batch, but they carry no row of
their own. A further `2`, `git_cached_obj` and `git_refcount`, are internal types
that `--api-headers-only` does not publish, so they had no match in the API view
and were judged by a separate implementation-anchored wave. Total judged: `373`
in the main pass, `2` in the internal pass, `28` folded into their owners.

### The 529 retry

Two of the main review pass's `21` batches died on `API Error: 529 Overloaded`
on their first turn, having done no work: the `4`-type `git_credential` batch and
a `150`-symbol batch. They were rerun verbatim as `review-first-half-retry` and
both landed clean. Because the rerun re-judged units the main pass had already
counted, the review batch tables sum to more units than the campaign contains —
`119` type-batch units and `410` symbol-batch units over `115` distinct types and
`288` distinct symbols. The Overview divides by the distinct counts.

### Reruns inflate the wrap batch tables too

`first-half-retry-1` rescheduled `377` of the original `403` units after the
first parallel session hit a shared-session ref race at `11` landed batches. No
forward work was discarded, but the same unit appears in two batches, so
`Batches — types, wrap` sums to `140` type-batch units against `115` distinct
types. Per-batch rows are what the agents were actually paid for; the Overview is
what the campaign actually contains.

### Where `loc` is missing

`loc` is the `.rs` insertion count of a batch's landing commit, recovered by
matching each commit to the agent whose run window contains its author
timestamp. `105` of `125` agents resolved that way. The rest read `—`: their
commits could not be attributed unambiguously, mostly in the two sessions where
per-agent branches were pruned after their commits chained onto the shared
session ref. No number was inferred for them, and the Σ rows sum only what
resolved, so a Σ `loc` understates the true total.

### The review moved the tree, then the audit moved it back

The review pass grew the tree — `code_lines` `7304` → `7422`, `unsafe_blocks`
`1286` → `1303`, and `43` new tests — while holding unsafe density flat at
`29.2`%. It also broke three gates that the first-half checkpoint had passed:
`cargo fmt --check` on three files, and `18` clippy errors under `-D warnings`
(`15` × `items_after_test_module` where agents appended items below `mod tests`,
`3` × `drop_non_drop`). Those were mechanical and fixed at landing.

More consequential: the final internal-types batch pushed two categorical targets
off zero — `raw_ptr_in_wrapper` `0` → `1` and `void_ptr_smell` `0` → `1`, both
the same private helper `GitRefcountRef::owner_ptr` returning `*mut c_void` from
inside a wrapper impl. `git_refcount` was rescheduled as a one-unit `wrap`
remediation wave under the implementation model, which contained the pointer
behind the seam. All eight categorical targets read `0` in the after column.

### The internal review changed C

Judging `git_refcount` required callable entry points for `GIT_REFCOUNT_VAL`,
`GIT_REFCOUNT_OWNER` and `GIT_REFCOUNT_OWN`, which are macros with no symbol to
bind. The agent added three `crustify_`-prefixed shims to `src/util/util.c` and
its header. This is the only C change in the campaign, and it is why the C
sanitizer suite was rerun for a review wave: it holds at `3` of `3` retained
CTest targets, matching `build.json`'s recorded baseline.

It also exposed a build-tree split worth recording. `libgit2-sys/build.rs`
link-searches `build-crustify`, the ASan/UBSan build, while the Rust test
binaries must *run* against `build-rust`, the unsanitized one — running them
against `build-crustify` fails with `ASan runtime does not come first`. A C
change is therefore only visible to the Rust suite once *both* trees are
rebuilt.

### Cost is not comparable across the two models

The implementation waves ran `openai/gpt-5.6-sol` on `api` billing; the review
pass ran `anthropic/claude-opus-5` on `subscription`. Every figure here is
computed from per-request token counts against public rates, never from
provider-reported dollars, so the review's `$254.37` is an API-equivalent
comparison value rather than an amount anyone was charged. The orchestrator's own
supervision is unmetered — it does not write `usage.json` — so the Σ row covers
recorded agents only and understates the campaign's true total.

### `crustify-log-cost` could not read this campaign

`layout.py` writes agent logs to `crustify/campaigns/<target>/logs/`, but the
cost reader globbed `crustify/targets/<target>/logs/`, so it matched nothing and
exited on every campaign. Every figure in this report depends on that reader.
Fixed on crustify-cli branch `fix/log-cost-campaigns-dir` at `0b85e55`.

### Second half not started

The campaign covers the first `403` units of the industry-surface closure. The
held remainder is scheduled at `crustify/campaigns/src/second-half.json` — `405`
units in `42` batches across `8` DAG layers — and has not been homed,
translated, reviewed or audited. Nothing in this report anticipates it.

### The UB pass has not run

`crustify-audit ub` requires its own explicit approval, which has not been given.
`crustify/audit/advisories/` does not exist, and every `ub` column reads `—` for
that reason rather than because a run found nothing.
