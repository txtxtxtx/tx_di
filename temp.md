Gnuplot not found, using plotters backend
topo_sort/no_deps_10    time:   [595.94 ns 597.40 ns 599.20 ns]
change: [−4.8517% −3.7102% −2.4164%] (p = 0.00 < 0.05)
Performance has improved.
Found 9 outliers among 100 measurements (9.00%)
4 (4.00%) high mild
5 (5.00%) high severe
topo_sort/no_deps_50    time:   [2.2329 µs 2.2475 µs 2.2647 µs]
change: [−0.9676% +1.9035% +5.0280%] (p = 0.96 > 0.05)
No change in performance detected.
Found 13 outliers among 100 measurements (13.00%)
5 (5.00%) high mild
8 (8.00%) high severe
topo_sort/no_deps_100   time:   [4.4976 µs 4.5085 µs 4.5203 µs]
change: [−20.161% −18.035% −15.644%] (p = 0.00 < 0.05)
Performance has improved.
Found 8 outliers among 100 measurements (8.00%)
4 (4.00%) high mild
4 (4.00%) high severe
topo_sort/chain_10      time:   [874.00 ns 876.15 ns 878.43 ns]
change: [−2.2584% −1.4021% −0.3137%] (p = 0.00 < 0.05)
Change within noise threshold.
Found 3 outliers among 100 measurements (3.00%)
1 (1.00%) high mild
2 (2.00%) high severe
topo_sort/chain_50      time:   [5.5151 µs 5.5931 µs 5.6978 µs]
change: [+6.3981% +11.605% +18.856%] (p = 0.02 < 0.05)
Performance has regressed.
Found 13 outliers among 100 measurements (13.00%)
6 (6.00%) high mild
7 (7.00%) high severe
topo_sort/chain_100     time:   [13.001 µs 13.294 µs 13.598 µs]
change: [−0.5046% +1.4580% +3.3452%] (p = 0.41 > 0.05)
No change in performance detected.
Found 5 outliers among 100 measurements (5.00%)
2 (2.00%) high mild
3 (3.00%) high severe

inject/singleton_u64    time:   [25.394 ns 25.687 ns 26.045 ns]
change: [−7.0515% −2.1737% +1.9434%] (p = 0.23 > 0.05)
No change in performance detected.
Found 11 outliers among 100 measurements (11.00%)
2 (2.00%) high mild
9 (9.00%) high severe
inject/singleton_large_object
time:   [25.277 ns 25.418 ns 25.591 ns]
change: [−8.5777% −4.0117% +0.2224%] (p = 0.03 < 0.05)
Change within noise threshold.
Found 3 outliers among 100 measurements (3.00%)
1 (1.00%) high mild
2 (2.00%) high severe
inject/prototype_factory
time:   [52.585 ns 52.795 ns 53.048 ns]
change: [−5.4672% −3.0057% −0.4520%] (p = 0.00 < 0.05)
Change within noise threshold.
Found 6 outliers among 100 measurements (6.00%)
2 (2.00%) high mild
4 (4.00%) high severe
inject/lookup_miss      time:   [14.966 ns 15.320 ns 15.864 ns]
change: [−1.3238% +0.5957% +3.2525%] (p = 0.76 > 0.05)
No change in performance detected.
Found 10 outliers among 100 measurements (10.00%)
4 (4.00%) high mild
6 (6.00%) high severe
inject/multi_type_10_keys
time:   [25.903 ns 26.878 ns 27.993 ns]
change: [−2.1663% −0.1766% +2.2903%] (p = 0.75 > 0.05)
No change in performance detected.
Found 17 outliers among 100 measurements (17.00%)
5 (5.00%) high mild
12 (12.00%) high severe

concurrent/read_only/2_threads
time:   [142.70 µs 145.05 µs 148.32 µs]
change: [−1.6353% −0.4152% +0.8249%] (p = 0.44 > 0.05)
No change in performance detected.
Found 5 outliers among 100 measurements (5.00%)
3 (3.00%) high mild
2 (2.00%) high severe
concurrent/read_only/4_threads
time:   [331.38 µs 337.26 µs 343.04 µs]
change: [−6.2869% −2.7504% +0.8226%] (p = 0.22 > 0.05)
No change in performance detected.
Found 5 outliers among 100 measurements (5.00%)
1 (1.00%) low severe
2 (2.00%) low mild
2 (2.00%) high mild
concurrent/read_only/8_threads
time:   [700.10 µs 716.43 µs 732.29 µs]
change: [−3.8046% +1.7356% +7.7120%] (p = 0.54 > 0.05)
No change in performance detected.
Found 3 outliers among 100 measurements (3.00%)
3 (3.00%) low mild
concurrent/read_write/2_threads
time:   [156.68 µs 157.92 µs 159.34 µs]
change: [−5.3697% −4.1246% −2.6393%] (p = 0.00 < 0.05)
Performance has improved.
Found 6 outliers among 100 measurements (6.00%)
4 (4.00%) high mild
2 (2.00%) high severe
concurrent/read_write/4_threads
time:   [277.11 µs 277.81 µs 278.60 µs]
change: [−6.9155% −6.4814% −6.0167%] (p = 0.00 < 0.05)
Performance has improved.
Found 16 outliers among 100 measurements (16.00%)
1 (1.00%) low mild
8 (8.00%) high mild
7 (7.00%) high severe
concurrent/read_write/8_threads
time:   [541.47 µs 549.60 µs 558.98 µs]
change: [−6.6137% −5.0906% −3.8077%] (p = 0.00 < 0.05)
Performance has improved.
Found 11 outliers among 100 measurements (11.00%)
8 (8.00%) high mild
3 (3.00%) high severe

comp_ref/cached_clone   time:   [9.5611 ns 9.5907 ns 9.6265 ns]
change: [−2.9482% −2.2787% −1.4998%] (p = 0.00 < 0.05)
Performance has improved.
Found 13 outliers among 100 measurements (13.00%)
4 (4.00%) low mild
3 (3.00%) high mild
6 (6.00%) high severe
comp_ref/cached_downcast
time:   [9.8429 ns 10.027 ns 10.246 ns]
change: [+2.8451% +4.2850% +5.8767%] (p = 0.00 < 0.05)
Performance has regressed.
Found 8 outliers among 100 measurements (8.00%)
3 (3.00%) high mild
5 (5.00%) high severe
comp_ref/factory_call   time:   [38.861 ns 39.441 ns 40.303 ns]
change: [−2.2848% −0.5145% +1.6244%] (p = 0.45 > 0.05)
No change in performance detected.
Found 15 outliers among 100 measurements (15.00%)
6 (6.00%) high mild
9 (9.00%) high severe
comp_ref/dashmap_insert_get
time:   [67.699 ns 68.248 ns 68.987 ns]
Found 10 outliers among 100 measurements (10.00%)
3 (3.00%) high mild
7 (7.00%) high severe
comp_ref/dashmap_get_existing
time:   [14.952 ns 15.307 ns 15.708 ns]
Found 13 outliers among 100 measurements (13.00%)
3 (3.00%) high mild
10 (10.00%) high severe

async_runtime/tokio_spawn
time:   [19.445 µs 19.587 µs 19.718 µs]
async_runtime/cancellation_token_clone
time:   [43.558 ns 43.899 ns 44.380 ns]
Found 8 outliers among 100 measurements (8.00%)
2 (2.00%) high mild
6 (6.00%) high severe
async_runtime/cancellation_check
time:   [15.093 ns 15.115 ns 15.142 ns]
Found 10 outliers among 100 measurements (10.00%)
6 (6.00%) high mild
4 (4.00%) high severe
async_runtime/arc_clone/1_threads
time:   [129.46 µs 130.27 µs 131.12 µs]
Found 1 outliers among 100 measurements (1.00%)
1 (1.00%) high mild
async_runtime/arc_clone/2_threads
time:   [210.72 µs 213.10 µs 216.14 µs]
Found 13 outliers among 100 measurements (13.00%)
3 (3.00%) high mild
10 (10.00%) high severe
async_runtime/arc_clone/4_threads
time:   [490.14 µs 521.95 µs 551.99 µs]
Found 15 outliers among 100 measurements (15.00%)
7 (7.00%) low severe
1 (1.00%) low mild
4 (4.00%) high mild
3 (3.00%) high severe
