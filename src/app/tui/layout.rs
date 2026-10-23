//! Screen layout: the metric boxes of the original macmon in the top part of the screen and the
//! process list in the rest.
//!
//! The metrics box holds three rows of boxes: one per CPU cluster, then GPU and RAM on top, CPU /
//! GPU / ANE / total power below them, and disk read / write with network receive / send at the
//! bottom. The boxes of a row split its width evenly, the rows split the height (the upper rows
//! get the odd cells). One structure at every size, only the scale changes. Without power sources
//! (a VM) the power row is left out, and the top and I/O rows split the height.

use ratatui::layout::{Margin, Rect};

/// Share of the screen height for the metrics box over the process list, in percent.
const METRICS_HEIGHT_PCT: u32 = 40;
/// Lowest metrics box over the process list: three rows of boxes with one graph row each, plus the
/// borders.
const METRICS_MIN_HEIGHT: u16 = 11;
/// Fewest process rows worth showing; with less room the process list is auto-hidden.
const PROC_MIN_ROWS: u16 = 3;
/// Rows of the process box besides the process rows: borders and the table header.
const PROC_CHROME: u16 = 3;

/// One box of the metrics box: a title over a history graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Metric {
  /// CPU cluster, by index into the cluster list.
  Cluster(usize),
  Gpu,
  Ram,
  CpuPower,
  GpuPower,
  AnePower,
  /// System power, or CPU + GPU + ANE without the system power sensor.
  TotalPower,
  DiskRead,
  DiskWrite,
  NetIn,
  NetOut,
}

/// Rectangles for one frame. Hidden parts are `None` / left out; every rect is non-empty.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct LayoutPlan {
  /// Metrics box, borders included.
  pub(super) top: Option<Rect>,
  /// Metric boxes inside the metrics box, borders included: the top row left to right, then the
  /// bottom row.
  pub(super) boxes: Vec<(Metric, Rect)>,
  /// Process box, borders included.
  pub(super) proc: Option<Rect>,
}

/// Boxes of the top row: CPU clusters, GPU, RAM.
fn top_row(clusters: usize) -> Vec<Metric> {
  let mut boxes: Vec<Metric> = (0..clusters).map(Metric::Cluster).collect();
  boxes.extend([Metric::Gpu, Metric::Ram]);
  boxes
}

/// Boxes of the power row: CPU, GPU, ANE and total power.
const BOTTOM_ROW: [Metric; 4] =
  [Metric::CpuPower, Metric::GpuPower, Metric::AnePower, Metric::TotalPower];

/// Boxes of the lowest row: disk read / write, network receive / send.
const IO_ROW: [Metric; 4] = [Metric::DiskRead, Metric::DiskWrite, Metric::NetIn, Metric::NetOut];

/// Height of the metrics box over the process list on a screen `height` rows tall:
/// `METRICS_HEIGHT_PCT` of it, rounded, but at least `METRICS_MIN_HEIGHT`.
fn metrics_height(height: u16) -> u16 {
  // a share of `height`, so it fits in u16
  let share = ((u32::from(height) * METRICS_HEIGHT_PCT + 50) / 100) as u16;
  share.max(METRICS_MIN_HEIGHT).min(height)
}

/// `count` (at least one) boxes side by side splitting the width of `row` evenly; the odd cells
/// are spread between them. Boxes without cells are left out.
fn split_row(row: Rect, count: usize) -> impl Iterator<Item = Rect> {
  let (width, count) = (u64::from(row.width), count as u64);
  // offsets stay within `row.width`, so they fit in u16
  let edge = move |i: u64| (width * i / count) as u16;
  (0..count)
    .map(move |i| Rect { x: row.x + edge(i), width: edge(i + 1) - edge(i), ..row })
    .filter(|r| !r.is_empty())
}

/// Whether `area` has room for the process list under the metrics box: at least `PROC_MIN_ROWS`
/// process rows. The process list setting doesn't matter.
pub(super) fn procs_fit(area: Rect) -> bool {
  !area.is_empty() && area.height - metrics_height(area.height) >= PROC_MIN_ROWS + PROC_CHROME
}

/// Splits `area` into the metrics box and the process box below it.
///
/// With the process list on (`procs`), the metrics box takes `METRICS_HEIGHT_PCT` of the height
/// and the process box the rest; left with fewer than `PROC_MIN_ROWS` process rows, the process
/// list is auto-hidden. Without it, the metrics box takes the whole screen. Without `power`, the
/// top and I/O rows split the metrics box.
pub(super) fn compute_layout(area: Rect, procs: bool, clusters: usize, power: bool) -> LayoutPlan {
  let mut plan = LayoutPlan::default();
  if area.is_empty() {
    return plan;
  }

  let mut top = area;
  if procs && procs_fit(area) {
    top.height = metrics_height(area.height);
    plan.proc = Some(Rect { y: top.bottom(), height: area.height - top.height, ..area });
  }
  plan.top = Some(top);

  let inner = top.inner(Margin::new(1, 1));
  let mut rows = vec![top_row(clusters)];
  if power {
    rows.push(BOTTOM_ROW.to_vec());
  }
  rows.push(IO_ROW.to_vec());

  let count = rows.len() as u16;
  let mut y = inner.y;
  for (i, metrics) in rows.into_iter().enumerate() {
    // the upper rows take the odd cells
    let height = inner.height / count + u16::from(i < usize::from(inner.height % count));
    let row = Rect { y, height, ..inner };
    y += height;
    if row.is_empty() {
      continue;
    }
    let boxes = metrics.len();
    plan.boxes.extend(metrics.into_iter().zip(split_row(row, boxes)));
  }

  plan
}

#[cfg(test)]
mod tests {
  use ratatui::layout::{Margin, Rect};

  use super::{LayoutPlan, PROC_CHROME, PROC_MIN_ROWS, compute_layout, procs_fit};

  #[test]
  fn metrics_take_40_percent_and_procs_the_rest() {
    // (width, height, metrics box height): 40 % of the height, rounded, but at least three rows of
    // boxes with a graph row each
    for (width, height, top) in [(200, 50, 20), (110, 32, 13), (80, 24, 11), (100, 20, 11)] {
      let plan = compute_layout(Rect::new(0, 0, width, height), true, 2, true);
      let ctx = format!("{width}x{height}");
      assert_eq!(plan.top, Some(Rect::new(0, 0, width, top)), "{ctx}");
      assert_eq!(plan.proc, Some(Rect::new(0, top, width, height - top)), "{ctx}");
    }
  }

  #[test]
  fn hidden_or_auto_hidden_list_gives_metrics_the_full_height() {
    let area = Rect::new(0, 0, 120, 40);
    let plan = compute_layout(area, false, 2, true);
    assert_eq!((plan.top, plan.proc), (Some(area), None));
    // 38 rows inside: 13 for each of the first two rows of boxes, 12 for the last
    let heights: Vec<u16> = plan.boxes.chunks(4).map(|row| row[0].1.height).collect();
    assert_eq!(heights, [13, 13, 12], "{plan:?}");

    // 100x13: an 11 rows metrics box would leave 2 rows, short of the smallest process box
    let small = Rect::new(0, 0, 100, 13);
    let plan = compute_layout(small, true, 2, true);
    assert_eq!((plan.top, plan.proc), (Some(small), None));
    assert!(!procs_fit(small) && !procs_fit(Rect { height: 16, ..small }));
    assert!(procs_fit(Rect { height: 17, ..small }));
  }

  #[test]
  fn without_power_the_top_and_io_rows_split_the_height() {
    use super::Metric::*;
    // 200x50 with the process list: a 20 rows metrics box, 18 rows inside
    let area = Rect::new(0, 0, 200, 50);
    let with = compute_layout(area, true, 1, true);
    let rows =
      |plan: &LayoutPlan| plan.boxes.iter().map(|(m, r)| (*m, r.y, r.height)).collect::<Vec<_>>();
    assert_eq!(
      rows(&with),
      [
        (Cluster(0), 1, 6),
        (Gpu, 1, 6),
        (Ram, 1, 6),
        (CpuPower, 7, 6),
        (GpuPower, 7, 6),
        (AnePower, 7, 6),
        (TotalPower, 7, 6),
        (DiskRead, 13, 6),
        (DiskWrite, 13, 6),
        (NetIn, 13, 6),
        (NetOut, 13, 6)
      ]
    );

    // the same metrics and process boxes, two rows of boxes
    let without = compute_layout(area, true, 1, false);
    assert_eq!((without.top, without.proc), (with.top, with.proc));
    assert_eq!(
      rows(&without),
      [
        (Cluster(0), 1, 9),
        (Gpu, 1, 9),
        (Ram, 1, 9),
        (DiskRead, 10, 9),
        (DiskWrite, 10, 9),
        (NetIn, 10, 9),
        (NetOut, 10, 9)
      ]
    );
    let widths = |plan: &LayoutPlan| {
      plan.boxes.iter().map(|(_, r)| (r.x, r.width)).take(3).collect::<Vec<_>>()
    };
    assert_eq!(widths(&without), widths(&with));
  }

  #[test]
  fn boxes_stay_inside_and_never_overlap() {
    let widths = [1, 2, 3, 5, 10, 69, 80, 110, 200, 400];
    let heights = [1, 2, 3, 5, 8, 13, 14, 24, 50, 120];
    for (width, height) in widths.into_iter().flat_map(|w| heights.map(|h| (w, h))) {
      let area = Rect::new(3, 2, width, height);
      let cases = [(0, true, true), (2, false, true), (2, true, true), (3, true, true)];
      for (clusters, procs, power) in cases.into_iter().chain([(1, true, false), (2, false, false)])
      {
        let plan = compute_layout(area, procs, clusters, power);
        let ctx = format!("{area:?} clusters={clusters} procs={procs} power={power}");

        // the metrics box on top, the process box right under it to the bottom
        let top = plan.top.expect("metrics box");
        assert_eq!(plan.proc.is_some(), procs && procs_fit(area), "{ctx}");
        match plan.proc {
          Some(proc) => {
            let rest = Rect { y: top.bottom(), height: area.bottom() - top.bottom(), ..area };
            assert_eq!(proc, rest, "{ctx}");
            assert!(proc.height >= PROC_MIN_ROWS + PROC_CHROME, "{ctx}");
          }
          None => assert_eq!(top, area, "{ctx}: metrics take the full height"),
        }

        // boxes sit inside the borders of the metrics box and never overlap
        let inner = top.inner(Margin::new(1, 1));
        for (i, (_, a)) in plan.boxes.iter().enumerate() {
          assert!(!a.is_empty() && inner.intersection(*a) == *a, "{a:?} out of {inner:?}: {ctx}");
          for (_, b) in &plan.boxes[i + 1..] {
            assert!(!a.intersects(*b), "{a:?} overlaps {b:?} in {ctx}");
          }
        }
      }
    }
  }
}
