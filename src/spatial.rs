/// Spatial utilities for test positioning and layout
///
/// This module provides utilities for:
/// - Calculating grid offsets for positioning tests in a grid-based layout, useful for running multiple tests in parallel without spatial conflicts.
/// - Applying offsets to positions or regions, enabling flexible placement of test structures.
/// - Computing grid dimensions for arranging tests efficiently.
///
///
/// Tests are arranged in a square grid centered at the origin (0, 0).
/// Each cell has a configurable size to ensure tests don't interfere with each other.
///
/// # Arguments
///
/// * `test_index` - Zero-based index of the test
/// * `total_tests` - Total number of tests to arrange
/// * `cell_size` - Size of each grid cell (width and depth in blocks)
///
/// # Returns
///
/// A 3D offset [x, y, z] for positioning the test in world coordinates
///
/// # Examples
///
/// ```
/// use flint_core::spatial::calculate_test_offset;
///
/// // For 4 tests in a 2x2 grid with 16-block cells:
/// let offset1 = calculate_test_offset(0, 4, 16);
/// let offset2 = calculate_test_offset(1, 4, 16);
/// assert_ne!(offset1, offset2);
/// ```
pub fn calculate_test_offset(test_index: usize, total_tests: usize, cell_size: i32) -> [i32; 3] {
    // Calculate grid size (ceil(sqrt(N)))
    let grid_size = (total_tests as f64).sqrt().ceil() as i32;

    // Calculate position in grid
    let grid_x = (test_index as i32) % grid_size;
    let grid_z = (test_index as i32) / grid_size;

    // Calculate base offset
    // For odd grid sizes, center at origin: offset = -(grid_size / 2) * cell_size
    // For even grid sizes, skew to positive: offset = -(grid_size / 2 - 1) * cell_size
    // This way: 1x1 → (0,0), 2x2 → (0,0),(1,0),(0,1),(1,1), 3x3 → (-1,-1)...(1,1)
    let base_offset = if grid_size % 2 == 1 {
        -(grid_size / 2) * cell_size // Odd: truly centered
    } else {
        -(grid_size / 2 - 1) * cell_size // Even: skew to positive
    };

    // Calculate world offset for this test
    [
        base_offset + grid_x * cell_size,
        0, // Y offset is always 0 (tests at same height)
        base_offset + grid_z * cell_size,
    ]
}

/// Calculate grid offset for a test with default cell size
///
/// Uses a default cell size of 16 blocks (15 test area + 1 spacing),
/// which is suitable for most Minecraft test scenarios.
///
/// # Arguments
///
/// * `test_index` - Zero-based index of the test
/// * `total_tests` - Total number of tests to arrange
///
/// # Returns
///
/// A 3D offset [x, y, z] for positioning the test in world coordinates
pub fn calculate_test_offset_default(test_index: usize, total_tests: usize) -> [i32; 3] {
    const DEFAULT_CELL_SIZE: i32 = 16;
    calculate_test_offset(test_index, total_tests, DEFAULT_CELL_SIZE)
}

/// Calculate the grid dimensions needed for a given number of tests
///
/// Returns (width, height) of the grid in cells.
pub fn calculate_grid_dimensions(total_tests: usize) -> (usize, usize) {
    let grid_size = (total_tests as f64).sqrt().ceil() as usize;
    (grid_size, grid_size)
}

/// Calculate all offsets for a collection of tests
///
/// # Arguments
///
/// * `total_tests` - Total number of tests
/// * `cell_size` - Size of each grid cell
///
/// # Returns
///
/// Vector of offsets, one for each test
pub fn calculate_all_offsets(total_tests: usize, cell_size: i32) -> Vec<[i32; 3]> {
    (0..total_tests)
        .map(|i| calculate_test_offset(i, total_tests, cell_size))
        .collect()
}

/// Apply an offset to a position
///
/// # Arguments
///
/// * `pos` - Original position [x, y, z]
/// * `offset` - Offset to apply [dx, dy, dz]
///
/// # Returns
///
/// New position with offset applied
pub fn apply_offset(pos: [i32; 3], offset: [i32; 3]) -> [i32; 3] {
    [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]]
}

/// Apply an offset to a region (pair of positions)
///
/// # Arguments
///
/// * `region` - Original region [[x1, y1, z1], [x2, y2, z2]]
/// * `offset` - Offset to apply [dx, dy, dz]
///
/// # Returns
///
/// New region with offset applied to both corners
pub fn apply_offset_to_region(region: [[i32; 3]; 2], offset: [i32; 3]) -> [[i32; 3]; 2] {
    [
        apply_offset(region[0], offset),
        apply_offset(region[1], offset),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_test_centered_at_origin() {
        let offset = calculate_test_offset(0, 1, 16);
        // Single test should be at origin (chunk 0,0)
        assert_eq!(offset[0], 0);
        assert_eq!(offset[1], 0);
        assert_eq!(offset[2], 0);
    }

    #[test]
    fn test_four_tests_in_grid() {
        let cell_size = 16;

        let offset0 = calculate_test_offset(0, 4, cell_size);
        let offset1 = calculate_test_offset(1, 4, cell_size);
        let offset2 = calculate_test_offset(2, 4, cell_size);
        let offset3 = calculate_test_offset(3, 4, cell_size);

        // For 4 tests (2x2 grid, even), should be in chunks (0,0), (1,0), (0,1), (1,1)
        assert_eq!(offset0, [0, 0, 0]); // chunk (0, 0)
        assert_eq!(offset1, [16, 0, 0]); // chunk (1, 0)
        assert_eq!(offset2, [0, 0, 16]); // chunk (0, 1)
        assert_eq!(offset3, [16, 0, 16]); // chunk (1, 1)

        // All tests should have Y=0
        assert_eq!(offset0[1], 0);
        assert_eq!(offset1[1], 0);
        assert_eq!(offset2[1], 0);
        assert_eq!(offset3[1], 0);

        // Tests should be spaced by cell_size
        assert_eq!(offset1[0] - offset0[0], cell_size);
        assert_eq!(offset2[2] - offset0[2], cell_size);
    }

    #[test]
    fn test_nine_tests_in_grid() {
        let offsets: Vec<_> = (0..9).map(|i| calculate_test_offset(i, 9, 16)).collect();

        // Should create a 3x3 grid (odd size, centered)
        assert_eq!(offsets.len(), 9);

        // All Y coordinates should be 0
        assert!(offsets.iter().all(|o| o[1] == 0));

        // Center test (index 4) should be at origin for odd grid
        let center = offsets[4];
        assert_eq!(center, [0, 0, 0]);

        // Corner tests should be at expected positions
        assert_eq!(offsets[0], [-16, 0, -16]); // chunk (-1, -1)
        assert_eq!(offsets[8], [16, 0, 16]); // chunk (1, 1)
    }

    #[test]
    fn test_grid_dimensions() {
        assert_eq!(calculate_grid_dimensions(1), (1, 1));
        assert_eq!(calculate_grid_dimensions(4), (2, 2));
        assert_eq!(calculate_grid_dimensions(9), (3, 3));
        assert_eq!(calculate_grid_dimensions(10), (4, 4)); // 10 tests need 4x4 grid
        assert_eq!(calculate_grid_dimensions(16), (4, 4));
        assert_eq!(calculate_grid_dimensions(17), (5, 5));
    }

    #[test]
    fn test_calculate_all_offsets() {
        let offsets = calculate_all_offsets(4, 16);
        assert_eq!(offsets.len(), 4);

        // Each offset should be unique
        for i in 0..offsets.len() {
            for j in i + 1..offsets.len() {
                assert_ne!(offsets[i], offsets[j]);
            }
        }
    }

    #[test]
    fn test_apply_offset() {
        let pos = [1, 2, 3];
        let offset = [10, 20, 30];
        let result = apply_offset(pos, offset);
        assert_eq!(result, [11, 22, 33]);
    }

    #[test]
    fn test_apply_offset_negative() {
        let pos = [10, 20, 30];
        let offset = [-5, -10, -15];
        let result = apply_offset(pos, offset);
        assert_eq!(result, [5, 10, 15]);
    }

    #[test]
    fn test_apply_offset_to_region() {
        let region = [[0, 0, 0], [10, 10, 10]];
        let offset = [5, 0, -5];
        let result = apply_offset_to_region(region, offset);
        assert_eq!(result, [[5, 0, -5], [15, 10, 5]]);
    }

    #[test]
    fn test_default_cell_size() {
        let offset1 = calculate_test_offset_default(0, 4);
        let offset2 = calculate_test_offset(0, 4, 16);
        assert_eq!(offset1, offset2);
    }

    #[test]
    fn test_different_cell_sizes() {
        // Test with different cell sizes - use index 1 to get non-zero offset
        let offset_small = calculate_test_offset(1, 4, 8);
        let offset_large = calculate_test_offset(1, 4, 32);

        // Both should be at grid position (1, 0)
        // small: [0 + 1*8, 0, 0] = [8, 0, 0]
        // large: [0 + 1*32, 0, 0] = [32, 0, 0]
        assert_eq!(offset_small, [8, 0, 0]);
        assert_eq!(offset_large, [32, 0, 0]);

        // Larger cell size produces larger spacing
        assert!(offset_large[0] > offset_small[0]);
    }
}
