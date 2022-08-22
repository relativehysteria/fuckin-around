//! Non-overlapping sets of inclusive ranges. Useful for physical memory
//! managementment.
// Almost completely copied from Brandon. Bless him.

#![no_std]

use core::cmp;

/// An inclusive range. `RangeInclusive` doesn't implement `Copy`, so it's not
/// used here.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Range {
    pub start: u64,
    pub end:   u64,
}

impl Range {
    /// Returns a new range
    pub fn new(start: u64, end: u64) -> Self {
        // Make sure we were given valid data
        assert!(start <= end, "End can't be lower than start.");

        Self { start, end }
    }

    /// Check whether `other` is completely contained withing this range.
    pub fn contains(&self, other: &Range) -> bool {
        // Make sure we were given a valid range
        assert!(other.start <= other.end, "End can't be lower than start.");

        // Check if `other` is completely contained within this range
        self.start <= other.start && self.end >= other.end
    }

    /// Check whether this range overlaps with another range.
    /// If it does, returns the overlap between the two ranges.
    pub fn overlaps(&self, other: &Range) -> Option<Range> {
        // Make sure we were given a valid range
        assert!(other.start <= other.end, "End can't be lower than start.");

        // Check if there is overlap
        if self.start <= other.end && other.start <= self.end {
            Some(Range {
                start: core::cmp::max(self.start, other.start),
                end:   core::cmp::min(self.end,   other.end)
            })
        } else {
            None
        }
    }
}

/// A set of non-overlapping inclusive `Range`s.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RangeSet {
    /// Array of ranges in the set
    ranges: [Range; 256],

    /// Number of range entries in use.
    ///
    /// This is not `usize` to make the size of the structure constant in all
    /// environments.
    in_use: u32,
}

impl RangeSet {
    /// Returns a new empty `RangeSet`
    pub const fn new() -> Self {
        RangeSet {
            ranges:  [Range { start: 0, end: 0 }; 256],
            in_use: 0,
        }
    }

    /// Returns all the used entries in a `RangeSet`
    pub fn entries(&self) -> &[Range] {
        &self.ranges[..self.in_use as usize]
    }

    /// Delete the range at `idx`
    fn delete(&mut self, idx: usize) {
        // Make sure we don't index out of bounds
        assert!(idx < self.in_use as usize, "Index out of bounds");

        // Put the delete range to the end
        for i in idx..self.in_use as usize - 1 {
            self.ranges.swap(i, i+1);
        }

        // Decrement the number of valid ranges
        self.in_use -= 1;
    }

    /// Insert a new range into the `RangeSet`.
    ///
    /// If the range overlaps with an existing range, both ranges will be merged
    /// into one.
    pub fn insert(&mut self, mut range: Range) {
        // Make sure we were given a valid range
        assert!(range.start <= range.end, "End can't be lower than start.");

        // Loop forever and merge overlapping ranges.
        // Once we run out of overlaps, break.
        'merges: loop {
            // Go through each entry in our ranges
            for idx in 0..self.in_use as usize {
                let entry = self.ranges[idx];

                // Check whether the ranges are overlapping or touching.
                // If the ranges don't overlap even after we increment their
                // ends, they are not touching, so we can simply continue with
                // another range.
                let first  =
                    Range::new(entry.start, entry.end.saturating_add(1));
                let second =
                    Range::new(range.start, range.end.saturating_add(1));
                if first.overlaps(&second).is_none() {
                    continue;
                }

                // One of our ranges and the `range` we were given were
                // overlapping. Merge them into one range.
                range.start = cmp::min(entry.start, range.start);
                range.end   = cmp::max(entry.end,   range.end);

                // Delete the old overlapping range
                self.delete(idx);

                // Since we have mutated the ranges, we have to start again and
                // look for new overlaps.
                continue 'merges;
            }

            break;
        }

        // Make sure that our ranges don't overflow
        assert!((self.in_use as usize) < self.ranges.len(),
            "Too many entries in RangeSet.");

        // Append the new range to our ranges
        self.ranges[self.in_use as usize] = range;
        self.in_use += 1;
    }

    /// Remove a `range` from this `RangeSet`.
    ///
    /// Any range overlapping with `range` will be trimmed. Any range that is
    /// completely contained within `range` will be entirely removed.
    pub fn remove(&mut self, range: Range) {
        // Make sure we were given a valid range
        assert!(range.start <= range.end, "End can't be lower than start.");

        'subtractions: loop {
            // Go through each entry in our ranges
            for idx in 0..self.in_use as usize {
                let entry = self.ranges[idx];

                // If there is no overlap, there is nothing to do with this
                // range
                if entry.overlaps(&range).is_none() {
                    continue;
                }

                // If the entry is completely contained in the range, delete it
                if range.contains(&entry) {
                    self.delete(idx);
                    continue 'subtractions;
                }

                // At this point there is a partial overlap.

                if range.start <= entry.start {
                    // If the overlap is at the start of our entry,
                    // adjust the start
                    self.ranges[idx].start = range.end.saturating_add(1);
                } else if range.end >= entry.end {
                    // If the overlap is at the end of our entry,
                    // adjust the end
                    self.ranges[idx].end = range.start.saturating_sub(1);
                } else {
                    // If the range is fully contained in our entry, we have to
                    // split our entry in two

                    // Second half of the range
                    self.ranges[idx].start = range.end.saturating_add(1);

                    assert!((self.in_use as usize) < self.ranges.len(),
                        "Too many entries in RangeSet on split");

                    // First half of the range
                    self.ranges[self.in_use as usize] = Range {
                        start: entry.start,
                        end:   range.start.saturating_sub(1),
                    };

                    self.in_use += 1;
                    continue 'subtractions;
                }
            }

            break;
        }
    }

    /// Allocate `size` bytes of memory with `align` requirements.
    /// The allocation is preferably done in `regions`. If an allocation cannot
    /// be satisfied from `regions`, it will come from whatever is best.
    /// If `regions` is `None`, the allocation will be satisfied from anywhere.
    ///
    /// Returns the pointer to the allocated memory.
    pub fn allocate(&mut self, size: u64, align: u64,
                    regions: Option<&RangeSet>) -> Option<usize> {
        // Don't allow 0-sized allocations
        if size == 0 {
            return None;
        }

        // Check that we have an alignment with a power of 2
        if align.count_ones() != 1 {
            return None;
        }

        // Generate a mask for the alignment
        let align_mask = align - 1;

        // Go through each range and see if an allocation can fit into it
        let mut allocation = None;
        'search: for entry in self.entries() {
            // Calculate the padding
            let padding = (align - (entry.start & align_mask)) & align_mask;

            // Compute the inclusive start and end of the allocation
            let start = entry.start;
            let end   = start.checked_add(size - 1)?.checked_add(padding)?;

            // Make sure that the allocation is addressable
            if start > core::usize::MAX as u64 || end > core::usize::MAX as u64{
                continue;
            }

            // Make sure this entry is large enough for the allocation
            if end > entry.end {
                continue;
            }

            // If we have a region allocation preference
            if let Some(regions) = regions {
                // Go through each preferenced region and check if our entry
                // is overlapping with this region
                for &region in regions.entries() {
                    if let Some(overlap) = (*entry).overlaps(&region) {
                        // Compute the rounded-up alignment from the
                        // overlapping region
                        let align_overlap =
                            (overlap.start.wrapping_add(align_mask)) &
                            !align_mask;

                        if align_overlap >= overlap.start &&
                                align_overlap <= overlap.end &&
                                (overlap.end - align_overlap) >= (size - 1) {
                            // Alignment did not cause an overflow AND
                            // Alignment did not cause exceeding the end AND
                            // Amount of aligned overlap can satisfy the
                            // allocation

                            // Compute the inclusive end of this proposed
                            // allocation
                            let overlap_alc_end = align_overlap + (size - 1);

                            // Make sure the allocation fits in the current
                            // addressable address space
                            if align_overlap > core::usize::MAX as u64 ||
                                    overlap_alc_end > core::usize::MAX as u64 {
                                continue 'search;
                            }

                            // We know the allocation can be satisfied starting
                            // at `align_overlap`
                            allocation = Some((align_overlap,
                                               overlap_alc_end,
                                               align_overlap as usize));
                            break 'search;
                        }
                    }
                }
            }

            // Compute the "best" allocation size to date
            let prev_size = allocation.map(|(start, end, _)| end - start);

            if allocation.is_none() || prev_size.unwrap() > end - start {
                // Update the allocation to the new best size
                allocation = Some((start, end, (start + padding) as usize));
            }
        }

        allocation.map(|(start, end, ptr)| {
            // Remove this range from the available set
            self.remove(Range { start: start, end: end });

            // Return out the pointer!
            ptr
        })
    }
}
