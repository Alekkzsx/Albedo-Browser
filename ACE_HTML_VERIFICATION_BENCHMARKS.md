# ACE-HTML Verification Benchmarks

- Generated at: 2026-04-26T22:06:25.778Z
- Host platform: linux
- Node: v24.11.1

## Dataset

- Required tree cases: 14
- html5lib tree cases (ACE full): 1498
- html5lib tree sample cases (browsers): 100
- Tokenizer subset cases: 20
- Perf HTML bytes: 1200000

## Results

### ACE-HTML (Albedo)

| Métrica | Required Tree | html5lib Full | Tokenizer Subset |
|---|---:|---:|---:|
| Total casos | 14 | 1498 | 20 |
| Passou | 14 | 1498 | 18 |
| Pass rate (%) | 100.00 | 100.00 | 90.00 |
| Tempo médio por caso (ms) | 0.063 | 0.043 | 0.017 |

| Performance parser | Valor |
|---|---:|
| Avg parse (ms) | 135.358 |
| Stddev parse (ms) | 10.130 |
| p50 parse (ms) | 131.856 |
| p95 parse (ms) | 155.001 |
| p99 parse (ms) | 159.816 |
| Throughput médio (MB/s) | 8.911 |

### ACE-HTML Memory Profile

| Métrica | Valor |
|---|---:|
| Cases parsed | 1,000 |
| Elapsed (ms) | 176.715 |
| Large HTML bytes | 988,962 |
| Alloc calls | 265,016 |
| Realloc calls | 851 |
| Dealloc calls | 262,988 |
| Total allocated bytes | 35,892,186 |
| Peak live bytes | 26,960,980 |
| Live bytes at end | 2,103,566 |
| Avg allocated bytes/case | 35892.2 |

### Chrome

- Executável: `/usr/bin/google-chrome`

| Métrica | Required Tree | html5lib Sample |
|---|---:|---:|
| Total casos | 14 | 100 |
| Passou | 14 | 99 |
| Pass rate (%) | 100.00 | 99.00 |
| Tempo médio por caso (ms) | 13.143 | 9.220 |
| p95 por caso (ms) | 23.000 | 15.000 |

| Performance parser | Valor |
|---|---:|
| Avg parse (ms) | 211.327 |
| Stddev parse (ms) | 40.632 |
| p50 parse (ms) | 212.580 |
| p95 parse (ms) | 282.370 |
| p99 parse (ms) | 282.370 |
| Throughput médio (MB/s) | 5.895 |

Falhas amostrais:
- `adoption01.dat#17`


### Firefox

- Executável: `/usr/bin/firefox`

| Métrica | Required Tree | html5lib Sample |
|---|---:|---:|
| Total casos | 14 | 100 |
| Passou | 14 | 100 |
| Pass rate (%) | 100.00 | 100.00 |
| Tempo médio por caso (ms) | 95.429 | 34.050 |
| p95 por caso (ms) | 212.000 | 104.000 |

| Performance parser | Valor |
|---|---:|
| Avg parse (ms) | 387.405 |
| Stddev parse (ms) | 79.434 |
| p50 parse (ms) | 391.600 |
| p95 parse (ms) | 585.850 |
| p99 parse (ms) | 585.850 |
| Throughput médio (MB/s) | 3.209 |


### Brave

- Executável: `/usr/bin/brave-browser`

| Métrica | Required Tree | html5lib Sample |
|---|---:|---:|
| Total casos | 14 | 100 |
| Passou | 14 | 99 |
| Pass rate (%) | 100.00 | 99.00 |
| Tempo médio por caso (ms) | 16.500 | 12.280 |
| p95 por caso (ms) | 19.000 | 18.000 |

| Performance parser | Valor |
|---|---:|
| Avg parse (ms) | 155.463 |
| Stddev parse (ms) | 26.643 |
| p50 parse (ms) | 149.215 |
| p95 parse (ms) | 208.050 |
| p99 parse (ms) | 208.050 |
| Throughput médio (MB/s) | 7.918 |

Falhas amostrais:
- `adoption01.dat#17`


## Quick Comparison

| Engine | Required Tree (%) | html5lib (%) | Throughput (MB/s) |
|---|---:|---:|---:|
| ACE-HTML | 100.00 | 100.00 | 8.911 |
| Chrome | 100.00 | 99.00 | 5.895 |
| Firefox | 100.00 | 100.00 | 3.209 |
| Brave | 100.00 | 99.00 | 7.918 |

## Raw Files

- `ace_html_super_benchmark_results.json`
- `ace_html_memory_profile_results.json`
- `browser_html_benchmark_results.json`
