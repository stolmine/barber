#[derive(Clone, Debug)]
pub enum Region {
    Source { source_start: usize, source_end: usize },
    Silence { len: usize },
}

impl Region {
    pub fn len(&self) -> usize {
        match self {
            Region::Source { source_start, source_end } => source_end - source_start,
            Region::Silence { len } => *len,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EditList {
    regions: Vec<Region>,
}

impl EditList {
    pub fn from_length(num_frames: usize) -> Self {
        Self {
            regions: vec![Region::Source { source_start: 0, source_end: num_frames }],
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
                return match region {
                    Region::Source { source_start, .. } => Some(source_start + (edit_frame - offset)),
                    Region::Silence { .. } => None,
                };
            }
            offset += rlen;
        }
        None
    }

    pub fn iter_source_frames(&self, start: usize, len: usize) -> impl Iterator<Item = Option<usize>> + '_ {
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
                    let result = match region {
                        Region::Source { source_start, .. } => Some(Some(source_start + (edit_pos - current_offset))),
                        Region::Silence { .. } => Some(None),
                    };
                    edit_pos += 1;
                    return result;
                }
                current_offset += rlen;
                current_idx += 1;
            }
        })
    }

    pub fn is_silence_range(&self, start: usize, end: usize) -> bool {
        if start >= end {
            return false;
        }
        let mut offset = 0usize;
        let mut checked = start;
        for region in &self.regions {
            let rlen = region.len();
            let r_start = offset;
            let r_end = offset + rlen;
            if r_end <= start {
                offset = r_end;
                continue;
            }
            if r_start >= end {
                break;
            }
            let overlap_start = start.max(r_start);
            let overlap_end = end.min(r_end);
            if overlap_start < overlap_end {
                if let Region::Source { .. } = region {
                    return false;
                }
                checked = overlap_end;
            }
            offset = r_end;
        }
        checked >= end
    }

    fn transform_regions<F>(&mut self, start: usize, end: usize, mut emit: F)
    where
        F: FnMut(&Region, usize, usize, usize, usize, &mut Vec<Region>),
    {
        let mut new_regions: Vec<Region> = Vec::new();
        let mut offset = 0usize;

        for region in &self.regions {
            let rlen = region.len();
            let r_start = offset;
            let r_end = offset + rlen;
            let overlap_start = start.max(r_start);
            let overlap_end = end.min(r_end);
            emit(region, overlap_start, overlap_end, r_start, r_end, &mut new_regions);
            offset = r_end;
        }

        self.regions = new_regions;
    }

    pub fn ripple_delete(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        self.transform_regions(start, end, |region, overlap_start, overlap_end, r_start, r_end, out| {
            if overlap_start < overlap_end {
                let pre_len = overlap_start - r_start;
                let post_len = r_end - overlap_end;
                if pre_len > 0 {
                    out.push(split_region_prefix(region, pre_len));
                }
                if post_len > 0 {
                    out.push(split_region_suffix(region, overlap_end - r_start, r_end - r_start));
                }
            } else {
                out.push(region.clone());
            }
        });
    }

    pub fn crop(&mut self, start: usize, end: usize) {
        if start >= end {
            self.regions.clear();
            return;
        }
        self.transform_regions(start, end, |region, overlap_start, overlap_end, r_start, _r_end, out| {
            if overlap_start < overlap_end {
                let inner_start = overlap_start - r_start;
                let inner_end = overlap_end - r_start;
                out.push(split_region_suffix(region, inner_start, inner_end));
            }
        });
    }

    pub fn extract_regions(&self, start: usize, end: usize) -> Vec<Region> {
        if start >= end {
            return Vec::new();
        }
        let mut result = Vec::new();
        let mut offset = 0usize;
        for region in &self.regions {
            let rlen = region.len();
            let r_start = offset;
            let r_end = offset + rlen;
            let overlap_start = start.max(r_start);
            let overlap_end = end.min(r_end);
            if overlap_start < overlap_end {
                let inner_start = overlap_start - r_start;
                let inner_end = overlap_end - r_start;
                result.push(split_region_suffix(region, inner_start, inner_end));
            }
            offset = r_end;
        }
        result
    }

    pub fn insert(&mut self, position: usize, regions: &[Region]) {
        if regions.is_empty() {
            return;
        }
        let mut new_regions = Vec::new();
        let mut offset = 0usize;
        let mut inserted = false;
        for region in &self.regions {
            let rlen = region.len();
            let r_start = offset;
            let r_end = offset + rlen;
            if !inserted && position <= r_end && position >= r_start {
                let pre_len = position - r_start;
                let post_len = r_end - position;
                if pre_len > 0 {
                    new_regions.push(split_region_prefix(region, pre_len));
                }
                new_regions.extend_from_slice(regions);
                inserted = true;
                if post_len > 0 {
                    new_regions.push(split_region_suffix(region, pre_len, rlen));
                }
            } else {
                new_regions.push(region.clone());
            }
            offset = r_end;
        }
        if !inserted {
            new_regions.extend_from_slice(regions);
        }
        self.regions = new_regions;
    }

    pub fn gap_delete(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        let gap_len = end - start;
        let mut inserted = false;
        self.transform_regions(start, end, |region, overlap_start, overlap_end, r_start, r_end, out| {
            if overlap_start < overlap_end {
                let pre_len = overlap_start - r_start;
                let post_len = r_end - overlap_end;
                if pre_len > 0 {
                    out.push(split_region_prefix(region, pre_len));
                }
                if !inserted {
                    out.push(Region::Silence { len: gap_len });
                    inserted = true;
                }
                if post_len > 0 {
                    out.push(split_region_suffix(region, overlap_end - r_start, r_end - r_start));
                }
            } else {
                out.push(region.clone());
            }
        });
    }
}

fn split_region_prefix(region: &Region, prefix_len: usize) -> Region {
    match region {
        Region::Source { source_start, .. } => Region::Source {
            source_start: *source_start,
            source_end: source_start + prefix_len,
        },
        Region::Silence { .. } => Region::Silence { len: prefix_len },
    }
}

fn split_region_suffix(region: &Region, inner_start: usize, inner_end: usize) -> Region {
    match region {
        Region::Source { source_start, .. } => Region::Source {
            source_start: source_start + inner_start,
            source_end: source_start + inner_end,
        },
        Region::Silence { .. } => Region::Silence { len: inner_end - inner_start },
    }
}

#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;
