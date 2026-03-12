#[derive(Clone, Debug)]
pub struct Region {
    pub source_start: usize,
    pub source_end: usize,
}

impl Region {
    pub fn len(&self) -> usize {
        self.source_end - self.source_start
    }
}

#[derive(Clone, Debug)]
pub struct EditList {
    regions: Vec<Region>,
}

impl EditList {
    pub fn from_length(num_frames: usize) -> Self {
        Self {
            regions: vec![Region { source_start: 0, source_end: num_frames }],
        }
    }

    pub fn total_frames(&self) -> usize {
        self.regions.iter().map(|r| r.len()).sum()
    }

    pub fn resolve(&self, edit_frame: usize) -> Option<usize> {
        let mut offset = 0;
        for region in &self.regions {
            let rlen = region.len();
            if edit_frame < offset + rlen {
                return Some(region.source_start + (edit_frame - offset));
            }
            offset += rlen;
        }
        None
    }

    pub fn iter_source_frames(&self, start: usize, len: usize) -> impl Iterator<Item = usize> + '_ {
        let end = start + len;
        let mut offset = 0usize;
        let mut region_idx = 0usize;

        while region_idx < self.regions.len() {
            let rlen = self.regions[region_idx].len();
            if offset + rlen > start {
                break;
            }
            offset += rlen;
            region_idx += 1;
        }

        let mut current_idx = region_idx;
        let mut current_offset = offset;
        let mut edit_pos = start;

        std::iter::from_fn(move || {
            if edit_pos >= end {
                return None;
            }
            loop {
                if current_idx >= self.regions.len() {
                    return None;
                }
                let region = &self.regions[current_idx];
                let rlen = region.len();
                if edit_pos < current_offset + rlen {
                    let source_frame = region.source_start + (edit_pos - current_offset);
                    edit_pos += 1;
                    return Some(source_frame);
                }
                current_offset += rlen;
                current_idx += 1;
            }
        })
    }

    pub fn ripple_delete(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        let mut new_regions: Vec<Region> = Vec::new();
        let mut offset = 0usize;

        for region in &self.regions {
            let rlen = region.len();
            let r_start = offset;
            let r_end = offset + rlen;

            let overlap_start = start.max(r_start);
            let overlap_end = end.min(r_end);

            if overlap_start < overlap_end {
                if r_start < overlap_start {
                    new_regions.push(Region {
                        source_start: region.source_start,
                        source_end: region.source_start + (overlap_start - r_start),
                    });
                }
                if overlap_end < r_end {
                    new_regions.push(Region {
                        source_start: region.source_start + (overlap_end - r_start),
                        source_end: region.source_end,
                    });
                }
            } else {
                new_regions.push(Region {
                    source_start: region.source_start,
                    source_end: region.source_end,
                });
            }

            offset = r_end;
        }

        self.regions = new_regions;
    }

    pub fn crop(&mut self, start: usize, end: usize) {
        if start >= end {
            self.regions.clear();
            return;
        }
        let mut new_regions: Vec<Region> = Vec::new();
        let mut offset = 0usize;

        for region in &self.regions {
            let rlen = region.len();
            let r_start = offset;
            let r_end = offset + rlen;

            let keep_start = start.max(r_start);
            let keep_end = end.min(r_end);

            if keep_start < keep_end {
                new_regions.push(Region {
                    source_start: region.source_start + (keep_start - r_start),
                    source_end: region.source_start + (keep_end - r_start),
                });
            }

            offset = r_end;
        }

        self.regions = new_regions;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_length_creates_single_region() {
        let el = EditList::from_length(100);
        assert_eq!(el.total_frames(), 100);
        assert_eq!(el.regions.len(), 1);
    }

    #[test]
    fn ripple_delete_middle() {
        let mut el = EditList::from_length(100);
        el.ripple_delete(20, 30);
        assert_eq!(el.total_frames(), 90);
        assert_eq!(el.resolve(20), Some(30));
        assert_eq!(el.resolve(19), Some(19));
        assert_eq!(el.resolve(89), Some(99));
        assert_eq!(el.resolve(90), None);
    }

    #[test]
    fn ripple_delete_at_boundaries() {
        let mut el = EditList::from_length(50);
        el.ripple_delete(0, 10);
        assert_eq!(el.total_frames(), 40);
        assert_eq!(el.resolve(0), Some(10));

        let mut el2 = EditList::from_length(50);
        el2.ripple_delete(40, 50);
        assert_eq!(el2.total_frames(), 40);
        assert_eq!(el2.resolve(39), Some(39));
        assert_eq!(el2.resolve(40), None);
    }

    #[test]
    fn crop_keeps_range() {
        let mut el = EditList::from_length(100);
        el.crop(10, 40);
        assert_eq!(el.total_frames(), 30);
        assert_eq!(el.resolve(0), Some(10));
        assert_eq!(el.resolve(29), Some(39));
        assert_eq!(el.resolve(30), None);
    }

    #[test]
    fn iter_source_frames_full() {
        let el = EditList::from_length(5);
        let frames: Vec<usize> = el.iter_source_frames(0, 5).collect();
        assert_eq!(frames, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn iter_source_frames_after_delete() {
        let mut el = EditList::from_length(100);
        el.ripple_delete(20, 30);
        let frames: Vec<usize> = el.iter_source_frames(18, 5).collect();
        assert_eq!(frames, vec![18, 19, 30, 31, 32]);
    }

    #[test]
    fn iter_source_frames_partial() {
        let el = EditList::from_length(10);
        let frames: Vec<usize> = el.iter_source_frames(3, 4).collect();
        assert_eq!(frames, vec![3, 4, 5, 6]);
    }
}
