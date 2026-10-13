//! The intrinsic main size of a column flex container is the largest sum, over its lines, of its
//! items' outer hypothetical main sizes: their flex base sizes clamped by their used min and max
//! main sizes. This is Chromium's `FlexLayoutAlgorithm` (`PlaceFlexItems` takes the lines'
//! largest `sum_hypothetical_main_size` as a column's intrinsic block size), not css-flexbox-1
//! §9.9.1's max-content flex fractions: an item with `flex: 1 1 50px; min-height: 0` and 100px of
//! content contributes 50px. A scroll container contributes its flex base size clamped by its
//! `min-height` and `max-height` (its automatic minimum size is zero).
//!
//! Expected values were measured in Chromium 153.0.8010.12 (the Runic conformance harness's
//! browser): a 400px column flex container with a 1px border (`box-sizing: border-box`) holding
//! the items below, each with a block child of the given height, followed by a 50px block.
#[cfg(test)]
mod column_intrinsic_main_size {
    use taffy::prelude::*;
    use taffy::{compute_leaf_layout, Overflow, Point};

    #[derive(Clone, Copy)]
    enum Wrapper {
        None,
        FlexRow,
        Grid,
        FixedColumn,
    }

    #[derive(Clone)]
    struct Item {
        style: Style,
        content: f32,
    }

    fn item(content: f32, f: impl FnOnce(&mut Style)) -> Item {
        let mut style = Style::default();
        f(&mut style);
        Item { style, content }
    }

    fn flex(style: &mut Style, grow: f32, shrink: f32, basis: Dimension) {
        style.flex_grow = grow;
        style.flex_shrink = shrink;
        style.flex_basis = basis;
    }

    fn hidden(style: &mut Style) {
        style.overflow = Point { x: Overflow::Hidden, y: Overflow::Hidden };
    }

    /// Lays the case out in an 800px wide block and returns the container's height and each
    /// item's (y, height) relative to the container, the trailing 50px block last. Each item is
    /// laid out twice, as a block holding a child of its content height and as a measured leaf
    /// (as a text area is in Blitz), which must agree.
    fn run(wrapper: Wrapper, container: impl Fn(&mut Style), items: Vec<Item>) -> (f32, Vec<(f32, f32)>) {
        let block = layout(false, wrapper, &container, items.clone());
        let leaf = layout(true, wrapper, &container, items);
        assert_eq!(block, leaf, "items as blocks (left) and as measured leaves (right)");
        block
    }

    fn layout(
        leaf: bool,
        wrapper: Wrapper,
        container: impl Fn(&mut Style),
        items: Vec<Item>,
    ) -> (f32, Vec<(f32, f32)>) {
        let mut taffy: TaffyTree<f32> = TaffyTree::new();
        let mut nodes = Vec::new();
        for Item { style, content } in items {
            if leaf {
                nodes.push(taffy.new_leaf_with_context(style, content).unwrap());
            } else {
                let child = taffy
                    .new_leaf(Style { size: Size { width: auto(), height: length(content) }, ..Default::default() })
                    .unwrap();
                nodes.push(taffy.new_with_children(style, &[child]).unwrap());
            }
        }
        nodes.push(
            taffy.new_leaf(Style { size: Size { width: auto(), height: length(50.0) }, ..Default::default() }).unwrap(),
        );
        let mut container_style = Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: Size { width: length(400.0), height: auto() },
            border: Rect::length(1.0),
            ..Default::default()
        };
        if let Wrapper::FixedColumn = wrapper {
            container_style.flex_shrink = 0.0;
        }
        container(&mut container_style);
        let flex_container = taffy.new_with_children(container_style, &nodes).unwrap();
        let outer = match wrapper {
            Wrapper::None => flex_container,
            Wrapper::FlexRow => taffy
                .new_with_children(
                    Style { display: Display::Flex, align_items: Some(AlignItems::FLEX_START), ..Default::default() },
                    &[flex_container],
                )
                .unwrap(),
            Wrapper::Grid => taffy
                .new_with_children(
                    Style { display: Display::Grid, align_items: Some(AlignItems::START), ..Default::default() },
                    &[flex_container],
                )
                .unwrap(),
            Wrapper::FixedColumn => taffy
                .new_with_children(
                    Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: Some(AlignItems::FLEX_START),
                        size: Size { width: auto(), height: length(300.0) },
                        ..Default::default()
                    },
                    &[flex_container],
                )
                .unwrap(),
        };
        let root = taffy
            .new_with_children(
                Style {
                    display: Display::Block,
                    size: Size { width: length(800.0), height: auto() },
                    ..Default::default()
                },
                &[outer],
            )
            .unwrap();
        taffy
            .compute_layout_with_measure(root, Size::MAX_CONTENT, |inputs, _, content, style| {
                let content = content.copied();
                compute_leaf_layout(
                    inputs,
                    style,
                    |_, _| 0.0,
                    |known, available| Size {
                        width: known.width.unwrap_or(match available.width {
                            AvailableSpace::Definite(width) => width,
                            _ => 0.0,
                        }),
                        height: known.height.unwrap_or(content.unwrap_or(0.0)),
                    },
                )
            })
            .unwrap();
        let height = taffy.layout(flex_container).unwrap().size.height;
        let items = nodes
            .iter()
            .map(|&node| {
                let layout = taffy.layout(node).unwrap();
                (layout.location.y, layout.size.height)
            })
            .collect();
        (height, items)
    }

    /// `overflow: hidden; flex: 1 1 0%; min-height: 64px` with 20px of content.
    fn growing_scroller(f: impl FnOnce(&mut Style)) -> Item {
        item(20.0, |s| {
            hidden(s);
            flex(s, 1.0, 1.0, percent(0.0));
            s.min_size.height = length(64.0);
            f(s);
        })
    }

    #[test]
    fn growing_scroll_container_with_min_height() {
        let scroller = growing_scroller(|s| s.padding = Rect::length(8.0));
        assert_eq!(run(Wrapper::None, |_| {}, vec![scroller]), (116.0, vec![(1.0, 64.0), (65.0, 50.0)]));
    }

    #[test]
    fn growing_scroll_container_with_scrollbars_and_min_height() {
        let scroller = growing_scroller(|s| {
            s.padding = Rect::length(8.0);
            s.overflow = Point { x: Overflow::Scroll, y: Overflow::Scroll };
            s.scrollbar_width = 0.0;
        });
        assert_eq!(run(Wrapper::None, |_| {}, vec![scroller]), (116.0, vec![(1.0, 64.0), (65.0, 50.0)]));
    }

    #[test]
    fn growing_block_with_min_height() {
        let block = item(20.0, |s| {
            flex(s, 1.0, 1.0, percent(0.0));
            s.min_size.height = length(64.0);
            s.padding = Rect::length(8.0);
        });
        assert_eq!(run(Wrapper::None, |_| {}, vec![block]), (116.0, vec![(1.0, 64.0), (65.0, 50.0)]));
    }

    #[test]
    fn growing_scroll_container_with_max_height() {
        let scroller = item(100.0, |s| {
            hidden(s);
            flex(s, 1.0, 1.0, auto());
            s.max_size.height = length(30.0);
        });
        assert_eq!(run(Wrapper::None, |_| {}, vec![scroller]), (82.0, vec![(1.0, 30.0), (31.0, 50.0)]));
    }

    #[test]
    fn rigid_scroll_container_with_max_height() {
        let scroller = item(100.0, |s| {
            hidden(s);
            flex(s, 0.0, 0.0, auto());
            s.max_size.height = length(30.0);
        });
        assert_eq!(run(Wrapper::None, |_| {}, vec![scroller]), (82.0, vec![(1.0, 30.0), (31.0, 50.0)]));
    }

    #[test]
    fn scroll_container_with_zero_flex_basis() {
        let scroller = item(100.0, |s| {
            hidden(s);
            flex(s, 1.0, 1.0, length(0.0));
        });
        assert_eq!(run(Wrapper::None, |_| {}, vec![scroller]), (52.0, vec![(1.0, 0.0), (1.0, 50.0)]));
    }

    /// The item's hypothetical main size (its 50px flex base size, without an automatic minimum)
    /// is its contribution, not its 100px max-content size: the item overflows the container.
    #[test]
    fn growing_block_without_minimum_contributes_its_flex_base_size() {
        let block = item(100.0, |s| {
            flex(s, 1.0, 1.0, length(50.0));
            s.min_size.height = length(0.0);
        });
        assert_eq!(run(Wrapper::None, |_| {}, vec![block]), (102.0, vec![(1.0, 50.0), (51.0, 50.0)]));
    }

    #[test]
    fn rigid_block_without_minimum_contributes_its_flex_base_size() {
        let block = item(100.0, |s| {
            flex(s, 0.0, 1.0, length(50.0));
            s.min_size.height = length(0.0);
        });
        assert_eq!(run(Wrapper::None, |_| {}, vec![block]), (102.0, vec![(1.0, 50.0), (51.0, 50.0)]));
    }

    #[test]
    fn block_with_zero_flex_basis_keeps_its_automatic_minimum() {
        let block = item(100.0, |s| flex(s, 1.0, 1.0, length(0.0)));
        assert_eq!(run(Wrapper::None, |_| {}, vec![block]), (152.0, vec![(1.0, 100.0), (101.0, 50.0)]));
    }

    #[test]
    fn scroll_container_min_height_wins_over_max_height() {
        let scroller = growing_scroller(|s| s.max_size.height = length(40.0));
        assert_eq!(run(Wrapper::None, |_| {}, vec![scroller]), (116.0, vec![(1.0, 64.0), (65.0, 50.0)]));
    }

    #[test]
    fn scroll_container_with_margins() {
        let scroller = growing_scroller(|s| {
            s.margin = Rect { left: zero(), right: zero(), top: length(5.0), bottom: length(5.0) }
        });
        assert_eq!(run(Wrapper::None, |_| {}, vec![scroller]), (126.0, vec![(6.0, 64.0), (75.0, 50.0)]));
    }

    #[test]
    fn two_growing_scroll_containers() {
        let first = item(10.0, |s| {
            hidden(s);
            flex(s, 1.0, 1.0, percent(0.0));
            s.min_size.height = length(40.0);
        });
        let second = item(10.0, |s| {
            hidden(s);
            flex(s, 2.0, 1.0, percent(0.0));
            s.min_size.height = length(70.0);
        });
        assert_eq!(
            run(Wrapper::None, |_| {}, vec![first, second]),
            (162.0, vec![(1.0, 40.0), (41.0, 70.0), (111.0, 50.0)])
        );
    }

    #[test]
    fn growing_scroll_container_with_gap() {
        let scroller = growing_scroller(|_| {});
        assert_eq!(
            run(Wrapper::None, |c| c.gap = Size { width: zero(), height: length(10.0) }, vec![scroller]),
            (126.0, vec![(1.0, 64.0), (75.0, 50.0)])
        );
    }

    #[test]
    fn growing_scroll_container_in_wrapping_column() {
        let scroller = growing_scroller(|_| {});
        assert_eq!(
            run(Wrapper::None, |c| c.flex_wrap = FlexWrap::Wrap, vec![scroller]),
            (116.0, vec![(1.0, 64.0), (65.0, 50.0)])
        );
    }

    #[test]
    fn scroll_container_flex_basis_above_max_height() {
        let scroller = item(20.0, |s| {
            hidden(s);
            flex(s, 1.0, 1.0, length(120.0));
            s.max_size.height = length(80.0);
        });
        assert_eq!(run(Wrapper::None, |_| {}, vec![scroller]), (132.0, vec![(1.0, 80.0), (81.0, 50.0)]));
    }

    #[test]
    fn growing_scroll_container_in_a_flex_row() {
        assert_eq!(
            run(Wrapper::FlexRow, |_| {}, vec![growing_scroller(|_| {})]),
            (116.0, vec![(1.0, 64.0), (65.0, 50.0)])
        );
    }

    #[test]
    fn growing_scroll_container_in_a_grid() {
        assert_eq!(
            run(Wrapper::Grid, |_| {}, vec![growing_scroller(|_| {})]),
            (116.0, vec![(1.0, 64.0), (65.0, 50.0)])
        );
    }

    #[test]
    fn growing_scroll_container_in_a_fixed_height_column() {
        assert_eq!(
            run(Wrapper::FixedColumn, |_| {}, vec![growing_scroller(|_| {})]),
            (116.0, vec![(1.0, 64.0), (65.0, 50.0)])
        );
    }
}
