//! A percentage `flex-basis` that cannot be resolved because the flex container's main size is
//! indefinite is `content` (<https://www.w3.org/TR/css-flexbox-1/#flex-basis-property>): the item
//! is sized by its content, not by its main size property (as in Chromium).
#[cfg(test)]
mod percentage_flex_basis {
    use taffy::prelude::*;

    /// An auto-height column holding a `flex: 1 1 0%` item of height 36 with 100px of content.
    fn item_height(basis: Dimension) -> f32 {
        let mut taffy: TaffyTree<()> = TaffyTree::new();
        let content = taffy
            .new_leaf(Style { size: Size { width: length(50.0), height: length(100.0) }, ..Default::default() })
            .unwrap();
        let item = taffy
            .new_with_children(
                Style {
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    flex_basis: basis,
                    size: Size { width: auto(), height: length(36.0) },
                    ..Default::default()
                },
                &[content],
            )
            .unwrap();
        let column = taffy
            .new_with_children(
                Style { display: Display::Flex, flex_direction: FlexDirection::Column, ..Default::default() },
                &[item],
            )
            .unwrap();
        taffy.compute_layout(column, Size::MAX_CONTENT).unwrap();
        taffy.layout(item).unwrap().size.height
    }

    #[test]
    fn unresolvable_percentage_is_content() {
        assert_eq!(item_height(percent(0.0)), 100.0);
    }

    #[test]
    fn auto_uses_the_main_size_property() {
        assert_eq!(item_height(auto()), 36.0);
    }
}
