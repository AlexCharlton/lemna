//! [Flexbox](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_flexible_box_layout/Basic_concepts_of_flexbox)-like layout resolution.
//! All [`Nodes`](crate::Node) have a [`Layout`] attached, and this module is responsible for assigning a [`LayoutResult`] -- an absolution position and size --
//! to the Node, during the draw phase. All [`Layout`] creation functionality -- and thus the entire user-facing interface -- is exposed through the less-verbose [`lay!`][crate::lay] macro.
//!
#![doc = include_str!("../../docs/layout.md")]
extern crate alloc;
use alloc::{vec, vec::Vec};

mod types;
pub use types::*;
#[macro_use]
mod macros;

use crate::renderable::Caches;

//--------------------------------
// MARK: Node::resolve_layout
//--------------------------------
impl super::node::Node {
    /// Returns the best available dimension for a child's bounds, preferring:
    /// 1. Child's resolved size (minus padding) if available
    /// 2. Parent's inner_size if resolved
    /// 3. Ancestor's bounds_size as fallback
    /// Will not exceed the bounds_dim.
    fn best_available_dimension(
        child_dim: Dimension,
        inner_dim: Dimension,
        bounds_dim: Dimension,
        padding_start: Dimension,
        padding_end: Dimension,
    ) -> Dimension {
        if child_dim.resolved() {
            (child_dim - padding_start - padding_end)
                .max(Dimension::Px(0.0))
                .min(bounds_dim)
        } else if inner_dim.resolved() {
            inner_dim.min(bounds_dim)
        } else {
            bounds_dim
        }
    }

    // Used to pass bounds from parent to children. Use the most specific available size for each dimension:
    // 1. Child's resolved size (minus padding) - if child was just resolved, use its size
    // 2. inner_size - parent's actual inner size if resolved
    // 3. bounds_size - fallback from ancestor
    // For wrapping nodes resolved from remaining space, use that remaining space as the constraint

    fn bounds_size(
        &self,
        parent_inner_size: Size,
        parent_bounds_size: Size,
        remaining_space_main: Dimension,
    ) -> Size {
        let dir = self.layout.direction;
        let padding = self.layout.padding.maybe_resolve(&parent_bounds_size);

        let main_dim = if !parent_inner_size.main(dir).resolved() {
            // Use remaining space if parent_inner_size is not resolved
            remaining_space_main
        } else {
            Self::best_available_dimension(
                self.layout_result.size.main(dir),
                parent_inner_size.main(dir),
                parent_bounds_size.main(dir).min(remaining_space_main),
                padding.main(dir, self.layout.axis_alignment),
                padding.main_reverse(dir, self.layout.axis_alignment),
            )
        };

        dir.size(
            main_dim,
            Self::best_available_dimension(
                self.layout_result.size.cross(dir),
                parent_inner_size.cross(dir),
                parent_bounds_size.cross(dir),
                padding.cross(dir, self.layout.cross_alignment),
                padding.cross_reverse(dir, self.layout.cross_alignment),
            ),
        )
    }

    fn resolve_child_sizes(
        &mut self,
        bounds_size: Size,
        caches: &Caches,
        scale_factor: f32,
        final_pass: bool,
    ) {
        let size = if self.layout_result.main_resolved {
            self.layout.size.most_specific(&self.layout_result.size)
        } else {
            self.layout.size
        };

        let mut inner_size = size.minus_bounds(&self.layout.padding.maybe_resolve(&bounds_size));
        if self.scroll_x().is_some() {
            inner_size.width = Dimension::Auto;
        };
        if self.scroll_y().is_some() {
            inner_size.height = Dimension::Auto;
        };

        let dir = self.layout.direction;
        let axis_align = self.layout.axis_alignment;
        // Calculate maximum available space - always constrained by bounds_size from parent
        // even if inner_size is resolved (which might be from a previous pass)
        let max_available = if bounds_size.main(dir).resolved() {
            // Always use bounds_size as the constraint from the parent, not inner_size
            // which might be from a previous pass and exceed the parent's constraint
            let padding = self.layout.padding.maybe_resolve(&bounds_size);
            let main_start_padding = padding.main(dir, axis_align);
            let main_end_padding = padding.main_reverse(dir, axis_align);
            (bounds_size.main(dir) - main_start_padding - main_end_padding).max(Dimension::Px(0.0))
        } else if inner_size.main(dir).resolved() {
            // Fallback to inner_size if bounds_size is not resolved
            inner_size.main(dir)
        } else {
            Dimension::Auto
        };
        let mut main_remaining = f64::from(max_available);
        let mut max_cross_size = 0.0;
        let mut unresolved = 0;
        let mut unresolved_flex_grow = 0.0;
        // dbg!(&self.component, inner_size);

        for child in self.children.iter_mut() {
            child.layout_result.direction = dir;
            // Stretch alignment - only apply if cross size is not already resolved
            if self.layout.cross_alignment == Alignment::Stretch
                && !child.layout_result.size.cross(dir).resolved()
            {
                *child.layout_result.size.cross_mut(dir) = Dimension::Pct(100.0)
            }

            if cfg!(debug_assertions) && child.layout.debug.is_some() {
                log::debug!(
                    "{} Resolving child position of {} - Basing off child.layout.size {:?}, child.layout_result.size {:?}, inner_size {:?})",
                    if final_pass {
                        "Final pass"
                    } else {
                        "First pass"
                    },
                    child.layout.debug.as_ref().unwrap(),
                    &child.layout.size,
                    &child.layout_result.size,
                    &inner_size,
                );
            }

            let child_margin = child.layout.margin.maybe_resolve(&inner_size);

            let resolved_size = child
                .layout
                .size
                .more_specific(&child.layout_result.size.plus_bounds(&child_margin))
                .maybe_resolve(&inner_size);

            // Only subtract margins if the size was computed (not explicitly set as pixels)
            // We can detect this by checking if layout.size.main(dir) was resolved before maybe_resolve
            child.layout_result.size = if child.layout.size.main(dir).resolved() {
                // Explicit pixel size is content size, don't subtract margin
                child.layout_result.main_resolved = true;
                child.layout_result.main_layout_type = LayoutType::Fixed;
                resolved_size
            } else {
                if child.layout.size.main(dir).is_pct() {
                    child.layout_result.main_layout_type = LayoutType::Percent;
                }
                // Computed size (percentages, etc.) might be bounds size, subtract margin
                resolved_size.minus_bounds(&child_margin)
            };

            if self.layout.axis_alignment == Alignment::Stretch
                && child.layout.size.main(dir) == Dimension::Auto
                && child.layout.flex_grow != 0.0
            {
                // We want to calculate this in the next for block
                *child.layout_result.size.main_mut(dir) = Dimension::Auto;
                child.layout_result.main_layout_type = LayoutType::Flex;
            } else {
                // The flex grow is not used for this child, so we set it to 0.0
                child.layout.flex_grow = 0.0;
            }
            if !child.layout_result.size.resolved() {
                // Use bounds_size as fallback when inner_size is not resolved (for fill_bounds constraints)
                let fill_bounds_size = inner_size.most_specific(&bounds_size);
                let fill_bounds_inner_size = fill_bounds_size
                    .minus_bounds(&child.layout.margin.maybe_resolve(&fill_bounds_size));
                let (w, h) = child.component.fill_bounds(
                    child.layout_result.size.width.maybe_px(),
                    child.layout_result.size.height.maybe_px(),
                    fill_bounds_inner_size.width.maybe_px().or(self
                        .layout
                        .max_size
                        .width
                        .maybe_px()),
                    fill_bounds_inner_size.height.maybe_px().or(self
                        .layout
                        .max_size
                        .height
                        .maybe_px()),
                    caches,
                    scale_factor,
                );
                if let Some(w) = w {
                    child.layout_result.size.width = Dimension::Px(w.into());
                }
                if let Some(h) = h {
                    child.layout_result.size.height = Dimension::Px(h.into());
                }
                if child.layout_result.size.main(dir).resolved() {
                    child.layout_result.main_resolved = true;
                    child.layout_result.main_layout_type = LayoutType::Intrinsic;
                }
            }

            if f32::from(child.layout_result.size.cross(dir)) > max_cross_size {
                max_cross_size = child.layout_result.size.cross(dir).into();
            }

            if let Dimension::Px(x) = child.layout_result.size.main(dir)
                && child.layout_result.main_resolved
            {
                if !self.layout.wrap {
                    // Don't subtract from main_remain for wrap nodes, since we always have the same main space for each row.
                    main_remaining -= x + f64::from(child_margin.main_total(dir));
                }
            } else {
                unresolved += 1;
                unresolved_flex_grow += child.layout.flex_grow;
            }
        }
        main_remaining = main_remaining.max(0.0);

        // We use this to track the remaining space for unresolved children.
        let mut current_main_remaining = f64::from(main_remaining);

        for child in self.children.iter_mut() {
            let main_remaining_before_this_child = current_main_remaining;
            if self.layout.axis_alignment == Alignment::Stretch
                && !child.layout_result.size.main(dir).resolved()
                && child.layout.flex_grow != 0.0
            {
                let margin = child.layout.margin.maybe_resolve(&inner_size);
                let flex_ratio = child.layout.flex_grow / unresolved_flex_grow;
                let size = main_remaining * flex_ratio;
                *child.layout_result.size.main_mut(dir) =
                    Dimension::Px(main_remaining * flex_ratio) - margin.main_total(dir);
                current_main_remaining -= size;
            } else if unresolved == 1
                && !child.layout.size.main(dir).resolved()
                && child.layout.wrap
                && main_remaining > 0.0
            {
                // If there's exactly one unresolved child with auto size and wrapping enabled,
                // and we have remaining space, resolve it from the remaining space
                // (all siblings have resolved sizes)
                let margin = child.layout.margin.maybe_resolve(&inner_size);
                let margin_main = f64::from(margin.main(dir, Alignment::Start))
                    + f64::from(margin.main(dir, Alignment::End));
                *child.layout_result.size.main_mut(dir) =
                    Dimension::Px(main_remaining - margin_main);
                current_main_remaining = 0.0;
                child.layout_result.main_layout_type = LayoutType::Wrapping;
            }

            // size as a pct of max sibling
            if (child.layout.size.cross_mut(dir).is_pct()
                || child.layout_result.size.cross_mut(dir).is_pct())
                && !child.layout_result.size.cross(dir).resolved()
                && !self.layout.wrap
                && max_cross_size > 0.0
            {
                let mut max_cross = Size::default();
                *max_cross.cross_mut(dir) = Dimension::Px(max_cross_size.into());
                let margin = child.layout.margin.maybe_resolve(&inner_size);
                let size = child
                    .layout
                    .size
                    .most_specific(&child.layout_result.size)
                    .maybe_resolve(&max_cross);

                child.layout_result.size = size.minus_bounds(&margin);
                current_main_remaining -= f64::from(size.main(dir));
            }

            let remaining_space_passed = Dimension::Px(main_remaining_before_this_child);

            child.resolve_layout(
                child.bounds_size(inner_size, bounds_size, remaining_space_passed),
                caches,
                scale_factor,
                final_pass,
            );

            current_main_remaining = current_main_remaining.max(0.0);
        }
    }

    fn resolve_position(&mut self, bounds: Size) {
        let pos = self.layout_result.position;
        let size = self.layout_result.size;
        match (pos.top, pos.bottom) {
            (Dimension::Px(top), _) => {
                // Correct any discrepancy with bottom relative to top
                self.layout_result.position.bottom = Dimension::Px(top + f64::from(size.height));
            }
            (_, Dimension::Px(bottom)) => {
                self.layout_result.position.top =
                    Dimension::Px(f64::from(bounds.height) - bottom - f64::from(size.height));
                // Transform the bottom relative position into top relative
                self.layout_result.position.bottom =
                    Dimension::Px(f64::from(bounds.height) - bottom);
            }
            _ => self.layout_result.position.top = Dimension::Px(0.0),
        }
        match (pos.left, pos.right) {
            (Dimension::Px(left), _) => {
                // Correct any discrepancy with bottom relative to top
                self.layout_result.position.right = Dimension::Px(left + f64::from(size.width));
            }
            (_, Dimension::Px(right)) => {
                self.layout_result.position.left =
                    Dimension::Px(f64::from(bounds.width) - right - f64::from(size.width));
                // Transform the right relative position into left relative
                self.layout_result.position.right = Dimension::Px(f64::from(bounds.width) - right);
            }
            _ => self.layout_result.position.left = Dimension::Px(0.0),
        }
    }

    fn set_children_position(&mut self, bounds_size: Size) -> Size {
        let dir = self.layout.direction;
        let size = self.layout.size.most_specific(&self.layout_result.size);
        let axis_align = self.layout.axis_alignment;
        let cross_align = self.layout.cross_alignment;
        let main_start_padding: f64 = self
            .layout
            .padding
            .main(dir, axis_align)
            .maybe_resolve(&size.main(dir))
            .into();
        let main_end_padding: f64 = self
            .layout
            .padding
            .main_reverse(dir, axis_align)
            .maybe_resolve(&size.main(dir))
            .into();
        let mut main_pos: f64 = main_start_padding;
        let mut cross_pos = self
            .layout
            .padding
            .cross(dir, cross_align)
            .maybe_resolve(&size.cross(dir))
            .into();
        let mut max_cross_size = 0.0;
        let mut row_lengths: Vec<(f64, usize)> = vec![];
        let mut row_elements_count: usize = 0;

        // Reverse the calculation when End axis_aligned
        let mut children: Vec<&mut Self> = if axis_align == Alignment::End {
            self.children.iter_mut().rev().collect()
        } else {
            self.children.iter_mut().collect()
        };

        for child in children.iter_mut() {
            let margin = child.layout.margin.maybe_resolve(&size);
            let child_outer_size = child.layout_result.size.plus_bounds(&margin);

            // Perform a wrap?
            // Use bounds_size as fallback when size is not resolved (for wrapping nodes with auto size)
            let wrap_size = if size.main(dir).resolved() {
                size.main(dir)
            } else if self.layout.wrap && bounds_size.main(dir).resolved() {
                bounds_size.main(dir)
            } else {
                Dimension::Auto
            };
            if self.layout.wrap
                && wrap_size.resolved()
                && child.layout.position_type != PositionType::Absolute
                && (main_pos + main_end_padding + f64::from(child_outer_size.main(dir)))
                    > f64::from(wrap_size)
                && main_pos > main_start_padding
            {
                row_lengths.push((main_pos, row_elements_count));
                main_pos = main_start_padding;
                cross_pos += max_cross_size;
                max_cross_size = 0.0;
                row_elements_count = 0;
            }

            if child.layout.position_type == PositionType::Relative {
                child.layout_result.position = dir.rect(
                    Dimension::Px(main_pos),
                    Dimension::Px(cross_pos),
                    axis_align,
                    cross_align,
                );
                *child.layout_result.position.main_mut(dir, axis_align) +=
                    margin.main(dir, axis_align);
                *child.layout_result.position.cross_mut(dir, cross_align) +=
                    margin.cross(dir, cross_align);

                child.resolve_position(size);

                // Push bounds
                main_pos += f64::from(child_outer_size.main(dir));
                row_elements_count += 1;
                let child_cross = f64::from(child_outer_size.cross(dir));
                if child_cross > max_cross_size {
                    max_cross_size = child_cross;
                }

                if cfg!(debug_assertions) && child.layout.debug.is_some() {
                    log::debug!(
                        "Setting relative position of {} to {:#?} - Basing off ...",
                        child.layout.debug.as_ref().unwrap(),
                        &child.layout_result.position,
                    );
                }
            } else {
                child.layout_result.position = child.layout.position.most_specific(&dir.rect(
                    Dimension::Px(main_pos),
                    Dimension::Px(cross_pos),
                    axis_align,
                    cross_align,
                ));
                *child.layout_result.position.main_mut(dir, axis_align) +=
                    margin.main(dir, axis_align);
                *child.layout_result.position.cross_mut(dir, cross_align) +=
                    margin.cross(dir, cross_align);

                child.resolve_position(size);

                // TODO: More of these
                if cfg!(debug_assertions) && child.layout.debug.is_some() {
                    log::debug!(
                        "Setting absolute position of {} to {:#?} - Basing off explicit position ({:#?}), parent size ({:#?}))",
                        child.layout.debug.as_ref().unwrap(),
                        &child.layout_result.position,
                        &child.layout.position,
                        &size
                    );
                }
            }
        }

        row_lengths.push((main_pos, row_elements_count));

        // Combined size of children
        let mut children_size = if self.children.is_empty() {
            Size::default()
        } else {
            // For wrapping nodes, use the maximum row width, not the current position
            let main_size = if self.layout.wrap && !row_lengths.is_empty() {
                row_lengths.iter().map(|(len, _)| *len).fold(0.0, f64::max)
            } else {
                main_pos
            };
            let cross_size = cross_pos + max_cross_size;
            dir.size(Dimension::Px(main_size), Dimension::Px(cross_size))
        };
        *children_size.main_mut(dir) += self.layout.padding.main_reverse(dir, axis_align);
        *children_size.cross_mut(dir) += self.layout.padding.cross_reverse(dir, cross_align);

        // TODO Alignment::Stretch when not all space is filled

        if axis_align == Alignment::Center || cross_align == Alignment::Center {
            // Reposition center alignment
            let main_offset = if axis_align == Alignment::Center && size.main(dir).resolved() {
                // This is only accurate when for non-wrapped elements.
                // For wrapped elements, we compute within the loop
                (f64::from(size.main(dir)) - f64::from(children_size.main(dir))) / 2.0
            } else {
                0.0
            };
            let cross_size = {
                if size.cross(dir).resolved() {
                    f64::from(size.cross(dir))
                } else {
                    f64::from(children_size.cross(dir))
                }
            };

            let mut elements_positioned_in_row = 0;
            let mut current_row = 0;
            for child in self.children.iter_mut() {
                if child.layout.position_type == PositionType::Absolute {
                    continue;
                }
                let main_offset = if self.layout.wrap {
                    if elements_positioned_in_row >= row_lengths[current_row].1 {
                        elements_positioned_in_row = 0;
                        current_row += 1;
                    }
                    (f64::from(size.main(dir)) - (row_lengths[current_row].0 + main_end_padding))
                        / 2.0
                } else {
                    main_offset
                };
                *child.layout_result.position.main_mut(dir, axis_align) +=
                    Dimension::Px(main_offset);

                if cross_align == Alignment::Center {
                    if row_lengths.len() > 1 {
                        // TODO: Center within a row?
                        *child.layout_result.position.cross_mut(dir, cross_align) +=
                            Dimension::Px((cross_size - f64::from(children_size.cross(dir))) / 2.0);
                    } else {
                        *child.layout_result.position.cross_mut(dir, cross_align) = Dimension::Px(
                            (cross_size - f64::from(child.layout_result.size.cross(dir))) / 2.0,
                        );
                    };
                }

                child.resolve_position(size);
                elements_positioned_in_row += 1;

                if cfg!(debug_assertions) && child.layout.debug.is_some() {
                    log::debug!(
                        "Resolved aligned position of {} to {:#?} - Basing off ...)",
                        child.layout.debug.as_ref().unwrap(),
                        &child.layout_result.position
                    );
                }
            }
        }

        children_size
    }

    /// Make sure the node has a size, either taken from its children or from itself
    fn resolve_size(&mut self, children_size: Size, final_pass: bool) {
        if self
            .layout
            .size
            .main(self.layout_result.direction)
            .resolved()
        {
            // Needed because root nodes can have a fixed size. Otherwise, only children have this set
            self.layout_result.main_layout_type = LayoutType::Fixed;
        }

        let mut size = self.layout.size.most_specific(&self.layout_result.size);

        let min_size = self.layout.min_size;
        let dir = self.layout.direction;
        if final_pass && self.layout_result.main_layout_type == LayoutType::Auto {
            *size.main_mut(self.layout_result.direction) = Dimension::Auto;
        }

        // For wrapping nodes with auto size that were temporarily resolved, allow shrinking to children's size
        // Allow shrinking on both main and cross axes if the original size was Auto
        let allow_shrink_main = self.layout.wrap
            && self.layout.size.main(dir) == Dimension::Auto
            && size.main(dir).resolved()
            && children_size.main(dir).resolved()
            && f64::from(children_size.main(dir)) < f64::from(size.main(dir));

        let allow_shrink_cross = self.layout.wrap
            && self.layout.size.cross(dir) == Dimension::Auto
            && size.cross(dir).resolved()
            && children_size.cross(dir).resolved()
            && f64::from(children_size.cross(dir)) < f64::from(size.cross(dir));

        if !size.width.resolved() || f64::from(size.width) < 0.0 {
            if self.scroll_x().is_none() && children_size.width.resolved() {
                size.width = children_size.width;
            } else if min_size.width.resolved() {
                size.width = min_size.width
            } else {
                size.width = Dimension::Px(10.0)
            }
        } else if allow_shrink_main
            && dir == Direction::Row
            && self.scroll_x().is_none()
            && children_size.width.resolved()
        {
            // Allow shrinking width for wrapping nodes with auto size
            size.width = children_size.width;
        }

        if !size.height.resolved() || f64::from(size.height) < 0.0 {
            if self.scroll_y().is_none() && children_size.height.resolved() {
                size.height = children_size.height;
            } else if min_size.height.resolved() {
                size.height = min_size.height
            } else {
                size.height = Dimension::Px(10.0)
            }
        } else if ((allow_shrink_main && dir == Direction::Column)
            || (allow_shrink_cross && dir == Direction::Row))
            && self.scroll_y().is_none()
            && children_size.height.resolved()
        {
            // Allow shrinking height for wrapping nodes with auto size
            size.height = children_size.height;
        }

        // Ensure the size is at least the min_size
        if !self.layout.size.width.resolved() {
            size.width = size.width.max(self.layout.min_size.width);
        }
        if !self.layout.size.height.resolved() {
            size.height = size.height.max(self.layout.min_size.height);
        }

        self.layout_result.size = size;
    }

    fn set_inner_scale(&mut self, children_size: Size) {
        if self.scrollable() {
            let inner_width = if self.scroll_x().is_some() {
                children_size.width.max(self.layout_result.size.width)
            } else {
                self.layout_result.size.width
            };
            let inner_height = if self.scroll_y().is_some() {
                children_size.height.max(self.layout_result.size.height)
            } else {
                self.layout_result.size.height
            };
            self.inner_scale = Some(crate::base_types::Scale {
                width: inner_width.into(),
                height: inner_height.into(),
            });
        }
    }

    /// For each axis in a node, it either has a size (or margin, or padding) in pixels,
    /// or its parent does (at the time of resolution). If a size axis is Auto, then
    /// it gets its size from its children, who must all have a resolved size on that axis.
    /// If it's children can not resolve its size, then it falls back to the min_size
    ///
    /// Wrapping cannot be performed on an axis that isn't resolved.
    ///
    /// A node that it scrollable on an axis must have a resolved size on that axis.
    fn resolve_layout(
        &mut self,
        bounds_size: Size,
        caches: &Caches,
        scale_factor: f32,
        final_pass: bool,
    ) {
        if cfg!(debug_assertions) && self.layout.debug.is_some() {
            log::debug!(
                "{} Laying out {} in bounds {:?}: {:#?}",
                if final_pass {
                    "Final pass"
                } else {
                    "First pass"
                },
                self.layout.debug.as_ref().unwrap(),
                &bounds_size,
                &self.layout,
            );
        }

        self.resolve_child_sizes(bounds_size, caches, scale_factor, final_pass);
        let children_size = self.set_children_position(bounds_size);
        self.resolve_size(children_size, final_pass);
        self.set_inner_scale(children_size);

        if !final_pass
            && (self.layout.size.main(self.layout.direction).resolved()
                || self
                    .children
                    .iter()
                    .all(|child| child.layout_result.main_resolved))
            && self.layout.flex_grow == 0.0
            && !self.layout.wrap
        {
            self.layout_result.main_resolved = true;
        }

        if cfg!(debug_assertions) && self.layout.debug.is_some() {
            log::debug!(
                "{} Layout result of {}: {:?}",
                if final_pass {
                    "Final pass"
                } else {
                    "First pass"
                },
                self.layout.debug.as_ref().unwrap(),
                &self.layout_result
            );
        }
    }

    pub(crate) fn calculate_layout(&mut self, caches: &Caches, scale_factor: f32) {
        self.layout_result.position = Bounds {
            top: Dimension::Px(0.0),
            left: Dimension::Px(0.0),
            bottom: Dimension::Auto,
            right: Dimension::Auto,
        };
        self.resolve_layout(self.layout.size, caches, scale_factor, false);
        // Layout is resolved twice, the second time to resolve percentages that couldn't have been known without better knowledge of the children
        self.resolve_layout(self.layout.size, caches, scale_factor, true);
    }
}

//--------------------------------
// MARK: Tests
//--------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Component;
    use crate::node;
    use crate::renderable::Caches;
    use crate::widgets::Div;
    use alloc::boxed::Box;

    /// A dummy widget for layout tests that always returns a fixed 100x100px size from `fill_bounds`.
    #[derive(Debug, Default)]
    struct FillBoundser;

    impl FillBoundser {
        fn new() -> Self {
            Self
        }
    }

    impl Component for FillBoundser {
        fn fill_bounds(
            &mut self,
            _width: Option<f32>,
            _height: Option<f32>,
            _max_width: Option<f32>,
            _max_height: Option<f32>,
            _caches: &Caches,
            _scale_factor: f32,
        ) -> (Option<f32>, Option<f32>) {
            (Some(100.0), Some(100.0))
        }
    }

    //---------------------------------------------------------------------------------

    /// A dummy widget for layout tests that always returns the provided width, unless it's None/0, in which case it returns 666px.
    #[derive(Debug, Default)]
    struct FillBoundsWithWidth;

    impl FillBoundsWithWidth {
        fn new() -> Self {
            Self
        }
    }

    impl Component for FillBoundsWithWidth {
        fn fill_bounds(
            &mut self,
            width: Option<f32>,
            _height: Option<f32>,
            max_width: Option<f32>,
            _max_height: Option<f32>,
            _caches: &Caches,
            _scale_factor: f32,
        ) -> (Option<f32>, Option<f32>) {
            let width = width.or(max_width).or(Some(666.0));
            if width.unwrap_or(0.0) == 0.0 {
                (Some(666.0), Some(100.0))
            } else {
                (width, Some(100.0))
            }
        }
    }

    //---------------------------------------------------------------------------------
    // Test cases

    #[test]
    fn test_empty() {
        let mut nodes = node!(Div::new(), lay!(size: size!(300.0)));
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        assert_eq!(nodes.layout_result.position.top, px!(0.0));
        assert_eq!(nodes.layout_result.position.left, px!(0.0));
    }

    #[test]
    fn test_fill_bounds_with_parent() {
        let mut nodes = node!(Div::new(), [size: [300.0, 300.0]]).push(
            // This node should become the size of its grandchild
            node!(Div::new(), [debug: "fake_button"])
                .push(node!(Div::new(), [size_pct: [100.0]]).push(node!(FillBoundser::new(), []))),
        );
        nodes.calculate_layout(&Caches::default(), 1.0);
        let fake_button = &nodes.children[0];
        let child = &fake_button.children[0];
        let grandchild = &child.children[0];
        assert_eq!(grandchild.layout_result.size, size!(100.0));
        assert_eq!(fake_button.layout_result.size, size!(100.0));
    }

    #[test]
    fn test_wrap() {
        let mut nodes = node!(
            Div::new(),
            lay!(size: size!(300.0), direction: Direction::Row, wrap: true)
        )
        .push(node!(Div::new(), lay!(size: size!(150.0))))
        .push(node!(Div::new(), lay!(size: size!(100.0))))
        .push(node!(Div::new(), lay!(size: size!(200.0))));
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        assert_eq!(nodes.children[0].layout_result.position.left, px!(0.0));
        assert_eq!(nodes.children[0].layout_result.position.top, px!(0.0));
        assert_eq!(nodes.children[1].layout_result.position.left, px!(150.0));
        assert_eq!(nodes.children[1].layout_result.position.top, px!(0.0));
        assert_eq!(nodes.children[2].layout_result.position.left, px!(0.0));
        assert_eq!(nodes.children[2].layout_result.position.top, px!(150.0));
    }

    #[test]
    fn test_wrap_with_auto_size_and_resolved_parent_and_sibling_sizes() {
        let mut nodes = node!(Div::new(), lay!(size: size!(300.0)))
            .push(node!(Div::new(), lay!(size: size!(100.0))))
            .push(
                // This node now has 200px to work with, so wrapping should be able to figure out the position of the children
                node!(Div::new(), lay!(wrap: true))
                    .push(node!(Div::new(), lay!(size: size!(100.0))))
                    .push(node!(Div::new(), lay!(size: size!(100.0))))
                    .push(node!(Div::new(), lay!(size: size!(200.0)))),
            );
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        let wrap_node = &nodes.children[1];
        assert_eq!(nodes.children[0].layout_result.position.left, px!(0.0));
        assert_eq!(nodes.children[0].layout_result.position.top, px!(0.0));
        assert_eq!(wrap_node.layout_result.position.left, px!(100.0));
        assert_eq!(wrap_node.layout_result.position.top, px!(0.0));
        assert_eq!(wrap_node.children[0].layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.children[0].layout_result.position.top, px!(0.0));
        assert_eq!(
            wrap_node.children[1].layout_result.position.left,
            px!(100.0)
        );
        assert_eq!(wrap_node.children[1].layout_result.position.top, px!(0.0));
        // This node should be wrapped to the next row
        assert_eq!(wrap_node.children[2].layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.children[2].layout_result.position.top, px!(100.0));
    }

    #[test]
    fn test_wrap_with_auto_size_and_unresolved_parent_and_resolved_sibling_sizes() {
        // ┌─────────────────────────────────────────┐
        // │ Root: 300px                             │
        // │ ┌─────────────────────────────────────┐ │
        // │ │ sub_root: 300px (100%)              │ │
        // │ │ ┌──────┐ ┌────────────────────────┐ │ │
        // │ │ │100px │ │ wrap_node: 200px       │ │ │
        // │ │ │sibl- │ │ ┌──────┐ ┌──────┐      │ │ │
        // │ │ │ing   │ │ │100px │ │100px │      │ │ │
        // │ │ │      │ │ └──────┘ └──────┘      │ │ │
        // │ │ │      │ │ ┌──────────────┐       │ │ │
        // │ │ │      │ │ │    200px     │       │ │ │
        // │ │ │      │ │ └──────────────┘       │ │ │
        // │ │ └──────┘ └────────────────────────┘ │ │
        // │ └─────────────────────────────────────┘ │
        // └─────────────────────────────────────────┘
        let mut nodes = node!(Div::new(), lay!(size: size!(300.0))).push(
            // We don't know the size of this node yet, but we do know it can't be larger than 300px
            node!(Div::new(), [])
                .push(node!(FillBoundser::new(), lay!()))
                .push(
                    // This node now has 200px to work with, so wrapping should be able to figure out the position of the children
                    node!(Div::new(), lay!(wrap: true, debug: "wrap_node"))
                        .push(node!(Div::new(), lay!(size: size!(100.0))))
                        .push(node!(Div::new(), lay!(size: size!(100.0))))
                        .push(node!(Div::new(), lay!(size: size!(200.0)))),
                ),
        );
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        let sub_root = &nodes.children[0];
        let wrap_node = &sub_root.children[1];
        assert_eq!(sub_root.children[0].layout_result.position.left, px!(0.0));
        assert_eq!(sub_root.children[0].layout_result.position.top, px!(0.0));
        assert_eq!(wrap_node.layout_result.position.left, px!(100.0));
        assert_eq!(wrap_node.layout_result.position.top, px!(0.0));
        assert_eq!(wrap_node.children[0].layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.children[0].layout_result.position.top, px!(0.0));
        assert_eq!(
            wrap_node.children[1].layout_result.position.left,
            px!(100.0)
        );
        assert_eq!(wrap_node.children[1].layout_result.position.top, px!(0.0));
        // This node should be wrapped to the next row
        assert_eq!(wrap_node.children[2].layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.children[2].layout_result.position.top, px!(100.0));
    }

    #[test]
    fn test_wrap_with_auto_size_and_unresolved_parent_with_sibling_and_unresolved_parent() {
        // ┌─────────────────────────────────────────┐
        // │ Root: 300px                             │
        // │ ┌─────────────────────────────────────┐ │
        // │ │ sub_root: Auto                      │ │
        // │ │ ┌──────┐ ┌────────────────────────┐ │ │
        // │ │ │100px │ │ wrap_parent: Auto      │ │ │
        // │ │ │sibl- │ │ ┌────────────────────┐ │ │ │
        // │ │ │ing   │ │ │ wrap_node: 200px   │ │ │ │
        // │ │ └──────┘ │ │ ┌──────┐ ┌──────┐  │ │ │ │
        // │ │          │ │ │100px │ │100px │  │ │ │ │
        // │ │          │ │ └──────┘ └──────┘  │ │ │ │
        // │ │          │ │ ┌──────┐ ┌──────┐  │ │ │ │
        // │ │          │ │ │100px │ │100px │  │ │ │ │
        // │ │          │ │ └──────┘ └──────┘  │ │ │ │
        // │ │          │ └────────────────────┘ │ │ │
        // │ │          └────────────────────────┘ │ │
        // │ └─────────────────────────────────────┘ │
        // └─────────────────────────────────────────┘
        let mut nodes = node!(Div::new(), lay!(size: size!(300.0))).push(
            // We don't know the size of this node yet, but we do know it can't be larger than 300px
            node!(Div::new(), lay!(debug: "sub_root"))
                .push(node!(FillBoundser::new(), lay!(debug: "fill_boundser")))
                .push(
                    node!(Div::new(), []).push(
                        // This node now has 200px to work with, so wrapping should be able to figure out the position of the children
                        node!(Div::new(), lay!(wrap: true, debug: "wrap_node"))
                            .push(node!(Div::new(), lay!(size: size!(100.0))))
                            .push(node!(Div::new(), lay!(size: size!(100.0))))
                            .push(node!(Div::new(), lay!(size: size!(100.0))))
                            .push(node!(Div::new(), lay!(size: size!(100.0)))),
                    ),
                ),
        );
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        let sub_root = &nodes.children[0];
        let wrap_sibling = &sub_root.children[0];
        let wrap_parent = &sub_root.children[1];
        let wrap_node = &wrap_parent.children[0];
        assert_eq!(wrap_parent.layout_result.position.left, px!(100.0));
        assert_eq!(wrap_parent.layout_result.position.top, px!(0.0));
        assert_eq!(wrap_parent.layout_result.size, size!(200.0, 200.0));
        assert_eq!(wrap_sibling.layout_result.position.left, px!(0.0));
        assert_eq!(wrap_sibling.layout_result.position.top, px!(0.0));
        assert_eq!(wrap_node.layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.layout_result.position.top, px!(0.0));
        assert_eq!(wrap_node.children[0].layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.children[0].layout_result.position.top, px!(0.0));
        assert_eq!(
            wrap_node.children[1].layout_result.position.left,
            px!(100.0)
        );
        assert_eq!(wrap_node.children[1].layout_result.position.top, px!(0.0));
        // This node should be wrapped to the next row
        assert_eq!(wrap_node.children[2].layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.children[2].layout_result.position.top, px!(100.0));
    }

    #[test]
    fn test_wrap_with_auto_size_and_unresolved_parent_with_solvable_sibling_and_unresolved_parent()
    {
        // ┌─────────────────────────────────────────┐
        // │ Root: 300px                             │
        // │ ┌─────────────────────────────────────┐ │
        // │ │ sub_root: Auto                      │ │
        // │ │ ┌──────┐ ┌────────────────────────┐ │ │
        // │ │ │100px │ │ wrap_parent: Auto      │ │ │
        // │ │ │sibl- │ │ ┌────────────────────┐ │ │ │
        // │ │ │ing   │ │ │ wrap_node: 200px   │ │ │ │
        // │ │ └──────┘ │ │ ┌──────┐ ┌──────┐  │ │ │ │
        // │ │          │ │ │100px │ │100px │  │ │ │ │
        // │ │          │ │ └──────┘ └──────┘  │ │ │ │
        // │ │          │ │ ┌──────┐ ┌──────┐  │ │ │ │
        // │ │          │ │ │100px │ │100px │  │ │ │ │
        // │ │          │ │ └──────┘ └──────┘  │ │ │ │
        // │ │          │ └────────────────────┘ │ │ │
        // │ │          └────────────────────────┘ │ │
        // │ └─────────────────────────────────────┘ │
        // └─────────────────────────────────────────┘
        let mut nodes = node!(Div::new(), lay!(size: size!(300.0))).push(
            // We don't know the size of this node yet, but we do know it can't be larger than 300px
            node!(Div::new(), lay!(debug: "sub_root"))
                .push(node!(Div::new()).push(node!(FillBoundser::new())))
                .push(
                    node!(Div::new(), []).push(
                        // This node now has 200px to work with, so wrapping should be able to figure out the position of the children
                        node!(Div::new(), lay!(wrap: true, debug: "wrap_node"))
                            .push(node!(Div::new(), lay!(size: size!(100.0))))
                            .push(node!(Div::new(), lay!(size: size!(100.0))))
                            .push(node!(Div::new(), lay!(size: size!(100.0))))
                            .push(node!(Div::new(), lay!(size: size!(100.0)))),
                    ),
                ),
        );
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        let sub_root = &nodes.children[0];
        let wrap_sibling = &sub_root.children[0];
        let wrap_parent = &sub_root.children[1];
        let wrap_node = &wrap_parent.children[0];
        assert_eq!(wrap_parent.layout_result.position.left, px!(100.0));
        assert_eq!(wrap_parent.layout_result.position.top, px!(0.0));
        assert_eq!(wrap_parent.layout_result.size, size!(200.0, 200.0));
        assert_eq!(wrap_sibling.layout_result.position.left, px!(0.0));
        assert_eq!(wrap_sibling.layout_result.position.top, px!(0.0));
        assert_eq!(wrap_node.layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.layout_result.position.top, px!(0.0));
        assert_eq!(wrap_node.children[0].layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.children[0].layout_result.position.top, px!(0.0));
        assert_eq!(
            wrap_node.children[1].layout_result.position.left,
            px!(100.0)
        );
        assert_eq!(wrap_node.children[1].layout_result.position.top, px!(0.0));
        // This node should be wrapped to the next row
        assert_eq!(wrap_node.children[2].layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.children[2].layout_result.position.top, px!(100.0));
    }

    #[test]
    fn test_wrap_with_auto_size_and_unresolved_parent_with_solvable_sibling_occurring_after_and_unresolved_parent()
     {
        // ┌─────────────────────────────────────────┐
        // │ Root: 300px                             │
        // │ ┌─────────────────────────────────────┐ │
        // │ │ sub_root: Auto                      │ │
        // │ │ ┌────────────────────────┐ ┌──────┐ │ │
        // │ │ │ wrap_parent: Auto      │ │100px │ │ │
        // │ │ │ ┌────────────────────┐ │ │sibl- │ │ │
        // │ │ │ │ wrap_node: 200px   │ │ │ing   │ │ │
        // │ │ │ │ ┌──────┐ ┌──────┐  │ │ └──────┘ │ │
        // │ │ │ │ │100px │ │100px │  │ │          │ │
        // │ │ │ │ └──────┘ └──────┘  │ │          │ │
        // │ │ │ │ ┌──────┐ ┌──────┐  │ │          │ │
        // │ │ │ │ │100px │ │100px │  │ │          │ │
        // │ │ │ │ └──────┘ └──────┘  │ │          │ │
        // │ │ │ └────────────────────┘ │          │ │
        // │ │ └────────────────────────┘          │ │
        // │ └─────────────────────────────────────┘ │
        // └─────────────────────────────────────────┘
        let mut nodes = node!(Div::new(), lay!(size: size!(300.0))).push(
            // We don't know the size of this node yet, but we do know it can't be larger than 300px
            node!(Div::new(), lay!(debug: "sub_root"))
                .push(
                    node!(Div::new(), [debug: "wrap_parent"]).push(
                        // This node now has 200px to work with, so wrapping should be able to figure out the position of the children
                        node!(Div::new(), lay!(wrap: true, debug: "wrap_node"))
                            .push(node!(Div::new(), lay!(size: size!(100.0))))
                            .push(node!(Div::new(), lay!(size: size!(100.0))))
                            .push(node!(Div::new(), lay!(size: size!(100.0))))
                            .push(node!(Div::new(), lay!(size: size!(100.0)))),
                    ),
                )
                .push(
                    node!(Div::new(), [debug: "sibling"])
                        .push(node!(FillBoundser::new(), [debug: "sibling_fill"])),
                ),
        );
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        let sub_root = &nodes.children[0];
        let wrap_parent = &sub_root.children[0];
        let wrap_sibling = &sub_root.children[1];
        let wrap_node = &wrap_parent.children[0];
        assert_eq!(wrap_parent.layout_result.position.left, px!(0.0));
        assert_eq!(wrap_parent.layout_result.position.top, px!(0.0));
        assert_eq!(wrap_parent.layout_result.size, size!(200.0, 200.0));
        assert_eq!(wrap_parent.layout_result.position.right, px!(200.0));
        assert_eq!(wrap_parent.layout_result.position.bottom, px!(200.0));
        assert_eq!(wrap_sibling.layout_result.position.left, px!(200.0));
        assert_eq!(wrap_sibling.layout_result.position.top, px!(0.0));
        assert_eq!(wrap_sibling.layout_result.size, size!(100.0));
        assert_eq!(wrap_node.layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.layout_result.position.top, px!(0.0));
        assert_eq!(wrap_node.children[0].layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.children[0].layout_result.position.top, px!(0.0));
        assert_eq!(
            wrap_node.children[1].layout_result.position.left,
            px!(100.0)
        );
        assert_eq!(wrap_node.children[1].layout_result.position.top, px!(0.0));
        // This node should be wrapped to the next row
        assert_eq!(wrap_node.children[2].layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.children[2].layout_result.position.top, px!(100.0));
    }

    #[test]
    fn test_wrap_with_auto_size_and_double_unresolved_parent_and_resolved_sibling_sizes() {
        // ┌─────────────────────────────────────────┐
        // │ Root: 300px                             │
        // │ ┌─────────────────────────────────────┐ │
        // │ │ sub_root: 300px (auto)              │ │
        // │ │ ┌─────────────────────────────────┐ │ │
        // │ │ │ sub_sub_root: 300px (auto)      │ │ │
        // │ │ │ ┌──────────────────┐ ┌──────┐   │ │ │
        // │ │ │ │ wrap_node: 200px │ │100px │   │ │ │
        // │ │ │ │ ┌──────┐ ┌──────┐│ │sibl- │   │ │ │
        // │ │ │ │ │100px │ │100px ││ │ing   │   │ │ │
        // │ │ │ │ └──────┘ └──────┘│ └──────┘   │ │ │
        // │ │ │ │ ┌──────┐         │            │ │ │
        // │ │ │ │ │100px │         │            │ │ │
        // │ │ │ │ └──────┘         │            │ │ │
        // │ │ │ └──────────────────┘            │ │ │
        // │ │ └─────────────────────────────────┘ │ │
        // │ └─────────────────────────────────────┘ │
        // └─────────────────────────────────────────┘
        let mut nodes = node!(Div::new(), lay!(size: size!(300.0))).push(
            // We don't know the size of this node yet, but we do know it can't be larger than 300px
            node!(Div::new(), []).push(
                node!(Div::new(), [])
                    .push(
                        // This node now has 200px to work with, so wrapping should be able to figure out the position of the children
                        node!(Div::new(), lay!(wrap: true, debug: "wrap_node"))
                            .push(node!(Div::new(), lay!(size: size!(100.0))))
                            .push(node!(Div::new(), lay!(size: size!(100.0))))
                            .push(node!(Div::new(), lay!(size: size!(100.0)))),
                    )
                    .push(node!(FillBoundser::new(), lay!())),
            ),
        );
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        let sub_root = &nodes.children[0];
        let sub_sub_root = &sub_root.children[0];
        let wrap_node = &sub_sub_root.children[0];
        assert_eq!(
            sub_sub_root.children[0].layout_result.position.left,
            px!(0.0)
        );
        assert_eq!(
            sub_sub_root.children[0].layout_result.position.top,
            px!(0.0)
        );
        assert_eq!(wrap_node.layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.layout_result.position.top, px!(0.0));
        assert_eq!(wrap_node.children[0].layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.children[0].layout_result.position.top, px!(0.0));
        assert_eq!(
            wrap_node.children[1].layout_result.position.left,
            px!(100.0)
        );
        assert_eq!(wrap_node.children[1].layout_result.position.top, px!(0.0));
        // This node should be wrapped to the next row
        assert_eq!(wrap_node.children[2].layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.children[2].layout_result.position.top, px!(100.0));
    }

    #[test]
    fn test_wrap_nested_with_auto_size() {
        // Layout structure:
        // Root (300px)
        // ├── sibling (100px) at x=0
        // └── outer_wrap_node (200px, auto size, wrap enabled) at x=100
        //     ├── child 0 (100px) at x=0, y=0
        //     ├── inner_wrap_node (200px, auto size, wrap enabled) at x=0, y=100 (wrapped to next row)
        //     │   ├── child 0 (100px) at x=0, y=0
        //     │   ├── child 1 (100px) at x=100, y=0
        //     │   └── child 2 (100px) at x=0, y=100 (wrapped to next row)
        //     └── child 2 (100px) at x=0, y=300 (wrapped to next row)
        //
        // ┌─────────────────────────────────────────┐
        // │ Root: 300px                             │
        // │ ┌──────┐ ┌────────────────────────────┐ │
        // │ │100px │ │ outer_wrap_node: 200px     │ │
        // │ │sibl- │ │ ┌──────┐                   │ │
        // │ │ing   │ │ │100px │                   │ │
        // │ │      │ │ │child0│                   │ │
        // │ │      │ │ └──────┘                   │ │
        // │ │      │ │ ┌────────────────────────┐ │ │
        // │ │      │ │ │ inner_wrap_node: 200px │ │ │
        // │ │      │ │ │ ┌──────┐ ┌──────┐      │ │ │
        // │ │      │ │ │ │100px │ │100px │      │ │ │
        // │ │      │ │ │ └──────┘ └──────┘      │ │ │
        // │ │      │ │ │ ┌──────┐               │ │ │
        // │ │      │ │ │ │100px │ (wrapped)     │ │ │
        // │ │      │ │ │ └──────┘               │ │ │
        // │ │      │ │ └────────────────────────┘ │ │
        // │ │      │ │ ┌──────┐                   │ │
        // │ │      │ │ │100px │                   │ │
        // │ │      │ │ │child2│                   │ │
        // │ │      │ │ └──────┘                   │ │
        // │ └──────┘ └────────────────────────────┘ │
        // └─────────────────────────────────────────┘
        let mut nodes = node!(Div::new(), [size: [300.0], debug: "root"])
            .push(node!(Div::new(), [size: [100.0], debug: "sibling"]))
            .push(
                // Outer wrap node should resolve to 200px (300 - 100 sibling)
                node!(Div::new(), lay!(wrap: true, debug: "outer_wrap_node"))
                    .push(node!(Div::new(), lay!(size: size!(100.0))))
                    .push(
                        // Inner wrap node should use outer_wrap_node's 200px
                        node!(Div::new(), lay!(wrap: true, debug: "inner_wrap_node"))
                            .push(node!(Div::new(), lay!(size: size!(100.0))))
                            .push(node!(Div::new(), lay!(size: size!(100.0))))
                            .push(node!(Div::new(), lay!(size: size!(100.0)))),
                    )
                    .push(node!(Div::new(), lay!(size: size!(100.0)))),
            );
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        let outer_wrap_node = &nodes.children[1];
        let inner_wrap_node = &outer_wrap_node.children[1];

        // Outer wrap node should be 200px (300 - 100 sibling)
        assert_eq!(outer_wrap_node.layout_result.size.width, px!(200.0));
        assert_eq!(outer_wrap_node.layout_result.position.left, px!(100.0));

        // Outer wrap node's children
        assert_eq!(
            outer_wrap_node.children[0].layout_result.position.left,
            px!(0.0)
        );
        assert_eq!(
            outer_wrap_node.children[0].layout_result.position.top,
            px!(0.0)
        );
        assert_eq!(inner_wrap_node.layout_result.position.left, px!(0.0));
        assert_eq!(inner_wrap_node.layout_result.position.top, px!(100.0));
        assert_eq!(
            outer_wrap_node.children[2].layout_result.position.left,
            px!(0.0)
        );
        assert_eq!(
            outer_wrap_node.children[2].layout_result.position.top,
            px!(300.0)
        );

        // Inner wrap node should be 200px (constrained by outer_wrap_node, not root)
        assert_eq!(inner_wrap_node.layout_result.size.width, px!(200.0));
        assert_eq!(inner_wrap_node.layout_result.position.left, px!(0.0));
        assert_eq!(inner_wrap_node.layout_result.position.top, px!(100.0));

        // Inner wrap node's children
        assert_eq!(
            inner_wrap_node.children[0].layout_result.position.left,
            px!(0.0)
        );
        assert_eq!(
            inner_wrap_node.children[0].layout_result.position.top,
            px!(0.0)
        );
        assert_eq!(
            inner_wrap_node.children[1].layout_result.position.left,
            px!(100.0)
        );
        assert_eq!(
            inner_wrap_node.children[1].layout_result.position.top,
            px!(0.0)
        );
        // Third child should wrap at 200px boundary (150 + 100 = 250 > 200)
        assert_eq!(
            inner_wrap_node.children[2].layout_result.position.left,
            px!(0.0)
        );
        assert_eq!(
            inner_wrap_node.children[2].layout_result.position.top,
            px!(100.0)
        );
    }

    #[test]
    fn test_wrap_nested_with_auto_size_and_multiple_unresolved_parents() {
        // Layout structure:
        // Root (300px)
        // └── sub_root (300px, auto size)
        //     └── sub_sub_root (300px, auto size)
        //         ├── sibling (100px) at x=0
        //         └── outer_wrap_node (200px, auto size, wrap enabled) at x=100
        //             ├── child 0 (100px) at x=0, y=0
        //             ├── inner_wrap_node (200px, auto size, wrap enabled) at x=0, y=100 (wrapped to next row)
        //             │   ├── child 0 (100px) at x=0, y=0
        //             │   ├── child 1 (100px) at x=100, y=0
        //             │   └── child 2 (100px) at x=0, y=100 (wrapped to next row)
        //             └── child 2 (100px) at x=0, y=300 (wrapped to next row)
        //
        // ┌─────────────────────────────────────────┐
        // │ Root: 300px                             │
        // │ ┌─────────────────────────────────────┐ │
        // │ │ sub_root: 300px (auto)              │ │
        // │ │ ┌─────────────────────────────────┐ │ │
        // │ │ │ sub_sub_root: 300px (auto)      │ │ │
        // │ │ │ ┌──────┐ ┌────────────────────┐ │ │ │
        // │ │ │ │100px │ │ outer_wrap_node:   │ │ │ │
        // │ │ │ │sibl- │ │ 200px              │ │ │ │
        // │ │ │ │ing   │ │ ┌──────┐           │ │ │ │
        // │ │ │ │      │ │ │100px │           │ │ │ │
        // │ │ │ │      │ │ │child0│           │ │ │ │
        // │ │ │ │      │ │ └──────┘           │ │ │ │
        // │ │ │ │      │ │ ┌────────────────┐ │ │ │ │
        // │ │ │ │      │ │ │ inner_wrap_node│ │ │ │ │
        // │ │ │ │      │ │ │ ┌──────┐ ┌────┐│ │ │ │ │
        // │ │ │ │      │ │ │ │100px │ │100 ││ │ │ │ │
        // │ │ │ │      │ │ │ └──────┘ └────┘│ │ │ │ │
        // │ │ │ │      │ │ │ ┌──────┐       │ │ │ │ │
        // │ │ │ │      │ │ │ │100px │(wrap) │ │ │ │ │
        // │ │ │ │      │ │ │ └──────┘       │ │ │ │ │
        // │ │ │ │      │ │ └────────────────┘ │ │ │ │
        // │ │ │ │      │ │ ┌──────┐           │ │ │ │
        // │ │ │ │      │ │ │100px │           │ │ │ │
        // │ │ │ │      │ │ │child2│           │ │ │ │
        // │ │ │ │      │ │ └──────┘           │ │ │ │
        // │ │ │ └──────┘ └────────────────────┘ │ │ │
        // │ │ └─────────────────────────────────┘ │ │
        // │ └─────────────────────────────────────┘ │
        // └─────────────────────────────────────────┘
        let mut nodes = node!(Div::new(), lay!(size: size!(300.0))).push(
            node!(Div::new(), lay!()).push(
                node!(Div::new(), lay!())
                    .push(node!(Div::new(), lay!(size: size!(100.0))))
                    .push(
                        // Outer wrap node should resolve to 200px (300 - 100 sibling)
                        node!(Div::new(), lay!(wrap: true, debug: "outer_wrap_node"))
                            .push(node!(Div::new(), lay!(size: size!(100.0))))
                            .push(
                                // Inner wrap node should use outer_wrap_node's 200px
                                node!(Div::new(), lay!(wrap: true, debug: "inner_wrap_node"))
                                    .push(node!(Div::new(), lay!(size: size!(100.0, 50.0))))
                                    .push(node!(Div::new(), lay!(size: size!(100.0, 50.0))))
                                    .push(node!(Div::new(), lay!(size: size!(100.0, 50.0)))),
                            )
                            .push(node!(Div::new(), lay!(size: size!(100.0)))),
                    ),
            ),
        );
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        let sub_root = &nodes.children[0];
        let sub_sub_root = &sub_root.children[0];
        let outer_wrap_node = &sub_sub_root.children[1];
        let inner_wrap_node = &outer_wrap_node.children[1];

        // Outer wrap node should be 200px (300 - 100 sibling)
        assert_eq!(outer_wrap_node.layout_result.size.width, px!(200.0));
        assert_eq!(outer_wrap_node.layout_result.position.left, px!(100.0));

        // Outer wrap node's children
        assert_eq!(
            outer_wrap_node.children[0].layout_result.position.left,
            px!(0.0)
        );
        assert_eq!(
            outer_wrap_node.children[0].layout_result.position.top,
            px!(0.0)
        );
        assert_eq!(inner_wrap_node.layout_result.position.left, px!(0.0));
        assert_eq!(inner_wrap_node.layout_result.position.top, px!(100.0));
        assert_eq!(
            outer_wrap_node.children[2].layout_result.position.left,
            px!(0.0)
        );
        assert_eq!(
            outer_wrap_node.children[2].layout_result.position.top,
            px!(200.0)
        );

        // Inner wrap node should be 200px (constrained by outer_wrap_node, not root)
        assert_eq!(inner_wrap_node.layout_result.size.width, px!(200.0));
        assert_eq!(inner_wrap_node.layout_result.position.left, px!(0.0));
        assert_eq!(inner_wrap_node.layout_result.position.top, px!(100.0));

        // Inner wrap node's children
        assert_eq!(
            inner_wrap_node.children[0].layout_result.position.left,
            px!(0.0)
        );
        assert_eq!(
            inner_wrap_node.children[0].layout_result.position.top,
            px!(0.0)
        );
        assert_eq!(
            inner_wrap_node.children[1].layout_result.position.left,
            px!(100.0)
        );
        assert_eq!(
            inner_wrap_node.children[1].layout_result.position.top,
            px!(0.0)
        );
        // Third child should wrap at 200px boundary (150 + 100 = 250 > 200)
        assert_eq!(
            inner_wrap_node.children[2].layout_result.position.left,
            px!(0.0)
        );
        assert_eq!(
            inner_wrap_node.children[2].layout_result.position.top,
            px!(50.0)
        );
    }

    #[test]
    fn test_wrap_with_auto_size_not_enough_children_to_wrap_and_unresolved_parent_and_resolved_sibling_sizes()
     {
        let mut nodes = node!(Div::new(), lay!(size: size!(300.0))).push(
            // We don't know the size of this node yet, but we do know it can't be larger than 300px
            node!(Div::new())
                .push(node!(Div::new(), lay!(size: size!(100.0))))
                .push(
                    // This node now has 200px to work with, but only one 100px child, so it shouldn't wrap, and its total size should be 100px
                    node!(Div::new(), lay!(wrap: true))
                        .push(node!(Div::new(), lay!(size: size!(100.0)))),
                ),
        );
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        let sub_root = &nodes.children[0];
        let wrap_node = &sub_root.children[1];
        assert_eq!(sub_root.children[0].layout_result.position.left, px!(0.0));
        assert_eq!(sub_root.children[0].layout_result.position.top, px!(0.0));
        assert_eq!(wrap_node.layout_result.position.left, px!(100.0));
        assert_eq!(wrap_node.layout_result.position.right, px!(200.0));
        assert_eq!(wrap_node.layout_result.position.top, px!(0.0));
        assert_eq!(wrap_node.children[0].layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.children[0].layout_result.position.top, px!(0.0));
    }

    #[test]
    fn test_wrap_with_column_parent() {
        // ┌─────────────────────────────────────────┐
        // │ Root: 300px (column)                    │
        // │ ┌─────────────────────────────────────┐ │
        // │ │ wrap_node (unfilled)                │ │
        // │ │ ┌──────┐                            │ │
        // │ │ │100px │                            │ │
        // │ │ └──────┘                            │ │
        // │ └─────────────────────────────────────┘ │
        // └─────────────────────────────────────────┘
        let mut nodes = node!(
            Div::new(),
            lay!(size: size!(300.0), direction: Direction::Column, debug: "root")
        )
        .push(
            node!(Div::new(), lay!(wrap: true, debug: "wrap_node"))
                .push(node!(Div::new(), lay!(size: size!(100.0)))),
        );
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        let wrap_node = &nodes.children[0];
        assert_eq!(wrap_node.layout_result.size, size!(100.0, 100.0));
        assert_eq!(wrap_node.layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.layout_result.position.top, px!(0.0));
        assert_eq!(wrap_node.children[0].layout_result.position.left, px!(0.0));
        assert_eq!(wrap_node.children[0].layout_result.position.top, px!(0.0));
    }

    #[test]
    fn test_wrap_margins_and_padding() {
        // ┌───────────────────────────────────────┐
        // │ Root: 300px (Row, wrap, padding: 1%)  │
        // │                                       │
        // │  ┌──────────┐ ┌────────┐              │
        // │  │ 150px    │ │ 100px  │              │
        // │  │ (margin: │ │(margin:│              │
        // │  │  1%)     │ │  1%)   │              │
        // │  └──────────┘ └────────┘              │
        // │  ┌──────────────┐                     │
        // │  │    200px     │                     │
        // │  │  (margin:    │                     │
        // │  │    1%)       │                     │
        // │  └──────────────┘                     │
        // └───────────────────────────────────────┘
        let mut nodes = node!(
            Div::new(),
            lay!(size: size!(300.0), direction: Direction::Row, wrap: true, padding: bounds_pct!(1.0))
        )
        .push(node!(
            Div::new(),
            lay!(size: size!(150.0), margin: bounds_pct!(1.0))
        ))
        .push(node!(
            Div::new(),
            lay!(size: size!(100.0), margin: bounds_pct!(1.0))
        ))
        .push(node!(
            Div::new(),
            lay!(size: size!(200.0), margin: bounds_pct!(1.0))
        ));
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        assert_eq!(
            nodes.children[0].layout_result.position.left,
            px!(3.0 + 3.0)
        );
        assert_eq!(nodes.children[0].layout_result.position.top, px!(3.0 + 3.0));
        assert_eq!(
            nodes.children[1].layout_result.position.left,
            px!((3.0 * 4.0) + 150.0)
        );
        assert_eq!(nodes.children[1].layout_result.position.top, px!(3.0 + 3.0));
        // Wrapped
        assert_eq!(
            nodes.children[2].layout_result.position.left,
            px!(3.0 + 3.0)
        );
        assert_eq!(
            nodes.children[2].layout_result.position.top,
            px!((3.0 * 4.0) + 150.0)
        );
    }

    #[test]
    fn test_wrap_margins_and_padding_when_shrinking() {
        // ┌─────────────────────────────────────────┐
        // │ Root: 300px                             │
        // │ ┌──────────────────────────┐            │
        // │ │ Wrapping node; 2px pad   │            │
        // │ │ ┌────────────┐ ┌────────┐│            │
        // │ │ │ 150px      │ │ 100px  ││            │
        // │ │ │ (margin:   │ │(margin:││            │
        // │ │ │  1px)      │ │ 1px)   ││            │
        // │ │ │            │ └────────┘│            │
        // │ │ │            │           │            │
        // │ │ └────────────┘           │            │
        // │ └──────────────────────────┘            │
        // │                                         │
        // │                                         │
        // │                                         │
        // │                                         │
        // │                                         │
        // └─────────────────────────────────────────┘
        let mut nodes = node!(
            Div::new(),
            [size: [300.0]]
        )
        .push(
            node!(
                Div::new(),
                [direction: Direction::Row, wrap: true, margin: [2.0], padding: [2.0], debug: "wrapping_node"]

            )
            .push(node!(
                Div::new(),
                [size: [150.0], margin: [1.0]]
            ))
            .push(node!(
                Div::new(),
                [size: [100.0], margin: [1.0]]
            )),
        );
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        let wrapping_node = &nodes.children[0];
        assert_eq!(wrapping_node.layout_result.size, size!(258.0, 156.0));
        assert_eq!(wrapping_node.layout_result.position.left, px!(2.0));
        assert_eq!(wrapping_node.layout_result.position.top, px!(2.0));
        let child1 = &wrapping_node.children[0];
        let child2 = &wrapping_node.children[1];
        assert_eq!(child1.layout_result.position.left, px!(3.0));
        assert_eq!(child1.layout_result.position.top, px!(3.0));
        assert_eq!(
            child2.layout_result.position.left,
            px!(3.0 + 150.0 + 1.0 + 1.0)
        ); // 3 (child1 pos) + 150 (child1 width) + 1 (child1 right margin) + 1 (child2 left margin) = 155px
        assert_eq!(child2.layout_result.position.top, px!(3.0));
    }

    #[test]
    fn test_bounds_propagation_with_multiple_undefined_parents() {
        // ┌─────────────────────────────────────────┐
        // │ Root: 300px                             │
        // │ ┌─────────────────────────────────────┐ │
        // │ │ sub_root: 300px (auto)              │ │
        // │ │ ┌─────────────────────────────────┐ │ │
        // │ │ │ sub_sub_root: 300px (auto)      │ │ │
        // │ │ │ ┌──────────────────────────────┐│ │ │
        // │ │ │ │ fill_bounds_with_width       ││ │ │
        // │ │ │ └──────────────────────────────┘│ │ │
        // │ │ └─────────────────────────────────┘ │ │
        // │ └─────────────────────────────────────┘ │
        // └─────────────────────────────────────────┘
        let mut nodes = node!(Div::new(), lay!(size: size!(300.0))).push(
            // We don't know the size of this node yet, but we do know it can't be larger than 300px
            node!(Div::new(), []).push(node!(Div::new(), []).push(node!(
                FillBoundsWithWidth::new(),
                lay!(debug: "fill_bounds_with_width")
            ))),
        );
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        let sub_root = &nodes.children[0];
        let sub_sub_root = &sub_root.children[0];
        let fill_bounds_with_width = &sub_sub_root.children[0];
        assert_eq!(fill_bounds_with_width.layout_result.position.left, px!(0.0));
        assert_eq!(fill_bounds_with_width.layout_result.position.top, px!(0.0));
        assert_eq!(
            fill_bounds_with_width.layout_result.size,
            // FillBoundsWithWidth returns a fixed 100px height and the maximum width provided, unless it's None/0, in which case it returns 666px.
            size!(300.0, 100.0)
        );
    }

    #[test]
    fn test_flex_grow() {
        // ┌─────────────────────────────────────────┐
        // │ Root: 300px × 400px                     │
        // │ ┌─────────────────┐                     │
        // │ │ sibling1: 100px │                     │
        // │ │                 │                     │
        // │ │                 │                     │
        // │ └─────────────────┘                     │
        // │ ┌─────────────────────────────────────┐ │
        // │ │ sibling2: 100% × Auto               │ │
        // │ │ ┌───────────────┐                   │ │
        // │ │ │ child: 100px  │                   │ │
        // │ │ └───────────────┘                   │ │
        // │ └─────────────────────────────────────┘ │
        // │ ┌─────────────────────────────────────┐ │
        // │ │ remaining: 100% × Auto              │ │
        // │ │                                     │ │
        // │ │                                     │ │
        // │ │                                     │ │
        // │ │                                     │ │
        // │ │                                     │ │
        // │ │                                     │ │
        // │ │                                     │ │
        // │ │                                     │ │
        // │ └─────────────────────────────────────┘ │
        // └─────────────────────────────────────────┘
        let mut nodes = node!(
            Div::new(),
            [size: [300.0, 400.0], direction: Direction::Column, axis_alignment: Alignment::Stretch, cross_alignment: Alignment::Stretch, debug: "root"]
        )
        .push(node!(
            FillBoundser::new(),
            [debug: "sibling1"]
        ))
        .push(node!(
            Div::new(),
            [size_pct: [100.0, Auto], flex_grow: 0.0, debug: "sibling2"]
                     ).push(node!(
                 FillBoundser::new(),
                 [size: [100.0, 100.0]]
        )))
        .push(
            node!(
                Div::new(),
                [size_pct: [100.0, Auto], debug: "remaining"]
            )
        );
        nodes.calculate_layout(&Caches::default(), 1.0);

        // Root should be 300px × 300px
        assert_eq!(nodes.layout_result.size, size!(300.0, 400.0));

        // First child (sibling) should be 100px × 100px
        let sibling1 = &nodes.children[0];
        assert_eq!(sibling1.layout_result.size, size!(100.0, 100.0));
        assert_eq!(sibling1.layout_result.position.top, px!(0.0));
        assert_eq!(sibling1.layout_result.position.left, px!(0.0));

        // Second child (sibling) should be 300px × 100px
        let sibling2 = &nodes.children[1];
        assert_eq!(sibling2.layout_result.size, size!(300.0, 100.0));
        assert_eq!(sibling2.layout_result.position.top, px!(100.0));
        assert_eq!(sibling2.layout_result.position.left, px!(0.0));

        // Remaining node should be 300px × 200px (remaining space)
        let remaining = &nodes.children[2];
        assert_eq!(remaining.layout_result.size, size!(300.0, 200.0));
        assert_eq!(remaining.layout_result.position.top, px!(200.0));
        assert_eq!(remaining.layout_result.position.left, px!(0.0));
    }

    #[test]
    fn test_flex_grow_with_different_weights() {
        // ┌─────────────────────────────────────────┐
        // │ Root: 300px × 400px                     │
        // │ ┌─────────────────────────────────────┐ │
        // │ │ fixed1: 100px × 50px                │ │
        // │ └─────────────────────────────────────┘ │
        // │ ┌─────────────────────────────────────┐ │
        // │ │ grow1 (flex_grow: 1): 100px         │ │
        // │ │ (should get 1/4 of remaining space) │ │
        // │ └─────────────────────────────────────┘ │
        // │ ┌─────────────────────────────────────┐ │
        // │ │ grow2 (flex_grow: 2): 200px         │ │
        // │ │ (should get 2/4 of remaining space) │ │
        // │ └─────────────────────────────────────┘ │
        // │ ┌─────────────────────────────────────┐ │
        // │ │ grow3 (flex_grow: 1): 100px         │ │
        // │ │ (should get 1/4 of remaining space) │ │
        // │ └─────────────────────────────────────┘ │
        // │ ┌─────────────────────────────────────┐ │
        // │ │ fixed2: 100px × 50px                │ │
        // │ └─────────────────────────────────────┘ │
        // └─────────────────────────────────────────┘
        // Total: 50 + 100 + 200 + 100 + 50 = 500px (but container is 400px)
        // Remaining after fixed: 400 - 50 - 50 = 300px
        // Distribution: grow1=75px, grow2=150px, grow3=75px (1:2:1 ratio)
        let mut nodes = node!(
            Div::new(),
            [size: [300.0, 400.0], direction: Direction::Column, axis_alignment: Alignment::Stretch]
        )
        .push(node!(
            FillBoundser::new(),
            [size: [100.0, 50.0], flex_grow: 0.0, debug: "fixed1"]
        ))
        .push(node!(
            Div::new(),
            [size_pct: [100.0, Auto], flex_grow: 1.0, debug: "grow1"]
        ))
        .push(node!(
            Div::new(),
            [size_pct: [100.0, Auto], flex_grow: 2.0, debug: "grow2"]
        ))
        .push(node!(
            Div::new(),
            [size_pct: [100.0, Auto], flex_grow: 1.0, debug: "grow3"]
        ))
        .push(node!(
            FillBoundser::new(),
            // Because it's fixed size, flex_grow is ignored
            [size: [90.0, 40.0], margin: [5.0], flex_grow: 1.0, debug: "fixed2"]
            // Previous value
            // [size: [100.0, 50.0], flex_grow: 1.0, debug: "fixed2"]
        ));
        nodes.calculate_layout(&Caches::default(), 1.0);

        // Root should be 300px × 400px
        assert_eq!(nodes.layout_result.size, size!(300.0, 400.0));

        // Fixed1 should be 100px × 50px
        let fixed1 = &nodes.children[0];
        assert_eq!(fixed1.layout_result.size, size!(100.0, 50.0));
        assert_eq!(fixed1.layout_result.position.top, px!(0.0));

        // Grow1 should be 300px × 75px (1/4 of 300px remaining)
        let grow1 = &nodes.children[1];
        assert_eq!(grow1.layout_result.size, size!(300.0, 75.0));
        assert_eq!(grow1.layout_result.position.top, px!(50.0));

        // Grow2 should be 300px × 150px (2/4 of 300px remaining)
        let grow2 = &nodes.children[2];
        assert_eq!(grow2.layout_result.size, size!(300.0, 150.0));
        assert_eq!(grow2.layout_result.position.top, px!(125.0));

        // Grow3 should be 300px × 75px (1/4 of 300px remaining)
        let grow3 = &nodes.children[3];
        assert_eq!(grow3.layout_result.size, size!(300.0, 75.0));
        assert_eq!(grow3.layout_result.position.top, px!(275.0));

        // Fixed2 should be 90px × 40px (the specified size, margin is separate)
        // Position is 355px because it includes the 5px top margin (350 + 5)
        let fixed2 = &nodes.children[4];
        assert_eq!(fixed2.layout_result.size, size!(90.0, 40.0));
        assert_eq!(fixed2.layout_result.position.top, px!(355.0));

        // Verify total height: 50 + 75 + 150 + 75 + 40 = 390px (content sizes only, margins not included in sum)
        let total_height = match (
            fixed1.layout_result.size.height,
            grow1.layout_result.size.height,
            grow2.layout_result.size.height,
            grow3.layout_result.size.height,
            fixed2.layout_result.size.height,
        ) {
            (
                Dimension::Px(h1),
                Dimension::Px(h2),
                Dimension::Px(h3),
                Dimension::Px(h4),
                Dimension::Px(h5),
            ) => f64::from(h1) + f64::from(h2) + f64::from(h3) + f64::from(h4) + f64::from(h5),
            _ => panic!("All heights should be resolved"),
        };
        assert_eq!(total_height, 390.0);
    }

    #[test]
    fn test_pct() {
        let mut nodes = node!(Div::new(), lay!(size: size!(300.0))).push(
            node!(Div::new(), lay!(size: size_pct!(50.0, 100.0)))
                .push(node!(Div::new(), lay!(size: size_pct!(50.0, 100.0)))),
        );
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        assert_eq!(nodes.children[0].layout_result.size, size!(150.0, 300.0));
        assert_eq!(
            nodes.children[0].children[0].layout_result.size,
            size!(75.0, 300.0)
        );
    }

    #[test]
    fn test_pct_from_sibling() {
        let mut nodes = node!(
            Div::new(),
            lay!(size: size!(Auto), direction: Direction::Column)
        )
        .push(node!(Div::new(), lay!(size: size!(50.0, 100.0))))
        .push(node!(
            Div::new(),
            lay!(size: Size {width: Dimension::Pct(100.0), height: Dimension::Px(50.0)})
        ));
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(50.0, 150.0));
        assert_eq!(nodes.children[0].layout_result.size, size!(50.0, 100.0));
        assert_eq!(nodes.children[1].layout_result.size, size!(50.0, 50.0));
    }

    #[test]
    fn test_stretch() {
        let mut nodes = node!(
            Div::new(),
            lay!(
                size: size!(300.0),
                direction: Direction::Row,
                axis_alignment: Alignment::Stretch,
                cross_alignment: Alignment::Stretch,
            )
        )
        .push(node!(Div::new()))
        .push(node!(Div::new()));
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        assert_eq!(nodes.children[0].layout_result.size, size!(150.0, 300.0));
        assert_eq!(nodes.children[0].layout_result.position.left, px!(0.0));
        assert_eq!(nodes.children[0].layout_result.position.top, px!(0.0));
        assert_eq!(nodes.children[1].layout_result.size, size!(150.0, 300.0));
        assert_eq!(nodes.children[1].layout_result.position.left, px!(150.0));
        assert_eq!(nodes.children[1].layout_result.position.top, px!(0.0));
    }

    #[test]
    fn test_stretch_with_resolved_nodes() {
        let mut nodes = node!(
            Div::new(),
            lay!(
                size: size!(300.0),
                direction: Direction::Row,
                axis_alignment: Alignment::Stretch,
                cross_alignment: Alignment::Stretch,
            )
        )
        .push(node!(Div::new()))
        .push(node!(Div::new(), lay!(size: size!(100.0))));
        nodes.calculate_layout(&Caches::default(), 1.0);

        assert_eq!(nodes.layout_result.size, size!(300.0));
        assert_eq!(nodes.children[0].layout_result.size, size!(200.0, 300.0));
        assert_eq!(nodes.children[0].layout_result.position.left, px!(0.0));
        assert_eq!(nodes.children[0].layout_result.position.top, px!(0.0));
        assert_eq!(nodes.children[1].layout_result.size, size!(100.0, 100.0));
        assert_eq!(nodes.children[1].layout_result.position.left, px!(200.0));
        assert_eq!(nodes.children[1].layout_result.position.top, px!(0.0));
    }

    #[test]
    fn test_padding() {
        let mut nodes = node!(
            Div::new(),
            lay!(size: size!(300.0), padding: bounds!(10.0, 20.0, 30.0, 40.0))
        )
        .push(node!(Div::new(), lay!(size: size_pct!(100.0, 100.0))));
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.children[0].layout_result.size, size!(240.0, 260.0));
        assert_eq!(nodes.children[0].layout_result.position.left, px!(20.0));
        assert_eq!(nodes.children[0].layout_result.position.top, px!(10.0));
    }

    #[test]
    fn test_padding_pct() {
        let mut nodes = node!(
            Div::new(),
            lay!(
                size: size!(300.0),
                padding: bounds_pct!(10.0, 20.0, 30.0, 40.0)
            )
        )
        .push(node!(Div::new(), lay!(size: size_pct!(100.0, 100.0))));
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.children[0].layout_result.size, size!(120.0, 180.0));
        assert_eq!(nodes.children[0].layout_result.position.left, px!(60.0));
        assert_eq!(nodes.children[0].layout_result.position.top, px!(30.0));
    }

    #[test]
    fn test_margin() {
        let mut nodes = node!(Div::new(), lay!(size: size!(300.0)))
            .push(node!(
                Div::new(),
                lay!(
                    size: size_pct!(50.0, 100.0),
                    margin: bounds!(5.0, 10.0, 15.0, 20.0)
                )
            ))
            .push(node!(
                Div::new(),
                lay!(
                    size: size_pct!(50.0, 100.0),
                    margin: bounds!(15.0, 10.0, 5.0, 20.0)
                )
            ));
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.children[0].layout_result.size, size!(120.0, 280.0));
        assert_eq!(nodes.children[0].layout_result.position.left, px!(10.0));
        assert_eq!(nodes.children[0].layout_result.position.top, px!(5.0));
        assert_eq!(nodes.children[1].layout_result.size, size!(120.0, 280.0));
        assert_eq!(nodes.children[1].layout_result.position.left, px!(160.0));
        assert_eq!(nodes.children[1].layout_result.position.top, px!(15.0));
    }

    #[test]
    fn test_margin_pct() {
        let mut nodes = node!(Div::new(), lay!(size: size!(300.0)))
            .push(node!(
                Div::new(),
                lay!(
                    size: size_pct!(50.0, 100.0),
                    margin: bounds_pct!(5.0, 10.0, 15.0, 20.0),
                    debug: "child1"
                )
            ))
            .push(node!(
                Div::new(),
                lay!(
                    size: size_pct!(50.0, 100.0),
                    margin: bounds_pct!(15.0, 10.0, 5.0, 20.0),
                    debug: "child2"
                )
            ));
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.children[0].layout_result.size, size!(60.0, 240.0));
        assert_eq!(nodes.children[0].layout_result.position.left, px!(30.0));
        assert_eq!(nodes.children[0].layout_result.position.top, px!(15.0));
        assert_eq!(nodes.children[1].layout_result.size, size!(60.0, 240.0));
        assert_eq!(nodes.children[1].layout_result.position.left, px!(180.0));
        assert_eq!(nodes.children[1].layout_result.position.top, px!(45.0));
    }

    #[test]
    fn test_auto() {
        let mut nodes = node!(
            Div::new(),
            lay!(direction: Direction::Row, padding: bounds!(10.0))
        )
        .push(node!(Div::new(), lay!(size: size!(150.0))))
        .push(node!(Div::new(), lay!(size: size!(100.0))))
        .push(node!(
            Div::new(),
            lay!(size: size!(200.0), margin: bounds!(2.0))
        ));
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(
            nodes.layout_result.size,
            size!(
                10.0 + 150.0 + 100.0 + 2.0 + 200.0 + 2.0 + 10.0,
                10.0 + 2.0 + 200.0 + 2.0 + 10.0
            )
        );
    }

    #[test]
    fn test_auto_no_children() {
        let mut nodes = node!(
            Div::new(),
            lay!(direction: Direction::Row, min_size: size!(250.0, 300.0))
        );
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(250.0, 300.0));
    }

    #[test]
    fn test_min_size_with_auto_size_and_small_children() {
        // Node with auto size, min_size, and children smaller than min_size
        // Should result in min_size
        let mut nodes = node!(
            Div::new(),
            lay!(direction: Direction::Row, min_size: size!(250.0, 200.0))
        )
        .push(node!(Div::new(), lay!(size: size!(50.0, 50.0))))
        .push(node!(Div::new(), lay!(size: size!(50.0, 50.0))));
        nodes.calculate_layout(&Caches::default(), 1.0);
        // Children total width is 100px, but min_size is 250px, so should be 250px
        // Children total height is 50px, but min_size is 200px, so should be 200px
        assert_eq!(nodes.layout_result.size, size!(250.0, 200.0));
    }

    #[test]
    fn test_min_size_with_auto_size_and_large_children() {
        // Node with auto size, min_size, and children larger than min_size
        // Should result in children's size (min_size doesn't expand beyond content)
        let mut nodes = node!(
            Div::new(),
            lay!(direction: Direction::Row, min_size: size!(100.0, 100.0))
        )
        .push(node!(Div::new(), lay!(size: size!(150.0, 150.0))))
        .push(node!(Div::new(), lay!(size: size!(100.0, 100.0))));
        nodes.calculate_layout(&Caches::default(), 1.0);
        // Children total width is 250px (larger than min_size 100px), so should be 250px
        // Children total height is 150px (larger than min_size 100px), so should be 150px
        assert_eq!(nodes.layout_result.size, size!(250.0, 150.0));
    }

    #[test]
    fn test_min_size_width_only_with_auto_size() {
        // Node with auto width, min_width, and children smaller than min_width
        let mut nodes = node!(
            Div::new(),
            lay!(direction: Direction::Row, min_size: size!(200.0, Auto))
        )
        .push(node!(Div::new(), lay!(size: size!(50.0, 100.0))));
        nodes.calculate_layout(&Caches::default(), 1.0);
        // Width should be at least 200px (min_size), height should be from children (100px)
        assert_eq!(nodes.layout_result.size, size!(200.0, 100.0));
    }

    #[test]
    fn test_min_size_height_only_with_auto_size() {
        // Node with auto height, min_height, and children smaller than min_height
        let mut nodes = node!(
            Div::new(),
            lay!(direction: Direction::Column, min_size: size!(Auto, 200.0))
        )
        .push(node!(Div::new(), lay!(size: size!(100.0, 50.0))));
        nodes.calculate_layout(&Caches::default(), 1.0);
        // Width should be from children (100px), height should be at least 200px (min_size)
        assert_eq!(nodes.layout_result.size, size!(100.0, 200.0));
    }

    #[test]
    fn test_min_size_with_resolved_size() {
        // Node with resolved size smaller than min_size
        // min_size should NOT override resolved size (only applies when size is Auto)
        let mut nodes = node!(
            Div::new(),
            lay!(size: size!(50.0, 50.0), min_size: size!(200.0, 200.0))
        );
        nodes.calculate_layout(&Caches::default(), 1.0);
        // Should respect the resolved size, not min_size
        assert_eq!(nodes.layout_result.size, size!(50.0, 50.0));
    }

    #[test]
    fn test_min_size_with_column_direction() {
        // Test min_size with Column direction
        let mut nodes = node!(
            Div::new(),
            lay!(direction: Direction::Column, min_size: size!(150.0, 250.0))
        )
        .push(node!(Div::new(), lay!(size: size!(50.0, 50.0))))
        .push(node!(Div::new(), lay!(size: size!(50.0, 50.0))));
        nodes.calculate_layout(&Caches::default(), 1.0);
        // Width should be at least 150px (min_size), height should be at least 250px (min_size)
        // Children width is 50px, so width becomes 150px
        // Children height is 100px, so height becomes 250px
        assert_eq!(nodes.layout_result.size, size!(150.0, 250.0));
    }

    #[test]
    fn test_min_size_with_wrapping() {
        // Test min_size with wrapping enabled
        let mut nodes = node!(
            Div::new(),
            lay!(
                direction: Direction::Row,
                wrap: true,
                min_size: size!(300.0, 200.0)
            )
        )
        .push(node!(Div::new(), lay!(size: size!(50.0, 50.0))))
        .push(node!(Div::new(), lay!(size: size!(50.0, 50.0))));
        nodes.calculate_layout(&Caches::default(), 1.0);
        // With wrapping, children should fit in one row (50 + 50 = 100px width)
        // But min_size is 300px, so width should be 300px
        // Height should be at least 200px (min_size), but children are 50px tall
        assert_eq!(nodes.layout_result.size, size!(300.0, 200.0));
    }

    #[test]
    fn test_min_size_partial_auto() {
        // Node with auto width but resolved height, and min_size
        let mut nodes = node!(
            Div::new(),
            lay!(size: size!(Auto, 100.0), min_size: size!(200.0, 150.0))
        )
        .push(node!(Div::new(), lay!(size: size!(50.0, 50.0))));
        nodes.calculate_layout(&Caches::default(), 1.0);
        // Width is Auto, so min_size.width (200px) should apply
        // Height is resolved (100px), so min_size.height should NOT apply
        assert_eq!(nodes.layout_result.size, size!(200.0, 100.0));
    }

    #[test]
    fn test_end_alignment() {
        let mut nodes = node!(
            Div::new(),
            lay!(size: size!(300.0), direction: Direction::Row,
                 wrap: true, axis_alignment: Alignment::End, cross_alignment: Alignment::End)
        )
        .push(node!(Div::new(), lay!(size: size!(150.0)))) // Child 0
        .push(node!(Div::new(), lay!(size: size!(100.0)))) // Child 1
        .push(node!(Div::new(), lay!(size: size!(200.0)))); // Child 2

        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));

        assert_eq!(nodes.children[0].layout_result.position.right, px!(300.0));
        assert_eq!(nodes.children[0].layout_result.position.bottom, px!(100.0));
        assert_eq!(nodes.children[0].layout_result.position.left, px!(150.0));
        assert_eq!(nodes.children[0].layout_result.position.top, px!(-50.0));

        assert_eq!(nodes.children[1].layout_result.position.right, px!(100.0));
        assert_eq!(nodes.children[1].layout_result.position.bottom, px!(300.0));

        assert_eq!(nodes.children[2].layout_result.position.right, px!(300.0));
        assert_eq!(nodes.children[2].layout_result.position.bottom, px!(300.0));
    }

    #[test]
    fn test_center_alignment() {
        let mut nodes = node!(
            Div::new(),
            lay!(size: size!(415.0), // This is just small enough to force a wrap
                 direction: Direction::Row,
                 padding: bounds!(5.0), wrap: true,
                 axis_alignment: Alignment::Center, cross_alignment: Alignment::Center)
        )
        .push(node!(
            Div::new(),
            lay!(size: size!(100.0), margin: bounds!(1.0))
        ))
        .push(node!(
            Div::new(),
            lay!(size: size!(200.0), margin: bounds!(1.0))
        ))
        .push(node!(
            Div::new(),
            lay!(size: size!(100.0), margin: bounds!(1.0))
        ));
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(415.0));
        assert_eq!(nodes.children[0].layout_result.position.left, px!(56.5));
        assert_eq!(nodes.children[0].layout_result.position.top, px!(56.5));
        assert_eq!(nodes.children[1].layout_result.position.left, px!(158.5));
        assert_eq!(nodes.children[1].layout_result.position.top, px!(56.5));
        assert_eq!(nodes.children[2].layout_result.position.left, px!(157.5));
        assert_eq!(nodes.children[2].layout_result.position.top, px!(258.5));
    }

    #[test]
    fn test_absolute_positioning() {
        let mut nodes = node!(
            Div::new(),
            lay!(size: size!(300.0), direction: Direction::Row, wrap: true)
        )
        .push(node!(Div::new(), lay!(size: size!(150.0)))) // Child 0
        .push(node!(Div::new(), lay!(size: size!(100.0)))) // Child 1
        .push(node!(Div::new(), lay!(size: size!(200.0)))) // Child 2
        .push(node!(
            // Child 3
            Div::new(),
            lay!(
                size: size!(100.0),
                position_type: PositionType::Absolute,
                position: bounds!(Auto, Auto, 10.0, 10.0)
            )
        ));
        nodes.calculate_layout(&Caches::default(), 1.0);
        assert_eq!(nodes.layout_result.size, size!(300.0));
        assert_eq!(nodes.children[0].layout_result.position.left, px!(0.0));
        assert_eq!(nodes.children[0].layout_result.position.top, px!(0.0));
        assert_eq!(nodes.children[1].layout_result.position.left, px!(150.0));
        assert_eq!(nodes.children[1].layout_result.position.top, px!(0.0));
        assert_eq!(nodes.children[2].layout_result.position.left, px!(0.0));
        assert_eq!(nodes.children[2].layout_result.position.top, px!(150.0));
        assert_eq!(nodes.children[3].layout_result.position.left, px!(190.0));
        assert_eq!(nodes.children[3].layout_result.position.top, px!(190.0));
    }
}
