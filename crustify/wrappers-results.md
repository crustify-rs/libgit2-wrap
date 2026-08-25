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
- **`--max-types`** — `4`
- **`--max-syms`** — `50`
- **`--max-loc`** — `1000`
- **`--min-fields`** — `20`
- **`--parallel-max`** — `16`
- **branch** — `crustify/src-gpt-5.6-sol`, tip `3d930bef6`
- **deps** — crustify-cli `51d44d1` (`docs/results-template-ub`), ffibox `600399f` (`main`)

## Review pass

`--objective review`, LLM-as-a-Judge over the landed waves.

- **agent backend** — `claude`
- **model** — `anthropic/claude-opus-5`
- **`--billing`** — `subscription`
- **`--max-types`** — `15`
- **`--max-syms`** — `150`
- **`--max-loc`** — `3000`
- **`--min-fields`** — `60`
- **`--parallel-max`** — `16`
- **branch** — `crustify/src-gpt-5.6-sol`, tip `3d930bef6`
- **agents** — `44`, over `5` session(s); both halves

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

- **Rust LoC, non-test** — `18,452`
- **Rust LoC, tests** — `10,231`
- **C LoC** — `171,084`
- **ported types** — `0`
- **ported symbols** — `0`
- **wrapped types** — `170` (`64.6`% of API)
- **wrapped symbols** — `638` (`63.4`% of API)
- **remaining types** — `92` with no anchor
- **remaining symbols** — `387` with no anchor

Implementation `openai/gpt-5.6-sol` via `codex`; review `anthropic/claude-opus-5`
via `claude`. Each row names the model that produced it.

| sub-campaign | objective | nr types | nr symbols | session wall | total | $/type | $/sym | ub wall | ub $ |
|---|---|---:|---:|---|---:|---:|---:|---|---:|
| `2-raw-lifetime` | raw lifetime | `0` | `3` | `16m37s` | `$9.85` (`gpt-5.6-sol`) | — | `$3.28` | — | — |
| `2-raw-lifetime-review` | review | `0` | `3` | `14m32s` | `$7.54` (`gpt-5.6-sol`) | — | `$2.51` | — | — |
| `5-first-half` | wrap | `115` | `288` | `3h14m00s` | `$362.45` (`gpt-5.6-sol`) | `$2.68` | `$0.19` | — | — |
| `4-review-first-half` | review | `115` | `288` | `1h20m44s` | `$257.58` (`claude-opus-5`) | `$1.91` | `$0.13` | — | — |
| `4-second-half` | wrap | `55` | `350` | `3h16m09s` | `$303.91` (`gpt-5.6-sol`) | `$3.40` | `$0.33` | — | — |
| `1-review-second-half` | review | `55` | `350` | `2h39m47s` | `$198.27` (`claude-opus-5`) | `$1.47` | `$0.34` | — | — |
| orchestrator | orchestration | `—` | `—` | — | `not metered`+ (`claude-opus-5`) | — | — | — | — |
| **Σ recorded agents** | | **`170`** | **`638`** | **`11h01m49s`** | **`$1,139.61`** | **`$4.68`** | **`$0.54`** | | **—** |

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
| `2` | `0` | `0` | `$3.95` | `11m46s` | `$1.97` | `—` |
| `2` | `0` | `0` | `$3.29` | `6m41s` | `$1.64` | `—` |
| `2` | `0` | `0` | `$3.96` | `7m26s` | `$1.98` | `—` |
| `2` | `0` | `0` | `$3.58` | `6m01s` | `$1.79` | `—` |
| `2` | `0` | `0` | `$4.39` | `7m57s` | `$2.19` | `—` |
| `2` | `0` | `0` | `$2.55` | `4m28s` | `$1.27` | `—` |
| `2` | `4` | `0` | `$4.84` | `10m12s` | `$2.42` | `$1.21` |
| `2` | `0` | `0` | `$4.65` | `9m00s` | `$2.33` | `—` |
| `2` | `4` | `0` | `$3.49` | `6m44s` | `$1.74` | `$0.87` |
| `2` | `5` | `0` | `$4.75` | `10m06s` | `$2.38` | `$0.95` |
| `2` | `20` | `0` | `$5.21` | `9m11s` | `$2.60` | `$0.26` |
| `2` | `5` | `0` | `$3.89` | `7m37s` | `$1.95` | `$0.78` |
| `2` | `9` | `0` | `$3.11` | `7m35s` | `$1.56` | `$0.35` |
| `2` | `13` | `0` | `$5.94` | `13m50s` | `$2.97` | `$0.46` |
| `2` | `12` | `0` | `$5.11` | `11m33s` | `$2.56` | `$0.43` |
| `2` | `16` | `0` | `$9.38` | `22m00s` | `$4.69` | `$0.59` |
| `2` | `11` | `0` | `$5.34` | `10m26s` | `$2.67` | `$0.49` |
| `1` | `20` | `0` | `$6.70` | `14m19s` | `$6.70` | `$0.34` |
| `2` | `12` | `0` | `$7.10` | `12m02s` | `$3.55` | `$0.59` |
| `2` | `23` | `0` | `$8.97` | `15m50s` | `$4.48` | `$0.39` |
| `2` | `9` | `0` | `$9.43` | `13m08s` | `$4.71` | `$1.05` |
| `1` | `14` | `0` | `$4.61` | `8m58s` | `$4.61` | `$0.33` |
| `2` | `9` | `0` | `$6.16` | `13m41s` | `$3.08` | `$0.68` |
| `2` | `17` | `0` | `$6.90` | `11m39s` | `$3.45` | `$0.41` |
| `2` | `22` | `0` | `$10.12` | `16m00s` | `$5.06` | `$0.46` |
| `2` | `10` | `0` | `$4.49` | `8m29s` | `$2.25` | `$0.45` |
| `2` | `16` | `0` | `$5.56` | `9m46s` | `$2.78` | `$0.35` |
| `1` | `3` | `0` | `$3.71` | `6m53s` | `$3.71` | `$1.24` |
| `2` | `14` | `0` | `$6.94` | `12m40s` | `$3.47` | `$0.50` |
| `1` | `3` | `0` | `$2.45` | `4m30s` | `$2.45` | `$0.82` |
| `2` | `0` | `2` | `$4.56` | `8m10s` | `$2.28` | `—` |
| `1` | `0` | `1` | `$3.02` | `5m49s` | `$3.02` | `—` |
| `2` | `5` | `2` | `$4.13` | `6m22s` | `$2.07` | `$0.83` |
| `2` | `21` | `0` | `$5.90` | `8m47s` | `$2.95` | `$0.28` |
| `1` | `4` | `0` | `$2.28` | `4m56s` | `$2.28` | `$0.57` |
| `1` | `0` | `2` | `$2.87` | `4m22s` | `$2.87` | `—` |
| `1` | `14` | `0` | `$3.48` | `5m33s` | `$3.48` | `$0.25` |
| **Σ `205`** | **`516`** | **`64`** | **`$495.08`** | — | **`$2.42`** | **`$0.96`** |

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
| `1` | `+5/-4` | `$3.21` | `5m28s` | `$3.21` |
| `15` | `+49/-2` | `$9.16` | `11m19s` | `$0.61` |
| `1` | `+27/-1` | `$2.91` | `7m03s` | `$2.91` |
| `4` | `+190/-25` | `$7.88` | `13m54s` | `$1.97` |
| `11` | `+164/-27` | `$11.50` | `14m50s` | `$1.05` |
| `3` | `+307/-1` | `$7.63` | `12m09s` | `$2.54` |
| `6` | `+166/-6` | `$8.94` | `11m55s` | `$1.49` |
| `2` | `+154/-10` | `$11.09` | `15m40s` | `$5.55` |
| `8` | `+323/-42` | `$11.77` | `15m21s` | `$1.47` |
| `3` | `+157/-3` | `$5.99` | `10m24s` | `$2.00` |
| `2` | `+97/-2` | `$4.00` | `6m27s` | `$2.00` |
| **Σ `175`** | **`+4021/-486`** | **`$300.98`** | — | **`$1.72`** |

### Batches — symbols

`openai/gpt-5.6-sol` via `codex`. One row per batch, in execution order.

| objective | symbols | loc | $ | wall | $/symbol | $/loc |
|---|---|---|---|---|---|---|
| raw lifetime | `1` | `100` | `$5.56` | `9m30s` | `$5.56` | `$0.06` |
| raw lifetime | `1` | `59` | `$4.29` | `7m07s` | `$4.29` | `$0.07` |
| wrap | `20` | `811` | `$7.58` | `16m07s` | `$0.38` | `$0.01` |
| wrap | `50` | `938` | `$7.52` | `16m49s` | `$0.15` | `$0.01` |
| wrap | `41` | `807` | `$12.63` | `24m47s` | `$0.31` | `$0.02` |
| wrap | `50` | `881` | `$6.41` | `19m35s` | `$0.13` | `$0.01` |
| wrap | `50` | `915` | `$7.06` | `14m54s` | `$0.14` | `$0.01` |
| wrap | `50` | `497` | `$6.84` | `14m36s` | `$0.14` | `$0.01` |
| wrap | `22` | `715` | `$6.14` | `11m04s` | `$0.28` | `$0.01` |
| wrap | `1` | `392` | `$5.38` | `13m02s` | `$5.38` | `$0.01` |
| wrap | `50` | `787` | `$9.72` | `22m12s` | `$0.19` | `$0.01` |
| wrap | `46` | `936` | `$9.05` | `16m57s` | `$0.20` | `$0.01` |
| wrap | `50` | `687` | `$8.11` | `16m29s` | `$0.16` | `$0.01` |
| wrap | `46` | `894` | `$10.94` | `20m04s` | `$0.24` | `$0.01` |
| wrap | `50` | `1476` | `$12.88` | `31m52s` | `$0.26` | `$0.01` |
| wrap | `14` | `1202` | `$8.18` | `15m10s` | `$0.58` | `$0.01` |
| wrap | `28` | `670` | `$11.51` | `25m37s` | `$0.41` | `$0.02` |
| wrap | `25` | `655` | `$10.39` | `22m21s` | `$0.42` | `$0.02` |
| wrap | `11` | `819` | `$11.58` | `19m30s` | `$1.05` | `$0.01` |
| wrap | `22` | `723` | `$7.25` | `13m02s` | `$0.33` | `$0.01` |
| wrap | `3` | `125` | `$5.49` | `9m07s` | `$1.83` | `$0.04` |
| wrap | `4` | `184` | `$6.63` | `12m22s` | `$1.66` | `$0.04` |
| **Σ** | **`636`** | **`15,273`** | **`$181.14`** | | **`$0.28`** | **`$0.01`** |

### Batches — review symbols

`anthropic/claude-opus-5` via `claude`. One row per batch, in execution order.

| symbols | rv loc | rv $ | rv wall | rv $/symbol |
|---|---|---|---|---|
| `1` | `+11/-5` | `$3.69` | `7m01s` | `$3.69` |
| `1` | `+7/-1` | `$3.85` | `7m31s` | `$3.85` |
| `20` | `+83/-27` | `$7.02` | `11m03s` | `$0.35` |
| `150` | — | — | `3m35s` | — |
| `90` | `+39/-1` | `$10.63` | `13m35s` | `$0.12` |
| `150` | `+170/-25` | `$19.81` | `19m43s` | `$0.13` |
| `1` | `+61/-2` | `$5.23` | `10m43s` | `$5.23` |
| `96` | `+138/-29` | `$15.34` | `25m10s` | `$0.16` |
| `150` | `+166/-16` | `$15.30` | `19m06s` | `$0.10` |
| `10` | `+142/-2` | `$7.34` | `12m29s` | `$0.73` |
| `27` | `+276/-19` | `$15.46` | `21m37s` | `$0.57` |
| `35` | `+272/-44` | `$20.47` | `26m16s` | `$0.58` |
| `22` | `+301/-29` | `$18.22` | `22m37s` | `$0.83` |
| `3` | `+283/-30` | `$11.21` | `19m11s` | `$3.74` |
| `4` | `+335/-3` | `$8.83` | `14m23s` | `$2.21` |
| **Σ `761`** | **`+2284/-233`** | **`$162.42`** | — | **`$0.21`** |

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

### The settings blocks are forward-looking

`Campaign` and `Review pass` record the caps the campaign is configured with,
not a per-wave history. The waves that have already run used narrower ones: the
first half `--max-types 2 --min-fields 10 --parallel-max 8`, dropping to `4`
then `2` concurrent for its two remediation waves; the second half
`--max-types 2 --min-fields 20 --parallel-max 16`, the `--min-fields` move
being the oracle's default changing under it rather than a deliberate choice.
The first-half review ran `--min-fields 30`, the second-half review `60`.
Per-batch unit counts in the tables below reflect what each wave actually
packed.

### How test LoC is counted

`Rust LoC` is `crustify-audit`'s `code_lines` — the union of HIR definition
spans. `cfg` stripping happens before HIR, so an inline `mod tests` contributes
nothing to it, and nothing to any other audit metric: test code puts neither its
lines in the denominator of the unsafe ratio nor its `unsafe` blocks in the
numerator. Every safety figure in this report is therefore about shipped code
only.

That also means the tool publishes no test-line metric, so `test LoC` is
measured separately: non-blank, non-comment lines inside `#[cfg(test)]` modules,
counted by brace-matching each one. On that same basis the non-test tree is
`21,272` lines, which is why the share reads `32.5`% rather than
`10,231 / 18,452` — the two numbers in the Overview come from different
instruments and should not be divided into each other.

There are no integration test directories; all `560` tests live in the `193`
inline modules.

### What the coverage percentages measure

The Overview's unit counts and its coverage percentages are taken on different
bases, and they do not reconcile line for line.

`wrapped types` / `wrapped symbols` count SCHEDULED UNITS — what agents were
given worklists for. The percentages count ANCHORS against
`query {types,symbols} --api-only`, the surface `include/` publishes. The two
differ in both directions. Two scheduled types, `git_cached_obj` and
`git_refcount`, are internal and not published at all, so they raise the unit
count without touching the denominator. In the other direction `637` published
functions carry an anchor against `605` scheduled function units, because a
type batch anchors the lifecycle primitives and accessors that belong to its
type without those being units of their own.

Macros are excluded from the denominator entirely. `conventions.md` puts their
whole surface in the `-sys` crate — a generated `pub const` or a
`crustify_<NAME>` shim — and no `.rs` anchor is ever laid for one, so counting
libgit2's `157` published macros as unwrapped would measure a decision rather
than a gap.

### What the remaining `479` are

`477` of them were never scheduled: the campaign's declared coverage was the
surface reached by the industry safe wrapper crate, not the whole API. Measured
against that target the campaign is `790` of `792` — and the two it appears to
miss, `git_oidarray_free` and `git_strarray_free`, are deprecated aliases whose
types are wrapped through the current spelling, `git_oidarray_dispose` and
`git_strarray_dispose`, both bound with `ffibox::impl_cvalued!`. Against its own
scope the campaign is complete.

The unscheduled remainder is not uniform. `149` sit in `include/git2/sys/*`,
the backend-authoring interfaces a consumer never calls — custom ODB, refdb,
transport, filter and stream implementations. `42` are deprecated spellings.
The other `286` are mainstream API that git2-rs simply does not bind, densest in
`index.h` (`19`), `oid.h` (`17`), and `commit.h`, `filter.h` and `repository.h`
(`12` each). That `286` is the natural next scope; the `sys/` and deprecated
sets are deliberate exclusions rather than backlog.

### Review is a sub-campaign, not a column

The oracle re-batches whatever it judges under the review pass's own budgets, so
review rows never line up with the wave underneath them. The first half was
emitted by `96` implementation agents under `--max-types 2`; the same units were
judged by `25` review agents under `--max-types 15 --max-syms 150`. That is why
the review batches are their own tables rather than `rv` columns on the wrap
tables, and why no row-for-row mapping between them exists.

### Both halves are reviewed, under different caps

The first half was judged at `--max-types 15 --max-syms 150 --min-fields 30`,
the second at the same types and symbol caps but `--min-fields 60`. The wider
floor packs harder: `19` batches covered the second half's `403` judged units
against `21` batches for the first half's `373`.

The two passes did not behave alike. The first-half review broke three gates it
inherited green — `18` clippy errors and three unformatted files — and pushed
two categorical audit targets off zero, which cost a further remediation wave.
The second-half review broke only formatting, in seven files, and moved no audit
target at all: the non-seam raw-pointer remainder held at exactly `15` and
unsafe density was flat, `26.76`% to `26.71`%. It added `48` tests.

Each review drops the lifecycle primitives its schedule would otherwise
duplicate — `28` from the first half, `2` from the second
(`git_config_iterator_free`, `git_odb_object_free`). Those units are judged
inside their owning type's batch and carry no row of their own, which is why a
review's judged count is below the wave's emitted count.

### Campaign-end audit

The `Safety audit` table brackets the review pass, so its "after" column is the
tree at `d71102b84` — before the second half existed. The campaign's final tree
at `3d930bef6` measures:

| metric | after first-half review (`d71102b84`) | campaign end (`3d930bef6`) |
|---|---|---|
| `code_lines` | `7,423` | `18,508` |
| unsafe loc | `2,164` | `4,944` |
| % of loc | `29.15`% | `26.71`% |
| `unsafe_blocks` | `1,303` | `2,769` |
| `unsafe_fns` / seam | `498` / `398` | `809` / `638` |
| `ffi_calls` | `366` | `703` |
| `wrapper_newtypes` / declared | `79` / `79` | `120` / `120` |
| raw-ptr positions / seam | `565` / `557` | `952` / `937` |
| **non-seam remainder** | **`8`** | **`15`** |

The tree grew `2.5x` while unsafe density FELL, from
`29.15`% to `26.71`%.

### Seven of eight categorical targets end at zero

`wrapper_declared_nonconformant`, `wrapper_newtypes_undeclared`,
`raw_ptr_wrapped`, `ref_to_type_wrapper`, `field_ref_wrapped`,
`field_proj_outside_impl` and `void_ptr_smell` all read `0` at `3d930bef6`.
`raw_ptr_in_wrapper` reads `1`.

The second half's first merged scan was much worse — `raw_ptr_wrapped` `8`,
`raw_ptr_in_wrapper` `4`, `void_ptr_smell` `4`, non-seam remainder `27`. Every
site was one of two shapes. The inbound `*_result` helpers took a raw pointer C
had just returned and built an owned wrapper from it; the outbound helpers took
`Option<Wrapper>` and produced the raw pointer an FFI argument needed. Two
translator remediation waves (`7` batches, `38m`) confined both: the first
cleared the inbound helpers and the two void-pointer cases, the second moved the
outbound marshalling to the call site.

The residual `raw_ptr_in_wrapper: 1` was left deliberately. `raw_ptr_wrapped` is
`0`, so its pointee is not a wrapped C type — the README calls the wrapped-pointee
count "the actionable defect". More practically, `crustify-audit` publishes no
site list for `raw_ptr_in_wrapper`: `raw_ptr_sites` carries only the
wrapped-pointee subset and is empty, so the tool reports the count without
saying where. A third speculative wave against an unlocatable position was not
worth its cost.

### One second-half unit landed unanchored

`git_diff_binary` was scheduled, and its agent emitted a `297`-line wrapper and
reported landing `c2b1f77e3`. Two step-2 agents wrote to `api/diff.rs`, and
forward-only landing kept the one that wrapped `git_diff_delta`; the other's
commit never reached the branch. Cherry-picking it conflicted in exactly that
region, so it was rerun as its own one-unit wave and landed clean.

The TODO-anchor count did not catch this. It read `0` both before and after,
because the scheduler's TODO was consumed even though no filled anchor replaced
it. What caught it was verifying every unit name in the wave document against
the anchors in the tree, which is now the completion check rather than the TODO
count.

### Five first-half units carry no anchor of their own

`git_annotated_commit_free`, `git_blame_free`, `git_branch_iterator_free`,
`git_buf_dispose` and `git_config_free` are lifecycle primitives realized as
`ffibox` bindings — `impl_cvalued!` or a `CDropped` impl on the owning type,
each with its own SAFETY justification — rather than as standalone `Wraps:`
anchors. Verified individually. Campaign coverage is `808` of `808`.

### Reruns inflate the batch tables

Per-batch rows count what agents were paid for, which exceeds the campaign's
distinct units. `first-half-retry-1` rescheduled `377` of the original `403`
after a shared-session ref race at `11` landed batches, and the review retry
re-judged `154` units after two `529` failures. The Overview divides by distinct
units; the batch tables do not.

### Where `loc` is missing

`loc` is the `.rs` insertion count of a batch's landing commit, recovered by
matching each commit to the agent whose run window contains its author
timestamp — agent branches are unreliable, since they are pruned once their
commits chain onto the session ref and same-stem batches collide on name.
`154` of `175` agents resolved. The rest read `—`; no number was inferred, so
Σ `loc` understates.

### Gates the agents broke, twice

Both the review pass and the second half landed code that failed the gates the
previous checkpoint had passed, in the same two ways: `items_after_test_module`
(`15` after the review, `13` after the second half) where agents appended items
below `mod tests`, and a SAFETY comment attached to an `assert_eq!` whose
argument held the `unsafe` block, which clippy cannot see. The second half also
shipped a `revwalk` test opening `"../.."`, which resolves to `crustify/` rather
than the repository root and returned `GIT_ENOTFOUND` outside an agent worktree.
All were fixed at landing. Agents validate inside their own worktree, where the
relative path and the pre-move file layout both happen to work.

### The internal review changed C

Judging `git_refcount` required callable entry points for `GIT_REFCOUNT_VAL`,
`GIT_REFCOUNT_OWNER` and `GIT_REFCOUNT_OWN`, which are macros with no symbol to
bind, so the agent added three `crustify_`-prefixed shims to `src/util/util.c`
and its header. This is the campaign's only C change. The retained sanitizer
suite holds at `3` of `3`, matching `build.json`'s baseline.

It also exposed a build-tree split: `libgit2-sys/build.rs` link-searches
`build-crustify`, the ASan/UBSan build, while the Rust test binaries must RUN
against `build-rust`, the unsanitized one — running them against
`build-crustify` fails with `ASan runtime does not come first`. A C change is
visible to the Rust suite only once both trees are rebuilt.

### Cost is not comparable across the two models

Implementation waves ran `openai/gpt-5.6-sol` on `api` billing; the review pass
ran `anthropic/claude-opus-5` on `subscription`. Every figure is computed from
per-request token counts against public rates, never from provider-reported
dollars, so the review's `$257.58` is an API-equivalent comparison value rather
than an amount anyone was charged. The orchestrator's own supervision is
unmetered — it writes no `usage.json` — so the Σ row covers recorded agents only
and understates the campaign total.

### `crustify-log-cost` could not read this campaign

`layout.py` writes agent logs to `crustify/campaigns/<target>/logs/`, but the
cost reader globbed `crustify/targets/<target>/logs/`, so it matched nothing and
exited on every campaign. Every figure in this report depends on that reader.
Fixed on crustify-cli branch `fix/log-cost-campaigns-dir` at `0b85e55`.

### A placement key that never matched its file

`crates.json` keyed the packbuilder home as `src/pack-objects.rs`, mirroring the
C translation unit, while the Rust file has always been `pack_objects.rs`.
`crates validate` accepted it, because it checks placement consistency rather
than the filesystem. The inconsistency only surfaced when second-half scaffolding
turned that key into `pub mod pack-objects;` — not a legal Rust module name.
Corrected while homing the second half.

### The UB pass has not run

`crustify-audit ub` requires its own explicit approval, which has not been given.
`crustify/audit/advisories/` does not exist, and every `ub` column reads `—` for
that reason rather than because a run found nothing.

