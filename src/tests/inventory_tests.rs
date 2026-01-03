#[cfg(test)]
mod tests {
    use crate::plugins::inventory::{InventoryGridState, ItemSize};
    use bevy::prelude::*;

    #[test]
    fn test_grid_bounds() {
        let grid = InventoryGridState {
            width: 8,
            height: 8,
            ..default()
        };

        let size = ItemSize { width: 2, height: 2 };

        // Valid position (0,0)
        assert!(grid.is_area_free(IVec2::new(0, 0), size, None));

        // Invalid position (out of bounds)
        assert!(!grid.is_area_free(IVec2::new(7, 7), size, None)); // 7+2 = 9 > 8
        assert!(!grid.is_area_free(IVec2::new(-1, 0), size, None));
    }

    #[test]
    fn test_grid_collision() {
        let mut grid = InventoryGridState {
            width: 8,
            height: 8,
            ..default()
        };

        let item1 = Entity::from_raw(1);
        let size1 = ItemSize { width: 2, height: 2 };
        let pos1 = IVec2::new(2, 2);

        // Place item 1
        for dy in 0..size1.height {
            for dx in 0..size1.width {
                grid.cells.insert(IVec2::new(pos1.x + dx, pos1.y + dy), item1);
            }
        }

        // Try to place item 2 overlapping
        let size2 = ItemSize { width: 2, height: 2 };
        let pos2 = IVec2::new(3, 3); // Overlaps at (3,3)
        assert!(!grid.is_area_free(pos2, size2, None));

        // Try to place item 2 non-overlapping
        let pos3 = IVec2::new(5, 5);
        assert!(grid.is_area_free(pos3, size2, None));

        // Test exclusion (moving item 1 to its own spot shouldn't fail)
        assert!(grid.is_area_free(pos1, size1, Some(item1)));
    }

    #[test]
    fn test_find_free_spot() {
        let mut grid = InventoryGridState {
            width: 4,
            height: 4,
            ..default()
        };

        // Fill first row
        let item1 = Entity::from_raw(1);
        for x in 0..4 {
            grid.cells.insert(IVec2::new(x, 0), item1);
        }

        // Find spot for 2x2
        let size = ItemSize { width: 2, height: 2 };
        let spot = grid.find_free_spot(size);

        // Should be (0, 1) or (1, 1) or (2, 1) -- wait, (0,1) occupies (0,1), (1,1), (0,2), (1,2).
        // (0,1) should be valid.
        assert_eq!(spot, Some(IVec2::new(0, 1)));
    }
}
