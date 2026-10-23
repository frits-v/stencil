//! junctions-plain (section 6): generated stroke sets that must match the shape, near
//! misses included, and close variants that must not.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::{layout, node, page_from};
use serde_json::json;
use stencil_layout::BoxRect;
use stencil_layout::checks::junctions_plain;
use stencil_layout::junctions::{InkRun, JUNCTION_CENTERS_MAX, JUNCTION_SCORE_MIN, scan_junctions};
use stencil_model::pointer::NodePointer;
use stencil_model::{PagePoint, Projection};

/// A generated figure: four arms from a center, right, down, left and up, each with a hook
/// at its end. A positive hook turns clockwise on screen, a negative one the other way, and
/// zero draws none.
#[derive(Debug, Clone, Copy)]
struct Figure {
    arms: [f32; 4],
    hooks: [f32; 4],
    /// Each arm starts this far from the center.
    gap: f32,
    /// The up and down arms stand this far right of the center.
    offset: f32,
    /// Each hook turns once more, away from the center, for this long.
    tail: f32,
    /// Every run drawn as 6 px dashes with 5 px gaps.
    dashed: bool,
    /// The side of a square outline at the center; 0 draws none.
    hub: f32,
}

impl Figure {
    fn of(arm: f32) -> Figure {
        Figure {
            arms: [arm; 4],
            hooks: [arm; 4],
            gap: 0.0,
            offset: 0.0,
            tail: 0.0,
            dashed: false,
            hub: 0.0,
        }
    }

    fn mirrored(mut self) -> Figure {
        self.hooks = self.hooks.map(|hook| -hook);
        self
    }

    fn runs(&self, center: PagePoint) -> (Vec<InkRun>, Vec<BoxRect>) {
        let directions = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];
        let mut runs = Vec::new();
        let mut push = |index: usize, start: PagePoint, end: PagePoint| {
            let owner = NodePointer::root().child("links").index(index);
            if self.dashed {
                let length = (end.x - start.x).hypot(end.y - start.y);
                let (ux, uy) = ((end.x - start.x) / length, (end.y - start.y) / length);
                let mut at = 0.0;
                while at < length {
                    let stop = (at + 6.0).min(length);
                    runs.push(InkRun {
                        owner: owner.clone(),
                        start: PagePoint {
                            x: start.x + ux * at,
                            y: start.y + uy * at,
                        },
                        end: PagePoint {
                            x: start.x + ux * stop,
                            y: start.y + uy * stop,
                        },
                    });
                    at += 11.0;
                }
            } else {
                runs.push(InkRun { owner, start, end });
            }
        };
        for (index, (dx, dy)) in directions.into_iter().enumerate() {
            let shift = if dy != 0.0 { self.offset } else { 0.0 };
            let point = |distance: f32| PagePoint {
                x: center.x + dx * distance + shift,
                y: center.y + dy * distance,
            };
            let start = point(self.gap.max(self.hub / 2.0));
            let corner = point(self.arms[index]);
            push(index, start, corner);
            let hook = self.hooks[index];
            if hook != 0.0 {
                // A clockwise quarter turn of (dx, dy) on screen is (-dy, dx).
                let (hx, hy) = (-dy * hook.signum(), dx * hook.signum());
                let end = PagePoint {
                    x: corner.x + hx * hook.abs(),
                    y: corner.y + hy * hook.abs(),
                };
                push(index, corner, end);
                if self.tail > 0.0 {
                    let tail_end = PagePoint {
                        x: end.x + dx * self.tail,
                        y: end.y + dy * self.tail,
                    };
                    push(index, end, tail_end);
                }
            }
        }
        let mut hubs = Vec::new();
        if self.hub > 0.0 {
            let half = self.hub / 2.0;
            let bounds = BoxRect {
                x: center.x - half,
                y: center.y - half,
                width: self.hub,
                height: self.hub,
            };
            let corner = |x: f32, y: f32| PagePoint { x, y };
            let (left, top, right, bottom) = (bounds.x, bounds.y, bounds.right(), bounds.bottom());
            for (start, end) in [
                (corner(left, top), corner(right, top)),
                (corner(right, top), corner(right, bottom)),
                (corner(left, bottom), corner(right, bottom)),
                (corner(left, top), corner(left, bottom)),
            ] {
                runs.push(InkRun {
                    owner: NodePointer::root().child("body").index(0),
                    start,
                    end,
                });
            }
            hubs.push(bounds);
        }
        (runs, hubs)
    }
}

/// Arm lengths from a tight tag-sized figure to one that spans a slab.
const ARMS: [f32; 5] = [24.0, 40.0, 64.0, 120.0, 240.0];
/// Centers off the pixel grid, so a cell boundary never lines up with a stroke by luck.
const CENTERS: [PagePoint; 2] = [
    PagePoint { x: 400.0, y: 300.0 },
    PagePoint {
        x: 413.37,
        y: 287.71,
    },
];

fn near_misses(arm: f32) -> Vec<(&'static str, Figure)> {
    let exact = Figure::of(arm);
    vec![
        ("exact", exact),
        (
            "half hooks",
            Figure {
                hooks: [arm * 0.5; 4],
                ..exact
            },
        ),
        (
            "uneven arms and hooks",
            Figure {
                arms: [arm, arm * 0.8, arm * 1.2, arm * 0.9],
                hooks: [arm * 0.7, arm, arm * 0.8, arm * 0.6],
                ..exact
            },
        ),
        (
            "gapped center",
            Figure {
                gap: (arm * 0.15).min(14.0),
                ..exact
            },
        ),
        (
            "arm pairs out of line",
            Figure {
                offset: (arm * 0.08).min(5.0),
                ..exact
            },
        ),
        (
            "dashed",
            Figure {
                dashed: true,
                ..exact
            },
        ),
        (
            "tailed hooks",
            Figure {
                tail: arm * 0.3,
                ..exact
            },
        ),
        (
            "small hub",
            Figure {
                hub: arm * 0.25,
                ..exact
            },
        ),
    ]
}

fn variants(arm: f32) -> Vec<(&'static str, Figure)> {
    let exact = Figure::of(arm);
    vec![
        (
            "one hook reversed",
            Figure {
                hooks: [arm, arm, arm, -arm],
                ..exact
            },
        ),
        (
            "two hooks reversed",
            Figure {
                hooks: [arm, -arm, arm, -arm],
                ..exact
            },
        ),
        (
            "plain cross",
            Figure {
                hooks: [0.0; 4],
                ..exact
            },
        ),
        (
            "three hooks",
            Figure {
                hooks: [arm, arm, arm, 0.0],
                ..exact
            },
        ),
        (
            "large box at the center",
            Figure {
                hub: arm * 1.2,
                ..exact
            },
        ),
    ]
}

/// The best score anywhere in the figure, and whether a match was found.
fn scan(figure: &Figure, center: PagePoint) -> (f32, bool) {
    let (runs, hubs) = figure.runs(center);
    let found = scan_junctions(&runs, &hubs).unwrap();
    assert!(found.examined > 0, "{figure:?}");
    (found.best_score, !found.found.is_empty())
}

#[test]
fn every_near_miss_matches_at_every_scale_and_in_both_turning_directions() {
    let mut examined = 0;
    for arm in ARMS {
        for (name, figure) in near_misses(arm) {
            for center in CENTERS {
                for drawn in [figure, figure.mirrored()] {
                    let (score, matched) = scan(&drawn, center);
                    assert!(matched, "{name} at arm {arm}: best score {score:.3}");
                    assert!(score >= JUNCTION_SCORE_MIN, "{name} at arm {arm}");
                    examined += 1;
                }
            }
        }
    }
    assert_eq!(examined, ARMS.len() * 8 * CENTERS.len() * 2);
}

/// A reversed hook, a missing one or a plain cross scores high over the whole window, up to
/// 0.94; what keeps them apart is that each of the eight strokes must be present.
#[test]
fn no_close_variant_matches() {
    let mut examined = 0;
    let mut over_threshold = 0;
    for arm in ARMS {
        for (name, figure) in variants(arm) {
            for center in CENTERS {
                for drawn in [figure, figure.mirrored()] {
                    let (score, matched) = scan(&drawn, center);
                    assert!(!matched, "{name} at arm {arm}: score {score:.3}");
                    if score >= JUNCTION_SCORE_MIN {
                        over_threshold += 1;
                    }
                    examined += 1;
                }
            }
        }
    }
    assert_eq!(examined, ARMS.len() * 5 * CENTERS.len() * 2);
    // Without the per-stroke floor these would match: the threshold alone does not hold.
    assert!(over_threshold > 0);
}

/// A lattice of 100 by 100 crossing lines meets at more junctions than the check examines,
/// and the scan says so rather than passing over part of it.
#[test]
fn a_page_with_more_junctions_than_the_cap_is_not_examined() {
    let owner = NodePointer::root().child("links").index(0);
    let mut runs = Vec::new();
    for line in 0..100 {
        let at = 10.0 + 20.0 * line as f32;
        runs.push(InkRun {
            owner: owner.clone(),
            start: PagePoint { x: 0.0, y: at },
            end: PagePoint { x: 2000.0, y: at },
        });
        runs.push(InkRun {
            owner: owner.clone(),
            start: PagePoint { x: at, y: 0.0 },
            end: PagePoint { x: at, y: 2000.0 },
        });
    }
    const { assert!(100 * 100 > JUNCTION_CENTERS_MAX) };
    assert_eq!(scan_junctions(&runs, &[]), None);
}

/// Four cards and four links laid out, then the links redrawn as four arms around a point
/// of open floor below the body, each turning clockwise: a defect at the first link that
/// names every arm. The page as laid out passes, over more than zero junctions.
#[test]
fn links_drawn_as_the_shape_are_a_defect_at_the_first_link() {
    let card = |id: &str| json!({ "tag": "Item", "kind": "product", "id": id, "title": id });
    let link = |from: &str, to: &str| json!({ "from": from, "to": to, "line": "solid", "tint": 1 });
    let page = page_from(json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "width": 960,
        "badge": "Customer",
        "body": [{ "tag": "Row", "gap": 64, "children": [
            card("a"), card("b"), card("c"), card("d")
        ]}],
        "legend": [{ "line": "solid", "tint": 1, "text": "request path" }],
        "links": [link("a", "b"), link("b", "c"), link("c", "d"), link("a", "d")]
    }));
    let mut geometry = layout(&page);
    let clean = junctions_plain(&geometry, Projection::Flat);
    assert!(clean.examined > 0, "{clean:?}");
    assert!(clean.defects.is_empty(), "{clean:?}");

    let body = node(&geometry, "/body").bounds;
    let center = PagePoint {
        x: body.x + body.width / 2.0,
        y: body.bottom() + 400.0,
    };
    let (runs, _) = Figure::of(120.0).runs(center);
    for (index, route) in geometry.links.iter_mut().enumerate() {
        let owner = NodePointer::root().child("links").index(index);
        let mine: Vec<&InkRun> = runs.iter().filter(|run| run.owner == owner).collect();
        route.points = vec![mine[0].start, mine[0].end, mine[1].end];
    }
    let report = junctions_plain(&geometry, Projection::Flat);
    assert_eq!(report.defects.len(), 1, "{report:?}");
    let defect = &report.defects[0];
    assert_eq!(defect.pointer.as_str(), "/links/0");
    assert!(
        defect.message.starts_with("four arms of ")
            && defect
                .message
                .contains("each turn the same way at their ends")
            && defect
                .message
                .ends_with("/links/0, /links/1, /links/2, /links/3"),
        "{}",
        defect.message
    );
}
