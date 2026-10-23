# Disk and network throughput (roadmap item 15)

System-wide disk read / write and network receive / send throughput: four new metric boxes in
the terminal UI, four gauges in Prometheus, and a `disk` / `network` object in the JSON. Rates are
bytes per second over the sampling interval. Aim ≤ 200 production lines (`make loc REF=origin/main`),
stop and report past it.

## Behavior

```
╭─ Apple M3 Pro (6E+6P+18GPU 36GB) ───────────────────────────────────── macmon v0.11.0 ─╮
│ ╭─ E-CPU 42% @ 1800 MHz ─╮ ╭─ P-CPU 77% @ 3200 MHz ─╮ ╭─ GPU 23% @ 1400 MHz ─╮ …     │
│ ╭─ CPU 4.50W (4.50, 4.50) ─╮ ╭─ GPU 2.00W ─╮ ╭─ ANE 0.10W ─╮ ╭─ Total 12.00W ─╮       │
│ ╭─ DISK R 12.3 MB/s ─╮ ╭─ DISK W 4.5 MB/s ─╮ ╭─ NET ↓ 1.2 MB/s ─╮ ╭─ NET ↑ 80 KB/s ─╮ │
```

- A third row of four boxes under the power row, always present: disk read, disk write, network
  receive, network send. Each box shows the current rate in its title over a graph of its history
  scaled to the largest visible sample, in the low load color (like the power boxes).
- Narrow boxes use `D-R`, `D-W`, `NET↓`, `NET↑` titles; the rate drops to a whole unit only when
  the box is narrower than the title.
- Without a power row (a VM) the top row and the I/O row split the metrics box.
- JSON gains `disk` (`read_bytes`, `write_bytes`) and `network` (`rx_bytes`, `tx_bytes`), all bytes
  per second. Prometheus gains `macmon_disk_read_bytes_per_second`,
  `macmon_disk_write_bytes_per_second`, `macmon_network_rx_bytes_per_second` and
  `macmon_network_tx_bytes_per_second`.

## Decisions

- Disk counters: sum of the `Statistics` `Bytes (Read)` / `Bytes (Write)` of every
  `IOBlockStorageDriver` service, read through the existing IORegistry helpers.
- Network counters: `getifaddrs` `if_data` of every non-loopback interface, kept per interface so a
  32-bit counter wrap is a per-interface wrapping difference. Per-interface counters also avoid
  losing an interface that disappears between samples.
- The sampler keeps the counters and timestamp of the previous sample: the first sample is 0 B/s,
  later rates divide by the measured wall time between counter reads.
- Library API is additive: `Metrics` gains `disk: DiskMetrics` and `network: NetMetrics`.
- Boxes use decimal units (`kB/s`, `MB/s`, `GB/s`) with one decimal above 1 kB/s, as disk and
  network speeds are normally quoted.

## Tasks

- [x] Sources and metrics: `disk_io_bytes`, `net_io_counters`, `IoCounters::rates`, `DiskMetrics`,
  `NetMetrics`, sampler baseline and rates; unit tests for counter wrap and rates.
- [x] JSON and Prometheus: fields and gauges, serve test count, readme JSON and example.
- [x] TUI: `Metric::DiskRead` / `DiskWrite` / `NetIn` / `NetOut`, third layout row, history store,
  rate formatting and titles; update layout and render tests; roadmap item 15 links this plan.
- [x] `make check` and `make test`; runtime check with `macmon pipe` and the TUI.
